use crate::config;
use sokobanitron_app::shared::PointerPhase;
use std::env;
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
const LIPC_GET_PROP: &str = "/usr/bin/lipc-get-prop";
const LIPC_SET_PROP: &str = "/usr/bin/lipc-set-prop";
const LIPC_SEND_EVENT: &str = "/usr/bin/lipc-send-event";
const DIRTY_FRAMEBUFFER_WRITE_ENV: &str = "SOKOBANITRON_KINDLE_DIRTY_FB_WRITE";
const PRESENT_METRICS_ENV: &str = "SOKOBANITRON_KINDLE_PRESENT_METRICS";
const POWERD_SERVICE: &str = "com.lab126.powerd";
const POWERD_DEBUG_SERVICE: &str = "com.lab126.powerd.debug";
const BLANKET_SERVICE: &str = "com.lab126.blanket";
const BLANKET_SCREENSAVER_MODULE: &str = "screensaver";
const POWERD_STATE_PROPERTY: &str = "state";
const POWERD_EVENT_MAG_SENSOR_CLOSED: &str = "dbg_mag_sensor_closed";
const POWERD_EVENT_MAG_SENSOR_OPENED: &str = "dbg_mag_sensor_opened";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub left: usize,
    pub top: usize,
    pub width: usize,
    pub height: usize,
}

pub struct Display {
    fb: File,
    previous_gray: Vec<u8>,
    has_previous_gray: bool,
    framebuffer: Vec<u8>,
    update_marker: u32,
    update_abi: Option<UpdateAbi>,
    logged_ioctl_failure: bool,
    dirty_framebuffer_writes: bool,
    log_present_metrics: bool,
}

struct PresentMetric<'a> {
    path: &'a str,
    mode: &'a str,
    region: Option<Region>,
    measured_scan_elapsed: Duration,
    write: Option<(Duration, usize)>,
    refresh_elapsed: Option<Duration>,
    present_start: Instant,
}

impl Display {
    pub fn new() -> Result<Self> {
        let fb = OpenOptions::new()
            .write(true)
            .open(config::FRAMEBUFFER_DEVICE)?;

        let _ = configure_update_mode(&fb);
        let update_abi = probe_update_abi(&fb);
        let blank_frame = vec![0xFFu8; config::STRIDE * config::HEIGHT];
        let blank_gray = vec![0xFFu8; config::WIDTH * config::HEIGHT];

        Ok(Self {
            fb,
            previous_gray: blank_gray,
            has_previous_gray: false,
            framebuffer: blank_frame,
            update_marker: 1,
            update_abi,
            logged_ioctl_failure: false,
            dirty_framebuffer_writes: dirty_framebuffer_writes_enabled(),
            log_present_metrics: present_metrics_enabled(),
        })
    }

    pub fn present_gray(&mut self, gray: &[u8]) -> Result<()> {
        self.present_gray_inner(gray, None)
    }

    pub fn present_gray_fast_partial(&mut self, gray: &[u8]) -> Result<()> {
        self.present_gray_inner(gray, Some(WAVEFORM_MODE_DU))
    }

    pub fn present_gray_region(&mut self, gray: &[u8], region: Region) -> Result<()> {
        self.present_gray_region_inner(gray, region, None)
    }

    pub fn present_gray_region_fast_partial(&mut self, gray: &[u8], region: Region) -> Result<()> {
        self.present_gray_region_inner(gray, region, Some(WAVEFORM_MODE_DU))
    }

    pub fn force_full_refresh_next(&mut self) {
        self.has_previous_gray = false;
    }

