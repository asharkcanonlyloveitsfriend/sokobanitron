use crate::config;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Result, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::FileExt;
use std::time::{Duration, Instant};

const MXCFB_SET_AUTO_UPDATE_MODE: u64 = iow(b'F', 0x2D, std::mem::size_of::<u32>());
const MXCFB_SET_UPDATE_SCHEME: u64 = iow(b'F', 0x32, std::mem::size_of::<u32>());

const AUTO_UPDATE_MODE_REGION_MODE: u32 = 0;
const UPDATE_SCHEME_SNAPSHOT: u32 = 0;

const WAVEFORM_MODE_DU: u32 = 1;
const WAVEFORM_MODE_AUTO: u32 = 257;
const UPDATE_MODE_PARTIAL: u32 = 0;
const UPDATE_MODE_FULL: u32 = 1;
const TEMP_USE_AMBIENT: i32 = 0x1000;
const DIRTY_FRAMEBUFFER_WRITE_ENV: &str = "SOKOBANITRON_KINDLE_DIRTY_FB_WRITE";
const PRESENT_METRICS_ENV: &str = "SOKOBANITRON_KINDLE_PRESENT_METRICS";

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

const fn iow(ty: u8, nr: u8, size: usize) -> u64 {
    (1u64 << 30) | ((size as u64) << 16) | ((ty as u64) << 8) | (nr as u64)
}

unsafe extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
}
