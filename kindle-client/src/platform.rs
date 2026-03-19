use crate::config;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Result, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::FileExt;
use std::process::Command;
use std::time::{Duration, Instant};

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

const SYN_REPORT: u16 = 0;
const BTN_TOUCH: u16 = 0x14a;
const KEY_POWER: u16 = 116;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;

const EVIOCGRAB: u64 = 0x40044590;
const MXCFB_SET_AUTO_UPDATE_MODE: u64 = iow(b'F', 0x2D, std::mem::size_of::<u32>());
const MXCFB_SET_UPDATE_SCHEME: u64 = iow(b'F', 0x32, std::mem::size_of::<u32>());

const AUTO_UPDATE_MODE_REGION_MODE: u32 = 0;
const UPDATE_SCHEME_SNAPSHOT: u32 = 0;

const WAVEFORM_MODE_DU: u32 = 1;
const WAVEFORM_MODE_AUTO: u32 = 257;
const UPDATE_MODE_PARTIAL: u32 = 0;
const UPDATE_MODE_FULL: u32 = 1;
const TEMP_USE_AMBIENT: i32 = 0x1000;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InputEvent {
    tv_sec: i32,
    tv_usec: i32,
    type_: u16,
    code: u16,
    value: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MxcfbRect {
    top: u32,
    left: u32,
    width: u32,
    height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MxcfbAltBufferNoVirt {
    phys_addr: u32,
    width: u32,
    height: u32,
    alt_update_region: MxcfbRect,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MxcfbAltBufferWithVirt {
    virt_addr: u32,
    phys_addr: u32,
    width: u32,
    height: u32,
    alt_update_region: MxcfbRect,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MxcfbUpdateUseAlt {
    update_region: MxcfbRect,
    waveform_mode: u32,
    update_mode: u32,
    update_marker: u32,
    temp: i32,
    use_alt_buffer: i32,
    alt_buffer_data: MxcfbAltBufferNoVirt,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MxcfbUpdateFlagsNoVirt {
    update_region: MxcfbRect,
    waveform_mode: u32,
    update_mode: u32,
    update_marker: u32,
    temp: i32,
    flags: u32,
    alt_buffer_data: MxcfbAltBufferNoVirt,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MxcfbUpdateFlagsWithVirt {
    update_region: MxcfbRect,
    waveform_mode: u32,
    update_mode: u32,
    update_marker: u32,
    temp: i32,
    flags: u32,
    alt_buffer_data: MxcfbAltBufferWithVirt,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct MxcfbUpdateFlagsWithVirtDither {
    update_region: MxcfbRect,
    waveform_mode: u32,
    update_mode: u32,
    update_marker: u32,
    temp: i32,
    flags: u32,
    dither_mode: i32,
    quant_bit: i32,
    alt_buffer_data: MxcfbAltBufferWithVirt,
}

#[derive(Clone, Copy)]
enum UpdateAbi {
    UseAltNoVirt,
    FlagsNoVirt,
    FlagsWithVirt,
    FlagsWithVirtDither,
}

impl UpdateAbi {
    fn name(self) -> &'static str {
        match self {
            Self::UseAltNoVirt => "use_alt_no_virt",
            Self::FlagsNoVirt => "flags_no_virt",
            Self::FlagsWithVirt => "flags_with_virt",
            Self::FlagsWithVirtDither => "flags_with_virt_dither",
        }
    }

    fn request(self) -> u64 {
        match self {
            Self::UseAltNoVirt => iow(b'F', 0x2E, std::mem::size_of::<MxcfbUpdateUseAlt>()),
            Self::FlagsNoVirt => iow(b'F', 0x2E, std::mem::size_of::<MxcfbUpdateFlagsNoVirt>()),
            Self::FlagsWithVirt => iow(b'F', 0x2E, std::mem::size_of::<MxcfbUpdateFlagsWithVirt>()),
            Self::FlagsWithVirtDither => iow(
                b'F',
                0x2E,
                std::mem::size_of::<MxcfbUpdateFlagsWithVirtDither>(),
            ),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Region {
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
}

pub struct Display {
    fb: File,
    previous_frame: Option<Vec<u8>>,
    update_marker: u32,
    update_abi: Option<UpdateAbi>,
    logged_ioctl_failure: bool,
}

impl Display {
    pub fn new() -> Result<Self> {
        let fb = OpenOptions::new()
            .write(true)
            .open(config::FRAMEBUFFER_DEVICE)?;

        let _ = configure_update_mode(&fb);
        let update_abi = probe_update_abi(&fb);

        Ok(Self {
            fb,
            previous_frame: None,
            update_marker: 1,
            update_abi,
            logged_ioctl_failure: false,
        })
    }

    pub fn present_rgba(&mut self, rgba: &[u8]) -> Result<()> {
        self.present_rgba_inner(rgba, None)
    }

    pub fn present_rgba_fast_partial(&mut self, rgba: &[u8]) -> Result<()> {
        self.present_rgba_inner(rgba, Some(WAVEFORM_MODE_DU))
    }

    pub fn force_full_refresh_next(&mut self) {
        self.previous_frame = None;
    }

    fn present_rgba_inner(&mut self, rgba: &[u8], partial_waveform: Option<u32>) -> Result<()> {
        let next = rgba_to_grayscale_framebuffer(rgba);
        let previous = self.previous_frame.as_ref();
        let dirty = previous.and_then(|prev| compute_dirty_rect(prev, &next));

        // Keep kernel framebuffer memory fully synchronized with current app frame.
        // Dirty-rect is used for EPDC update region, not for framebuffer write scope.
        write_full_framebuffer(&self.fb, &next)?;

        match dirty {
            None if previous.is_some() => {}
            None => {
                self.request_full_refresh()?;
            }
            Some(region) => {
                self.request_partial_refresh(region, partial_waveform)?;
            }
        }

        self.previous_frame = Some(next);
        Ok(())
    }

    fn request_partial_refresh(
        &mut self,
        region: Region,
        waveform_mode: Option<u32>,
    ) -> Result<()> {
        let waveform = waveform_mode.unwrap_or(WAVEFORM_MODE_DU);
        if let Some(abi) = self.update_abi {
            match send_update_ioctl(
                &self.fb,
                abi,
                region,
                waveform,
                UPDATE_MODE_PARTIAL,
                self.update_marker,
            ) {
                Ok(()) => {
                    self.update_marker = self.update_marker.wrapping_add(1).max(1);
                    return Ok(());
                }
                Err(err) => {
                    self.log_update_error_once("partial", abi, err);
                    self.update_abi = None;
                }
            }
        }

        self.sysfs_region_refresh(region, UPDATE_MODE_PARTIAL, waveform_mode)
    }

    fn request_full_refresh(&mut self) -> Result<()> {
        if let Some(abi) = self.update_abi {
            let full = Region {
                left: 0,
                top: 0,
                width: config::WIDTH,
                height: config::HEIGHT,
            };
            match send_update_ioctl(
                &self.fb,
                abi,
                full,
                WAVEFORM_MODE_AUTO,
                UPDATE_MODE_FULL,
                self.update_marker,
            ) {
                Ok(()) => {
                    self.update_marker = self.update_marker.wrapping_add(1).max(1);
                    return Ok(());
                }
                Err(err) => {
                    self.log_update_error_once("full", abi, err);
                    self.update_abi = None;
                }
            }
        }

        // Use legacy full refresh command that is known-good on this Kindle.
        write_sysfs_refresh("0\n")
    }

    fn log_update_error_once(&mut self, mode: &str, abi: UpdateAbi, err: io::Error) {
        if self.logged_ioctl_failure {
            return;
        }
        self.logged_ioctl_failure = true;
        eprintln!(
            "warning: MXCFB_SEND_UPDATE {} failed for ABI {}: {} (request=0x{:x})",
            mode,
            abi.name(),
            err,
            abi.request(),
        );
    }

    fn sysfs_region_refresh(
        &mut self,
        region: Region,
        update_mode: u32,
        waveform_mode: Option<u32>,
    ) -> Result<()> {
        let aligned = align_region_for_epdc(region);
        // Observed Kindle sysfs format:
        //   <waveform> <update_mode> <top> <left> <width> <height>
        // Use waveform=0 (AUTO) by default for stability; allow explicit override.
        let waveform = waveform_mode.unwrap_or(0);
        let cmd = format!(
            "{} {} {} {} {} {}\n",
            waveform, update_mode, aligned.top, aligned.left, aligned.width, aligned.height
        );
        if write_sysfs_refresh(&cmd).is_ok() {
            return Ok(());
        }
        if waveform_mode.is_some() {
            // Fallback to AUTO waveform if explicit mode is unsupported.
            let fallback = format!(
                "{} {} {} {} {} {}\n",
                0, update_mode, aligned.top, aligned.left, aligned.width, aligned.height
            );
            return write_sysfs_refresh(&fallback);
        }
        write_sysfs_refresh(&cmd)
    }
}

pub struct TouchReader {
    touch: File,
    power: Option<File>,
    packet: [u8; std::mem::size_of::<InputEvent>()],
    power_packet: [u8; std::mem::size_of::<InputEvent>()],
    latest_x: Option<i32>,
    latest_y: Option<i32>,
    touching: bool,
    pending_tap: bool,
    power_down_at: Option<Instant>,
    power_long_emitted: bool,
}

pub enum AppInputEvent {
    Tap(i32, i32),
    PowerShortPress,
    PowerLongPress,
}

impl TouchReader {
    pub fn new() -> Result<Self> {
        let touch = OpenOptions::new().read(true).open(config::TOUCH_DEVICE)?;
        let power = OpenOptions::new()
            .read(true)
            .open(config::POWER_DEVICE)
            .ok();
        if let Err(err) = grab_input(&touch) {
            eprintln!("warning: failed to grab input device: {err}");
        }

        Ok(Self {
            touch,
            power,
            packet: [0u8; std::mem::size_of::<InputEvent>()],
            power_packet: [0u8; std::mem::size_of::<InputEvent>()],
            latest_x: None,
            latest_y: None,
            touching: false,
            pending_tap: false,
            power_down_at: None,
            power_long_emitted: false,
        })
    }

    pub fn next_input_event(&mut self) -> Result<AppInputEvent> {
        loop {
            self.wait_for_input()?;

            if self.read_touch_event()? {
                if self.touching && self.pending_tap {
                    if let (Some(raw_x), Some(raw_y)) = (self.latest_x, self.latest_y) {
                        self.pending_tap = false;
                        return Ok(AppInputEvent::Tap(raw_x, raw_y));
                    }
                } else if !self.touching {
                    self.pending_tap = false;
                }
            }

            if let Some(power_event) = self.read_power_event()? {
                return Ok(power_event);
            }
        }
    }

    fn wait_for_input(&self) -> Result<()> {
        let mut fds = [PollFd {
            fd: self.touch.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }; 2];
        let mut nfds = 1usize;
        if let Some(power) = &self.power {
            fds[1] = PollFd {
                fd: power.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            };
            nfds = 2;
        }

        // SAFETY: pointers and nfds are valid for the stack-allocated array.
        let rc = unsafe { poll(fds.as_mut_ptr(), nfds, -1) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    fn read_touch_event(&mut self) -> Result<bool> {
        let mut fds = [PollFd {
            fd: self.touch.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }];
        // SAFETY: pointers and nfds are valid for the stack-allocated array.
        let rc = unsafe { poll(fds.as_mut_ptr(), 1, 0) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        if rc == 0 || (fds[0].revents & POLLIN) == 0 {
            return Ok(false);
        }

        self.touch.read_exact(&mut self.packet)?;
        let event = parse_input_event(&self.packet);
        match (event.type_, event.code) {
            (EV_ABS, ABS_X) | (EV_ABS, ABS_MT_POSITION_X) => self.latest_x = Some(event.value),
            (EV_ABS, ABS_Y) | (EV_ABS, ABS_MT_POSITION_Y) => self.latest_y = Some(event.value),
            (EV_KEY, BTN_TOUCH) => {
                if event.value != 0 {
                    self.touching = true;
                    self.pending_tap = true;
                } else {
                    self.touching = false;
                    self.pending_tap = false;
                }
            }
            _ => {}
        }
        Ok(event.type_ == EV_SYN && event.code == SYN_REPORT)
    }

    fn read_power_event(&mut self) -> Result<Option<AppInputEvent>> {
        let Some(power) = self.power.as_mut() else {
            return Ok(None);
        };

        let mut fds = [PollFd {
            fd: power.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }];
        // SAFETY: pointers and nfds are valid for the stack-allocated array.
        let rc = unsafe { poll(fds.as_mut_ptr(), 1, 0) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        if rc == 0 || (fds[0].revents & POLLIN) == 0 {
            return Ok(None);
        }

        power.read_exact(&mut self.power_packet)?;
        let event = parse_input_event(&self.power_packet);
        if event.type_ == EV_KEY && event.code == KEY_POWER {
            let long_press = Duration::from_millis(config::POWER_LONG_PRESS_MS);
            match event.value {
                1 => {
                    self.power_down_at = Some(Instant::now());
                    self.power_long_emitted = false;
                }
                0 => {
                    if let Some(started) = self.power_down_at.take() {
                        if !self.power_long_emitted && started.elapsed() >= long_press {
                            self.power_long_emitted = true;
                            return Ok(Some(AppInputEvent::PowerLongPress));
                        }
                        if !self.power_long_emitted {
                            return Ok(Some(AppInputEvent::PowerShortPress));
                        }
                    }
                    self.power_long_emitted = false;
                }
                2 => {
                    if let Some(started) = self.power_down_at {
                        if !self.power_long_emitted && started.elapsed() >= long_press {
                            self.power_long_emitted = true;
                            return Ok(Some(AppInputEvent::PowerLongPress));
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(None)
    }
}

pub fn start_lab126_gui() -> io::Result<()> {
    let status = Command::new("/sbin/initctl")
        .arg("start")
        .arg("lab126_gui")
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "initctl start lab126_gui failed with status {status}"
        )))
    }
}

pub fn map_touch_to_screen(raw_x: i32, raw_y: i32) -> io::Result<(usize, usize)> {
    let x = map_touch(
        raw_x,
        config::TOUCH_MIN_X,
        config::TOUCH_MAX_X,
        config::WIDTH,
    )?;
    let y = map_touch(
        raw_y,
        config::TOUCH_MIN_Y,
        config::TOUCH_MAX_Y,
        config::HEIGHT,
    )?;
    Ok((x, y))
}

fn probe_update_abi(fb: &File) -> Option<UpdateAbi> {
    let test_region = Region {
        left: 0,
        top: 0,
        width: config::WIDTH,
        height: config::HEIGHT,
    };
    let candidates = [
        UpdateAbi::UseAltNoVirt,
        UpdateAbi::FlagsNoVirt,
        UpdateAbi::FlagsWithVirt,
        UpdateAbi::FlagsWithVirtDither,
    ];

    candidates.into_iter().find(|abi| {
        send_update_ioctl(
            fb,
            *abi,
            test_region,
            WAVEFORM_MODE_AUTO,
            UPDATE_MODE_FULL,
            1,
        )
        .is_ok()
    })
}

fn send_update_ioctl(
    fb: &File,
    abi: UpdateAbi,
    region: Region,
    waveform_mode: u32,
    update_mode: u32,
    update_marker: u32,
) -> io::Result<()> {
    let aligned = align_region_for_epdc(region);
    let rect = MxcfbRect {
        top: aligned.top as u32,
        left: aligned.left as u32,
        width: aligned.width as u32,
        height: aligned.height as u32,
    };

    let rc = match abi {
        UpdateAbi::UseAltNoVirt => {
            let update = MxcfbUpdateUseAlt {
                update_region: rect,
                waveform_mode,
                update_mode,
                update_marker,
                temp: TEMP_USE_AMBIENT,
                use_alt_buffer: 0,
                alt_buffer_data: MxcfbAltBufferNoVirt::default(),
            };
            // SAFETY: fd and pointer are valid, request is ABI-specific ioctl number.
            unsafe { ioctl(fb.as_raw_fd(), abi.request(), &update) }
        }
        UpdateAbi::FlagsNoVirt => {
            let update = MxcfbUpdateFlagsNoVirt {
                update_region: rect,
                waveform_mode,
                update_mode,
                update_marker,
                temp: TEMP_USE_AMBIENT,
                flags: 0,
                alt_buffer_data: MxcfbAltBufferNoVirt::default(),
            };
            // SAFETY: fd and pointer are valid, request is ABI-specific ioctl number.
            unsafe { ioctl(fb.as_raw_fd(), abi.request(), &update) }
        }
        UpdateAbi::FlagsWithVirt => {
            let update = MxcfbUpdateFlagsWithVirt {
                update_region: rect,
                waveform_mode,
                update_mode,
                update_marker,
                temp: TEMP_USE_AMBIENT,
                flags: 0,
                alt_buffer_data: MxcfbAltBufferWithVirt::default(),
            };
            // SAFETY: fd and pointer are valid, request is ABI-specific ioctl number.
            unsafe { ioctl(fb.as_raw_fd(), abi.request(), &update) }
        }
        UpdateAbi::FlagsWithVirtDither => {
            let update = MxcfbUpdateFlagsWithVirtDither {
                update_region: rect,
                waveform_mode,
                update_mode,
                update_marker,
                temp: TEMP_USE_AMBIENT,
                flags: 0,
                dither_mode: 0,
                quant_bit: 0,
                alt_buffer_data: MxcfbAltBufferWithVirt::default(),
            };
            // SAFETY: fd and pointer are valid, request is ABI-specific ioctl number.
            unsafe { ioctl(fb.as_raw_fd(), abi.request(), &update) }
        }
    };

    if rc == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn rgba_to_grayscale_framebuffer(rgba: &[u8]) -> Vec<u8> {
    let mut framebuffer = vec![0xFFu8; config::STRIDE * config::HEIGHT];
    for y in 0..config::HEIGHT {
        let src_row = y * config::WIDTH * 4;
        let dst_row = y * config::STRIDE;
        for x in 0..config::WIDTH {
            let i = src_row + x * 4;
            let r = rgba[i] as u16;
            let g = rgba[i + 1] as u16;
            let b = rgba[i + 2] as u16;
            let gray = ((77 * r + 150 * g + 29 * b) >> 8) as u8;
            framebuffer[dst_row + x] = gray;
        }
    }
    framebuffer
}

fn write_full_framebuffer(fb: &File, frame: &[u8]) -> Result<()> {
    fb.write_all_at(frame, 0)?;
    Ok(())
}

fn compute_dirty_rect(previous: &[u8], next: &[u8]) -> Option<Region> {
    let mut min_x = config::WIDTH;
    let mut min_y = config::HEIGHT;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut found = false;

    for y in 0..config::HEIGHT {
        let row = y * config::STRIDE;
        for x in 0..config::WIDTH {
            if previous[row + x] != next[row + x] {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !found {
        return None;
    }

    Some(Region {
        left: min_x,
        top: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    })
}

#[derive(Clone, Copy)]
struct AlignedRegion {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
}

fn align_region_for_epdc(region: Region) -> AlignedRegion {
    const X_ALIGN: usize = 8;
    const Y_ALIGN: usize = 1;

    let left = (region.left / X_ALIGN) * X_ALIGN;
    let top = (region.top / Y_ALIGN) * Y_ALIGN;

    let right = (region.left + region.width).min(config::WIDTH);
    let bottom = (region.top + region.height).min(config::HEIGHT);

    let right_aligned = ((right + (X_ALIGN - 1)) / X_ALIGN * X_ALIGN).min(config::WIDTH);
    let bottom_aligned = ((bottom + (Y_ALIGN - 1)) / Y_ALIGN * Y_ALIGN).min(config::HEIGHT);

    AlignedRegion {
        left,
        top,
        width: right_aligned.saturating_sub(left).max(1),
        height: bottom_aligned.saturating_sub(top).max(1),
    }
}

fn parse_input_event(packet: &[u8; std::mem::size_of::<InputEvent>()]) -> InputEvent {
    InputEvent {
        tv_sec: i32::from_ne_bytes(packet[0..4].try_into().expect("tv_sec bytes")),
        tv_usec: i32::from_ne_bytes(packet[4..8].try_into().expect("tv_usec bytes")),
        type_: u16::from_ne_bytes(packet[8..10].try_into().expect("type bytes")),
        code: u16::from_ne_bytes(packet[10..12].try_into().expect("code bytes")),
        value: i32::from_ne_bytes(packet[12..16].try_into().expect("value bytes")),
    }
}

fn grab_input(touch: &File) -> Result<()> {
    // SAFETY: ioctl is called with a valid file descriptor and EVIOCGRAB request.
    let rc = unsafe { ioctl(touch.as_raw_fd(), EVIOCGRAB, 1) };
    if rc == -1 {
        let err = io::Error::last_os_error();
        match err.raw_os_error() {
            Some(22) | Some(25) => return Ok(()),
            _ => return Err(err),
        }
    }
    Ok(())
}

unsafe extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
    fn poll(fds: *mut PollFd, nfds: usize, timeout: i32) -> i32;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct PollFd {
    fd: i32,
    events: i16,
    revents: i16,
}

const POLLIN: i16 = 0x0001;

fn map_touch(raw: i32, min: i32, max: i32, dimension: usize) -> io::Result<usize> {
    if max <= min {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "touch calibration max must be greater than min",
        ));
    }

    let clamped = raw.clamp(min, max);
    let span = (max - min) as i64;
    let offset = (clamped - min) as i64;
    let scaled = offset * (dimension.saturating_sub(1) as i64) / span;
    Ok(scaled as usize)
}

const fn iow(ty: u8, nr: u8, size: usize) -> u64 {
    (1u64 << 30) | ((size as u64) << 16) | ((ty as u64) << 8) | (nr as u64)
}

fn configure_update_mode(fb: &File) -> io::Result<()> {
    let auto_mode = AUTO_UPDATE_MODE_REGION_MODE;
    // SAFETY: fb fd is valid, request and pointer type match kernel ioctl expectations.
    let rc = unsafe { ioctl(fb.as_raw_fd(), MXCFB_SET_AUTO_UPDATE_MODE, &auto_mode) };
    if rc == -1 {
        return Err(io::Error::last_os_error());
    }

    let scheme = UPDATE_SCHEME_SNAPSHOT;
    // SAFETY: fb fd is valid, request and pointer type match kernel ioctl expectations.
    let rc = unsafe { ioctl(fb.as_raw_fd(), MXCFB_SET_UPDATE_SCHEME, &scheme) };
    if rc == -1 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

fn write_sysfs_refresh(command: &str) -> io::Result<()> {
    let mut f = OpenOptions::new()
        .write(true)
        .open(config::REFRESH_DEVICE)?;
    f.write_all(command.as_bytes())
}