    fn present_gray_inner(&mut self, gray: &[u8], partial_waveform: Option<u32>) -> Result<()> {
        let present_start = Instant::now();
        let scan_start = Instant::now();
        let dirty = if self.has_previous_gray {
            update_grayscale_framebuffer_from_gray_diff(
                gray,
                self.previous_gray.as_slice(),
                &mut self.framebuffer,
            )
        } else {
            copy_gray_framebuffer_full(gray, &mut self.framebuffer);
            None
        };
        let scan_elapsed = scan_start.elapsed();

        if self.has_previous_gray && dirty.is_none() {
            self.log_present_metric(PresentMetric {
                path: "gray_diff",
                mode: "noop",
                region: None,
                measured_scan_elapsed: scan_elapsed,
                write: None,
                refresh_elapsed: None,
                present_start,
            });
            return Ok(());
        }

        match dirty {
            None => {
                let write_start = Instant::now();
                write_full_framebuffer(&self.fb, &self.framebuffer)?;
                let write_elapsed = write_start.elapsed();
                let refresh_start = Instant::now();
                self.request_full_refresh()?;
                let refresh_elapsed = refresh_start.elapsed();
                self.previous_gray.copy_from_slice(gray);
                self.log_present_metric(PresentMetric {
                    path: "gray_diff",
                    mode: "full",
                    region: None,
                    measured_scan_elapsed: scan_elapsed,
                    write: Some((write_elapsed, self.framebuffer.len())),
                    refresh_elapsed: Some(refresh_elapsed),
                    present_start,
                });
            }
            Some(region) => {
                let write_start = Instant::now();
                let bytes_written;
                if self.dirty_framebuffer_writes {
                    bytes_written = framebuffer_region_byte_len(region);
                    write_framebuffer_region(&self.fb, &self.framebuffer, region)?;
                } else {
                    // Baseline path: dirty-rect is used for EPDC update region, not for
                    // framebuffer write scope.
                    bytes_written = self.framebuffer.len();
                    write_full_framebuffer(&self.fb, &self.framebuffer)?;
                }
                let write_elapsed = write_start.elapsed();
                let refresh_start = Instant::now();
                self.request_partial_refresh(region, partial_waveform)?;
                let refresh_elapsed = refresh_start.elapsed();
                copy_gray_region(gray, &mut self.previous_gray, region);
                self.log_present_metric(PresentMetric {
                    path: "gray_diff",
                    mode: "partial",
                    region: Some(region),
                    measured_scan_elapsed: scan_elapsed,
                    write: Some((write_elapsed, bytes_written)),
                    refresh_elapsed: Some(refresh_elapsed),
                    present_start,
                });
            }
        }

        self.has_previous_gray = true;
        Ok(())
    }

    fn present_gray_region_inner(
        &mut self,
        gray: &[u8],
        region: Region,
        partial_waveform: Option<u32>,
    ) -> Result<()> {
        if !self.has_previous_gray {
            return self.present_gray_inner(gray, partial_waveform);
        }

        let present_start = Instant::now();
        let prepare_start = Instant::now();
        let bytes_written = if self.dirty_framebuffer_writes {
            copy_gray_framebuffer_region(gray, &mut self.framebuffer, region);
            framebuffer_region_byte_len(region)
        } else {
            copy_gray_framebuffer_full(gray, &mut self.framebuffer);
            self.framebuffer.len()
        };
        let prepare_elapsed = prepare_start.elapsed();

        let write_start = Instant::now();
        if self.dirty_framebuffer_writes {
            write_framebuffer_region(&self.fb, &self.framebuffer, region)?;
        } else {
            write_full_framebuffer(&self.fb, &self.framebuffer)?;
        }
        let write_elapsed = write_start.elapsed();

        let refresh_start = Instant::now();
        self.request_partial_refresh(region, partial_waveform)?;
        let refresh_elapsed = refresh_start.elapsed();

        copy_gray_region(gray, &mut self.previous_gray, aligned_region(region));
        self.log_present_metric(PresentMetric {
            path: "declared_dirty",
            mode: "partial",
            region: Some(region),
            measured_scan_elapsed: prepare_elapsed,
            write: Some((write_elapsed, bytes_written)),
            refresh_elapsed: Some(refresh_elapsed),
            present_start,
        });

        self.has_previous_gray = true;
        Ok(())
    }

