use renderer::{BoardViewport, Renderer};
use sokobanitron_gameplay::GameplaySession;
use std::fs::OpenOptions;
use std::io::{self, Read, Result, Write};
use std::os::fd::AsRawFd;

const WIDTH: usize = 1072;
const HEIGHT: usize = 1448;
const STRIDE: usize = 1088;

const TOUCH_DEVICE: &str = "/dev/input/event1";
const FRAMEBUFFER_DEVICE: &str = "/dev/fb0";
const REFRESH_DEVICE: &str = "/sys/devices/platform/imx_epdc_fb/mxc_epdc_update";

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

const TOUCH_MIN_X: i32 = 0;
const TOUCH_MAX_X: i32 = WIDTH as i32 - 1;
const TOUCH_MIN_Y: i32 = 0;
const TOUCH_MAX_Y: i32 = HEIGHT as i32 - 1;

const FOOTER_HEIGHT: usize = 180;
const RESTART_BUTTON_WIDTH: usize = 420;
const RESTART_BUTTON_HEIGHT: usize = 96;
const UI_TEXT_SCALE: usize = 8;
const UI_TEXT_SPACING: usize = 10;
const PORTRAIT_LEVEL_VISUAL: &str = "\
_@_#\n\
_#_#\n\
___#\n\
#_.#\n\
#_.#\n\
#_.#\n\
__##\n\
_$__\n\
_$$_\n\
____";

fn main() -> Result<()> {
    let mut app = KindleApp::new();
    app.render()?;
    app.run_touch_loop()
}

struct KindleApp {
    renderer: Renderer,
    session: GameplaySession,
    viewport: BoardViewport,
}

impl KindleApp {
    fn new() -> Self {
        let session = GameplaySession::from_level_ascii(visual_to_level(PORTRAIT_LEVEL_VISUAL));
        let board_area_height = (HEIGHT.saturating_sub(FOOTER_HEIGHT)) as u32;
        let viewport = BoardViewport::fit_to_window(WIDTH as u32, board_area_height, session.board());
        Self {
            renderer: Renderer::new(),
            session,
            viewport,
        }
    }

    fn update_viewport(&mut self) {
        let board_area_height = (HEIGHT.saturating_sub(FOOTER_HEIGHT)) as u32;
        self.viewport =
            BoardViewport::fit_to_window(WIDTH as u32, board_area_height, self.session.board());
    }

    fn render(&mut self) -> Result<()> {
        let mut rgba = vec![0u8; WIDTH * HEIGHT * 4];
        self.renderer.draw(
            &mut rgba,
            WIDTH as u32,
            HEIGHT as u32,
            self.session.board(),
            &self.viewport,
        );
        draw_restart_ui(&mut rgba);
        let grayscale = rgba_to_grayscale_framebuffer(&rgba);
        write_framebuffer(&grayscale)?;
        trigger_refresh()
    }

    fn on_tap(&mut self, raw_x: i32, raw_y: i32) -> Result<()> {
        let screen_x = map_touch(raw_x, TOUCH_MIN_X, TOUCH_MAX_X, WIDTH)?;
        let screen_y = map_touch(raw_y, TOUCH_MIN_Y, TOUCH_MAX_Y, HEIGHT)?;
        let restart = restart_button_rect();

        if restart.contains(screen_x, screen_y) {
            self.session.restart();
            self.update_viewport();
            self.render()?;
            return Ok(());
        }

        if let Some((x, y)) = self.viewport.screen_to_cell(
            screen_x as f64,
            screen_y as f64,
            self.session.board(),
        ) {
            self.session.click_cell(x, y);
            self.render()?;
        }
        Ok(())
    }

    fn run_touch_loop(&mut self) -> Result<()> {
        let mut touch = OpenOptions::new().read(true).open(TOUCH_DEVICE)?;
        if let Err(err) = grab_input(&touch) {
            eprintln!("warning: failed to grab input device: {err}");
        }

        let mut packet = [0u8; std::mem::size_of::<InputEvent>()];
        let mut latest_x: Option<i32> = None;
        let mut latest_y: Option<i32> = None;
        let mut touching = false;
        let mut pending_tap = false;

        loop {
            touch.read_exact(&mut packet)?;
            let event = parse_input_event(&packet);

            match (event.type_, event.code) {
                (EV_ABS, ABS_X) | (EV_ABS, ABS_MT_POSITION_X) => latest_x = Some(event.value),
                (EV_ABS, ABS_Y) | (EV_ABS, ABS_MT_POSITION_Y) => latest_y = Some(event.value),
                (EV_KEY, BTN_TOUCH) => {
                    if event.value != 0 {
                        touching = true;
                        pending_tap = true;
                    } else {
                        touching = false;
                        pending_tap = false;
                    }
                }
                (EV_SYN, SYN_REPORT) => {
                    if touching && pending_tap {
                        if let (Some(raw_x), Some(raw_y)) = (latest_x, latest_y) {
                            self.on_tap(raw_x, raw_y)?;
                            pending_tap = false;
                        }
                    } else if !touching {
                        pending_tap = false;
                    }
                }
                _ => {}
            }
        }
    }
}

