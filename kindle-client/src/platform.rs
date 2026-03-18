use crate::config;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Result, Write};
use std::os::fd::AsRawFd;

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

const SYN_REPORT: u16 = 0;
const BTN_TOUCH: u16 = 0x14a;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;

const EVIOCGRAB: u64 = 0x40044590;

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InputEvent {
    tv_sec: i32,
    tv_usec: i32,
    type_: u16,
    code: u16,
    value: i32,
}

pub struct TouchReader {
    touch: File,
    packet: [u8; std::mem::size_of::<InputEvent>()],
    latest_x: Option<i32>,
    latest_y: Option<i32>,
    touching: bool,
    pending_tap: bool,
}

impl TouchReader {
    pub fn new() -> Result<Self> {
        let touch = OpenOptions::new().read(true).open(config::TOUCH_DEVICE)?;
        if let Err(err) = grab_input(&touch) {
            eprintln!("warning: failed to grab input device: {err}");
        }

        Ok(Self {
            touch,
            packet: [0u8; std::mem::size_of::<InputEvent>()],
            latest_x: None,
            latest_y: None,
            touching: false,
            pending_tap: false,
        })
    }

    pub fn next_tap_raw(&mut self) -> Result<(i32, i32)> {
        loop {
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
                (EV_SYN, SYN_REPORT) => {
                    if self.touching && self.pending_tap {
                        if let (Some(raw_x), Some(raw_y)) = (self.latest_x, self.latest_y) {
                            self.pending_tap = false;
                            return Ok((raw_x, raw_y));
                        }
                    } else if !self.touching {
                        self.pending_tap = false;
                    }
                }
                _ => {}
            }
        }
    }
}

pub fn map_touch_to_screen(raw_x: i32, raw_y: i32) -> io::Result<(usize, usize)> {
    let x = map_touch(raw_x, config::TOUCH_MIN_X, config::TOUCH_MAX_X, config::WIDTH)?;
    let y = map_touch(raw_y, config::TOUCH_MIN_Y, config::TOUCH_MAX_Y, config::HEIGHT)?;
    Ok((x, y))
}

pub fn write_rgba_frame(rgba: &[u8]) -> Result<()> {
    let grayscale = rgba_to_grayscale_framebuffer(rgba);
    let mut fb = OpenOptions::new().write(true).open(config::FRAMEBUFFER_DEVICE)?;
    fb.write_all(&grayscale)?;

    let mut epdc = OpenOptions::new().write(true).open(config::REFRESH_DEVICE)?;
    epdc.write_all(b"0")?;
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
}

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