    fn log_present_metric(&self, metric: PresentMetric<'_>) {
        if !self.log_present_metrics {
            return;
        }
        let region = metric
            .region
            .map(|region| {
                let aligned = align_region_for_epdc(region);
                format!(
                    "{}x{}+{}+{} aligned={}x{}+{}+{}",
                    region.width,
                    region.height,
                    region.left,
                    region.top,
                    aligned.width,
                    aligned.height,
                    aligned.left,
                    aligned.top
                )
            })
            .unwrap_or_else(|| "full".to_string());
        let (write_us, bytes_written) = metric
            .write
            .map(|(elapsed, bytes)| (elapsed.as_micros(), bytes))
            .unwrap_or((0, 0));
        let refresh_us = metric
            .refresh_elapsed
            .map(|elapsed| elapsed.as_micros())
            .unwrap_or(0);
        eprintln!(
            "present path={} mode={} region={} dirty_fb_write={} scan_us={} write_us={} refresh_request_us={} total_us={} bytes_written={}",
            metric.path,
            metric.mode,
            region,
            self.dirty_framebuffer_writes,
            metric.measured_scan_elapsed.as_micros(),
            write_us,
            refresh_us,
            metric.present_start.elapsed().as_micros(),
            bytes_written,
        );
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
    pending_touch_phase: Option<PointerPhase>,
    position_dirty: bool,
    x_dirty: bool,
    y_dirty: bool,
    power_down_at: Option<Instant>,
    power_long_emitted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppInputEvent {
    Pointer {
        phase: PointerPhase,
        raw_x: i32,
        raw_y: i32,
    },
    PowerShortPress,
    PowerLongPress,
    /// Synthetic wakeup used so the outer loop can poll `powerd` and perform housekeeping even
    /// when no touch or power input arrives.
    IdleTick,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerdScreensaverState {
    Active,
    ScreenSaver,
    Other,
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
            pending_touch_phase: None,
            position_dirty: false,
            x_dirty: false,
            y_dirty: false,
            power_down_at: None,
            power_long_emitted: false,
        })
    }

    /// Returns real touch/power events, or `IdleTick` when the caller requested a timeout so the
    /// app loop can resynchronize Kindle sleep state.
    pub fn next_input_event(&mut self, timeout_ms: Option<i32>) -> Result<AppInputEvent> {
        loop {
            if !self.wait_for_input(timeout_ms)? {
                return Ok(AppInputEvent::IdleTick);
            }

            if let Some(event) = self.read_touch_event()? {
                return Ok(event);
            }

            if let Some(power_event) = self.read_power_event()? {
                return Ok(power_event);
            }
        }
    }

    fn wait_for_input(&self, timeout_ms: Option<i32>) -> Result<bool> {
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
        let rc = unsafe { poll(fds.as_mut_ptr(), nfds, timeout_ms.unwrap_or(-1)) };
        if rc < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(rc > 0)
    }

    fn read_touch_event(&mut self) -> Result<Option<AppInputEvent>> {
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
            return Ok(None);
        }