fn visual_to_level(level: &str) -> String {
    level
        .chars()
        .map(|ch| if ch == '_' { ' ' } else { ch })
        .collect()
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct InputEvent {
    tv_sec: i32,
    tv_usec: i32,
    type_: u16,
    code: u16,
    value: i32,
}

fn rgba_to_grayscale_framebuffer(rgba: &[u8]) -> Vec<u8> {
    let mut framebuffer = vec![0xFFu8; STRIDE * HEIGHT];
    for y in 0..HEIGHT {
        let src_row = y * WIDTH * 4;
        let dst_row = y * STRIDE;
        for x in 0..WIDTH {
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

fn grab_input(touch: &std::fs::File) -> Result<()> {
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

fn write_framebuffer(frame: &[u8]) -> Result<()> {
    let mut fb = OpenOptions::new().write(true).open(FRAMEBUFFER_DEVICE)?;
    fb.write_all(frame)?;
    Ok(())
}

fn trigger_refresh() -> Result<()> {
    let mut epdc = OpenOptions::new().write(true).open(REFRESH_DEVICE)?;
    epdc.write_all(b"0")?;
    Ok(())
}

#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl Rect {
    fn contains(self, px: usize, py: usize) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

fn restart_button_rect() -> Rect {
    Rect {
        x: (WIDTH - RESTART_BUTTON_WIDTH) / 2,
        y: HEIGHT - FOOTER_HEIGHT + (FOOTER_HEIGHT - RESTART_BUTTON_HEIGHT) / 2,
        w: RESTART_BUTTON_WIDTH,
        h: RESTART_BUTTON_HEIGHT,
    }
}

fn draw_restart_ui(frame: &mut [u8]) {
    let footer_top = HEIGHT - FOOTER_HEIGHT;
    draw_rect_rgba(frame, 0, footer_top, WIDTH, FOOTER_HEIGHT, [232, 232, 232, 255]);

    let button = restart_button_rect();
    draw_rect_rgba(frame, button.x, button.y, button.w, button.h, [255, 255, 255, 255]);
    draw_rect_outline_rgba(frame, button, 5, [0, 0, 0, 255]);
    draw_centered_label(frame, button, "RESTART", UI_TEXT_SCALE, UI_TEXT_SPACING, [0, 0, 0, 255]);
}

fn draw_centered_label(
    frame: &mut [u8],
    rect: Rect,
    text: &str,
    scale: usize,
    spacing: usize,
    color: [u8; 4],
) {
    let text_width = measure_text(text, scale, spacing);
    let text_height = 7 * scale;
    let x = rect.x + rect.w.saturating_sub(text_width) / 2;
    let y = rect.y + rect.h.saturating_sub(text_height) / 2;
    draw_text(frame, x, y, text, scale, spacing, color);
}

fn measure_text(text: &str, scale: usize, spacing: usize) -> usize {
    let mut width = 0usize;
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        width += glyph_width(ch) * scale;
        if chars.peek().is_some() {
            width += spacing;
        }
    }
    width
}

fn draw_text(
    frame: &mut [u8],
    x: usize,
    y: usize,
    text: &str,
    scale: usize,
    spacing: usize,
    color: [u8; 4],
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        draw_glyph(frame, cursor_x, y, ch, scale, color);
        cursor_x += glyph_width(ch) * scale + spacing;
    }
}

fn draw_glyph(frame: &mut [u8], x: usize, y: usize, ch: char, scale: usize, color: [u8; 4]) {
    let glyph = glyph_pattern(ch);
    for (row_idx, row_bits) in glyph.iter().enumerate() {
        for col_idx in 0..5 {
            if (row_bits >> (4 - col_idx)) & 1 == 1 {
                draw_rect_rgba(
                    frame,
                    x + col_idx * scale,
                    y + row_idx * scale,
                    scale,
                    scale,
                    color,
                );
            }
        }
    }
}

fn glyph_width(ch: char) -> usize {
    match ch {
        ' ' => 3,
        _ => 5,
    }
}

fn glyph_pattern(ch: char) -> [u8; 7] {
    match ch {
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        _ => [0b11111, 0b10001, 0b00110, 0b00100, 0b00110, 0b10001, 0b11111],
    }
}

fn draw_rect_outline_rgba(frame: &mut [u8], rect: Rect, thickness: usize, color: [u8; 4]) {
    draw_rect_rgba(frame, rect.x, rect.y, rect.w, thickness, color);
    draw_rect_rgba(
        frame,
        rect.x,
        rect.y + rect.h.saturating_sub(thickness),
        rect.w,
        thickness,
        color,
    );
    draw_rect_rgba(frame, rect.x, rect.y, thickness, rect.h, color);
    draw_rect_rgba(
        frame,
        rect.x + rect.w.saturating_sub(thickness),
        rect.y,
        thickness,
        rect.h,
        color,
    );
}

fn draw_rect_rgba(frame: &mut [u8], x: usize, y: usize, w: usize, h: usize, color: [u8; 4]) {
    let x_end = (x + w).min(WIDTH);
    let y_end = (y + h).min(HEIGHT);

    for yy in y..y_end {
        let row = yy * WIDTH * 4;
        for xx in x..x_end {
            let idx = row + xx * 4;
            frame[idx] = color[0];
            frame[idx + 1] = color[1];
            frame[idx + 2] = color[2];
            frame[idx + 3] = color[3];
        }
    }
}