        self.touch.read_exact(&mut self.packet)?;
        let event = parse_input_event(&self.packet);
        self.apply_touch_packet(event);
        if event.type_ == EV_SYN && event.code == SYN_REPORT {
            return Ok(self.finish_touch_report());
        }
        Ok(None)
    }

    fn apply_touch_packet(&mut self, event: InputEvent) {
        match (event.type_, event.code) {
            (EV_ABS, ABS_X) | (EV_ABS, ABS_MT_POSITION_X) => {
                self.latest_x = Some(event.value);
                self.position_dirty = true;
                self.x_dirty = true;
            }
            (EV_ABS, ABS_Y) | (EV_ABS, ABS_MT_POSITION_Y) => {
                self.latest_y = Some(event.value);
                self.position_dirty = true;
                self.y_dirty = true;
            }
            (EV_KEY, BTN_TOUCH) => {
                if event.value != 0 {
                    self.touching = true;
                    self.pending_touch_phase = Some(PointerPhase::Started);
                    if !self.x_dirty {
                        self.latest_x = None;
                    }
                    if !self.y_dirty {
                        self.latest_y = None;
                    }
                } else {
                    self.touching = false;
                    self.pending_touch_phase = Some(PointerPhase::Ended);
                }
            }
            _ => {}
        }
    }

    fn finish_touch_report(&mut self) -> Option<AppInputEvent> {
        let phase = match self.pending_touch_phase {
            Some(phase) => Some(phase),
            None if self.touching && self.position_dirty => Some(PointerPhase::Moved),
            None => None,
        };
        let event = match phase {
            Some(PointerPhase::Started) if !(self.x_dirty && self.y_dirty) => {
                eprintln!(
                    "warning: dropping touch start report: pending_phase={:?} touching={} latest_x={:?} latest_y={:?} x_dirty={} y_dirty={}",
                    self.pending_touch_phase,
                    self.touching,
                    self.latest_x,
                    self.latest_y,
                    self.x_dirty,
                    self.y_dirty,
                );
                None
            }
            Some(phase) => match (self.latest_x, self.latest_y) {
                (Some(raw_x), Some(raw_y)) => Some(AppInputEvent::Pointer {
                    phase,
                    raw_x,
                    raw_y,
                }),
                _ => None,
            },
            None => None,
        };
        self.clear_touch_report_state();
        event
    }

    fn clear_touch_report_state(&mut self) {
        self.pending_touch_phase = None;
        self.position_dirty = false;
        self.x_dirty = false;
        self.y_dirty = false;
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
                    if let Some(started) = self.power_down_at
                        && !self.power_long_emitted
                        && started.elapsed() >= long_press
                    {
                        self.power_long_emitted = true;
                        return Ok(Some(AppInputEvent::PowerLongPress));
                    }
                }
                _ => {}
            }
        }
        Ok(None)
    }
}

pub fn start_lab126_gui() -> io::Result<()> {
    // When we hand control back to the stock Kindle UI, reload Blanket's screensaver module so
    // lab126_gui can use the normal system screensaver path again.
    let _ = set_blanket_module("load", BLANKET_SCREENSAVER_MODULE);
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

pub fn read_powerd_state() -> io::Result<PowerdScreensaverState> {
    let output = Command::new(LIPC_GET_PROP)
        .arg(POWERD_SERVICE)
        .arg(POWERD_STATE_PROPERTY)
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "lipc-get-prop {} {} failed with status {}",
            POWERD_SERVICE, POWERD_STATE_PROPERTY, output.status
        )));
    }

    let state = String::from_utf8_lossy(&output.stdout);
    Ok(match state.trim() {
        "active" => PowerdScreensaverState::Active,
        "screenSaver" => PowerdScreensaverState::ScreenSaver,
        other => {
            eprintln!("warning: unexpected powerd state: {other}");
            PowerdScreensaverState::Other
        }
    })
}

pub fn enter_powerd_screensaver() -> io::Result<()> {
    // Our app draws the sleep image itself before handing off to powerd, so Blanket's own
    // screensaver module must be unloaded to avoid stacking the stock sleep overlay on top.
    set_blanket_module("unload", BLANKET_SCREENSAVER_MODULE)?;
    send_powerd_debug_event(POWERD_EVENT_MAG_SENSOR_CLOSED)
}

pub fn enter_system_screensaver() -> io::Result<()> {
    send_powerd_debug_event(POWERD_EVENT_MAG_SENSOR_CLOSED)
}

pub fn exit_powerd_screensaver() -> io::Result<()> {
    send_powerd_debug_event(POWERD_EVENT_MAG_SENSOR_OPENED)
}

fn send_powerd_debug_event(event: &str) -> io::Result<()> {
    let status = Command::new(LIPC_SEND_EVENT)
        .arg(POWERD_DEBUG_SERVICE)
        .arg(event)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "lipc-send-event {} {} failed with status {}",
            POWERD_DEBUG_SERVICE, event, status
        )))
    }
}

fn set_blanket_module(action: &str, module: &str) -> io::Result<()> {
    let status = Command::new(LIPC_SET_PROP)
        .arg(BLANKET_SERVICE)
        .arg(action)
        .arg(module)
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "lipc-set-prop {} {} {} failed with status {}",
            BLANKET_SERVICE, action, module, status
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

fn copy_gray_framebuffer_full(gray: &[u8], framebuffer: &mut [u8]) {
    for y in 0..config::HEIGHT {
        let src_row = y * config::WIDTH;
        let dst_row = y * config::STRIDE;
        framebuffer[dst_row..dst_row + config::WIDTH]
            .copy_from_slice(&gray[src_row..src_row + config::WIDTH]);
        framebuffer[dst_row + config::WIDTH..dst_row + config::STRIDE].fill(0xFF);
    }
}

fn copy_gray_framebuffer_region(gray: &[u8], framebuffer: &mut [u8], region: Region) {
    let region = aligned_region(region);
    let left = region.left.min(config::WIDTH);
    let top = region.top.min(config::HEIGHT);
    let right = (region.left + region.width).min(config::WIDTH);
    let bottom = (region.top + region.height).min(config::HEIGHT);

    for y in top..bottom {
        let src_row_start = y * config::WIDTH + left;
        let src_row_end = y * config::WIDTH + right;
        let dst_row_start = y * config::STRIDE + left;
        let dst_row_end = y * config::STRIDE + right;
        framebuffer[dst_row_start..dst_row_end].copy_from_slice(&gray[src_row_start..src_row_end]);
    }
}

fn update_grayscale_framebuffer_from_gray_diff(
    gray: &[u8],
    previous_gray: &[u8],
    framebuffer: &mut [u8],
) -> Option<Region> {
    let mut min_x = config::WIDTH;
    let mut min_y = config::HEIGHT;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut found = false;

    for y in 0..config::HEIGHT {
        let src_row = y * config::WIDTH;
        let dst_row = y * config::STRIDE;
        for x in 0..config::WIDTH {
            let i = src_row + x;
            let previous = previous_gray[i];
            let current = gray[i];
            if current != previous {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                framebuffer[dst_row + x] = current;
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

fn copy_gray_region(gray: &[u8], previous_gray: &mut [u8], region: Region) {
    let left = region.left.min(config::WIDTH);
    let top = region.top.min(config::HEIGHT);
    let right = (region.left + region.width).min(config::WIDTH);
    let bottom = (region.top + region.height).min(config::HEIGHT);

    for y in top..bottom {
        let row_start = y * config::WIDTH + left;
        let row_end = y * config::WIDTH + right;
        previous_gray[row_start..row_end].copy_from_slice(&gray[row_start..row_end]);
    }
}

fn write_full_framebuffer(fb: &File, frame: &[u8]) -> Result<()> {
    fb.write_all_at(frame, 0)?;
    Ok(())
}

fn write_framebuffer_region(fb: &File, frame: &[u8], region: Region) -> Result<()> {
    let aligned = align_region_for_epdc(region);
    let row_bytes = aligned.width;
    for y in aligned.top..aligned.top + aligned.height {
        let row_start = y * config::STRIDE + aligned.left;
        let row_end = row_start + row_bytes;
        fb.write_all_at(&frame[row_start..row_end], row_start as u64)?;
    }
    Ok(())
}

fn framebuffer_region_byte_len(region: Region) -> usize {
    let aligned = align_region_for_epdc(region);
    aligned.width * aligned.height
}

fn aligned_region(region: Region) -> Region {
    let aligned = align_region_for_epdc(region);
    Region {
        left: aligned.left,
        top: aligned.top,
        width: aligned.width,
        height: aligned.height,
    }
}

fn dirty_framebuffer_writes_enabled() -> bool {
    env_flag_enabled(DIRTY_FRAMEBUFFER_WRITE_ENV)
}

fn present_metrics_enabled() -> bool {
    env_flag_enabled(PRESENT_METRICS_ENV)
}

fn env_flag_enabled(name: &str) -> bool {
    matches!(
        env::var(name)
            .as_deref()
            .map(str::to_ascii_lowercase),
        Ok(value) if matches!(value.as_str(), "1" | "true" | "yes" | "on")
    )
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

    let right_aligned = (right.div_ceil(X_ALIGN) * X_ALIGN).min(config::WIDTH);
    let bottom_aligned = (bottom.div_ceil(Y_ALIGN) * Y_ALIGN).min(config::HEIGHT);

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

#[cfg(test)]
mod tests {
    use super::{AppInputEvent, BTN_TOUCH, EV_KEY, InputEvent, TouchReader};
    use sokobanitron_app::shared::PointerPhase;
    use std::fs::File;

    fn test_touch_reader() -> TouchReader {
        TouchReader {
            touch: File::open("/dev/null").expect("open /dev/null for touch"),
            power: None,
            packet: [0u8; std::mem::size_of::<super::InputEvent>()],
            power_packet: [0u8; std::mem::size_of::<super::InputEvent>()],
            latest_x: None,
            latest_y: None,
            touching: false,
            pending_touch_phase: None,
            position_dirty: false,
            x_dirty: false,
            y_dirty: false,
            power_down_at: None,
            power_long_emitted: false,
        }
    }

    #[test]
    fn missing_coordinates_do_not_leak_started_into_next_report() {
        let mut state = test_touch_reader();

        state.touching = true;
        state.pending_touch_phase = Some(PointerPhase::Started);
        assert_eq!(state.finish_touch_report(), None);

        state.latest_x = Some(120);
        state.latest_y = Some(240);
        state.position_dirty = true;
        state.x_dirty = true;
        state.y_dirty = true;
        assert_eq!(
            state.finish_touch_report(),
            Some(AppInputEvent::Pointer {
                phase: PointerPhase::Moved,
                raw_x: 120,
                raw_y: 240,
            })
        );
    }

    #[test]
    fn new_touch_start_cannot_reuse_previous_touch_coordinates() {
        let mut state = test_touch_reader();

        state.latest_x = Some(12);
        state.latest_y = Some(34);
        state.position_dirty = true;
        state.x_dirty = true;
        state.y_dirty = true;
        state.touching = true;
        state.pending_touch_phase = Some(PointerPhase::Started);
        assert_eq!(
            state.finish_touch_report(),
            Some(AppInputEvent::Pointer {
                phase: PointerPhase::Started,
                raw_x: 12,
                raw_y: 34,
            })
        );

        state.touching = false;
        state.pending_touch_phase = Some(PointerPhase::Ended);
        assert_eq!(
            state.finish_touch_report(),
            Some(AppInputEvent::Pointer {
                phase: PointerPhase::Ended,
                raw_x: 12,
                raw_y: 34,
            })
        );

        state.apply_touch_packet(InputEvent {
            type_: EV_KEY,
            code: BTN_TOUCH,
            value: 1,
            ..InputEvent::default()
        });

        assert_eq!(state.latest_x, None);
        assert_eq!(state.latest_y, None);
        assert_eq!(state.finish_touch_report(), None);
    }

    #[test]
    fn fresh_coordinates_and_touch_down_emit_started() {
        let mut state = test_touch_reader();

        state.latest_x = Some(120);
        state.latest_y = Some(240);
        state.position_dirty = true;
        state.x_dirty = true;
        state.y_dirty = true;
        state.touching = true;
        state.pending_touch_phase = Some(PointerPhase::Started);

        assert_eq!(
            state.finish_touch_report(),
            Some(AppInputEvent::Pointer {
                phase: PointerPhase::Started,
                raw_x: 120,
                raw_y: 240,
            })
        );
    }
}
