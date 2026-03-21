use crate::constants::{BUTTON_TEXT_COLOR, MODE_ICON_SCALE, MODE_ICON_SIZE, UI_TEXT_SCALE};
use renderer::{UI_BUTTON_MARGIN, UI_BUTTON_SIZE, UI_MENU_BUTTON_HEIGHT};

#[derive(Clone, Copy)]
pub struct ScreenRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl ScreenRect {
    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x as f64
            && py >= self.y as f64
            && px < self.x.saturating_add(self.w) as f64
            && py < self.y.saturating_add(self.h) as f64
    }
}

pub enum ZoomButtonAction {
    ZoomOut,
    ZoomIn,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModeIcon {
    Draw,
    Manipulate,
}

type IconBits = [u16; MODE_ICON_SIZE];
const MODE_ICON_DRAW_PENCIL: IconBits = [
    0b110000000,
    0b111000000,
    0b011100000,
    0b001110000,
    0b000111000,
    0b000011100,
    0b000001111,
    0b000000101,
    0b000000111,
];
const MODE_ICON_MANIPULATE_CURSOR: IconBits = [
    0b001100000,
    0b000110001,
    0b000011011,
    0b000111111,
    0b000011111,
    0b000001111,
    0b000000111,
    0b000000011,
    0b000000001,
];

pub fn zoom_button_action_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
    can_zoom_out: bool,
    can_zoom_in: bool,
) -> Option<ZoomButtonAction> {
    if can_zoom_out {
        let minus = zoom_out_button_rect(height);
        if minus.contains(px, py) {
            return Some(ZoomButtonAction::ZoomOut);
        }
    }
    if can_zoom_in {
        let plus = zoom_in_button_rect(width, height);
        if plus.contains(px, py) {
            return Some(ZoomButtonAction::ZoomIn);
        }
    }
    None
}

pub fn mode_toggle_button_rect() -> ScreenRect {
    ScreenRect {
        x: UI_BUTTON_MARGIN,
        y: UI_BUTTON_MARGIN,
        w: UI_BUTTON_SIZE,
        h: UI_BUTTON_SIZE,
    }
}

pub fn top_menu_button_rect(width: u32) -> ScreenRect {
    ScreenRect {
        x: width.saturating_sub(UI_BUTTON_SIZE) / 2,
        y: UI_BUTTON_MARGIN,
        w: UI_BUTTON_SIZE,
        h: UI_MENU_BUTTON_HEIGHT,
    }
}

pub fn top_menu_button_hit_rect(width: u32) -> ScreenRect {
    let base = top_menu_button_rect(width);
    let extra_h = base.h / 4;
    let hit_h = base.h + extra_h;
    let hit_y = base.y.saturating_sub(extra_h / 2);
    ScreenRect {
        x: base.x,
        y: hit_y,
        w: base.w,
        h: hit_h,
    }
}

pub fn top_menu_button_contains(px: f64, py: f64, width: u32) -> bool {
    top_menu_button_hit_rect(width).contains(px, py)
}

pub fn draw_top_menu_toggle(frame: &mut [u8], width: u32, height: u32, open: bool) {
    let rect = top_menu_button_rect(width);
    let glyph = if open { "/\\" } else { "\\/" };
    draw_centered_label(
        frame,
        width,
        height,
        rect,
        glyph,
        UI_TEXT_SCALE,
        BUTTON_TEXT_COLOR,
    );
}

pub fn draw_mode_icon_in_rect(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    icon: ModeIcon,
) {
    draw_mode_toggle_icon(frame, width, height, rect, icon);
}

pub fn draw_controls(
    frame: &mut [u8],
    width: u32,
    height: u32,
    can_zoom_out: bool,
    can_zoom_in: bool,
    draw_mode_active: bool,
) {
    let mode_rect = mode_toggle_button_rect();
    let mode_icon = if draw_mode_active {
        ModeIcon::Draw
    } else {
        ModeIcon::Manipulate
    };
    draw_mode_toggle_icon(frame, width, height, mode_rect, mode_icon);

    if can_zoom_out {
        let minus = zoom_out_button_rect(height);
        draw_centered_label(
            frame,
            width,
            height,
            minus,
            "-",
            UI_TEXT_SCALE,
            BUTTON_TEXT_COLOR,
        );
    }
    if can_zoom_in {
        let plus = zoom_in_button_rect(width, height);
        draw_centered_label(
            frame,
            width,
            height,
            plus,
            "+",
            UI_TEXT_SCALE,
            BUTTON_TEXT_COLOR,
        );
    }
}

fn zoom_out_button_rect(height: u32) -> ScreenRect {
    ScreenRect {
        x: UI_BUTTON_MARGIN,
        y: height.saturating_sub(UI_BUTTON_MARGIN + UI_BUTTON_SIZE),
        w: UI_BUTTON_SIZE,
        h: UI_BUTTON_SIZE,
    }
}

fn zoom_in_button_rect(width: u32, height: u32) -> ScreenRect {
    ScreenRect {
        x: width.saturating_sub(UI_BUTTON_MARGIN + UI_BUTTON_SIZE),
        y: height.saturating_sub(UI_BUTTON_MARGIN + UI_BUTTON_SIZE),
        w: UI_BUTTON_SIZE,
        h: UI_BUTTON_SIZE,
    }
}

fn draw_mode_toggle_icon(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    mode_icon: ModeIcon,
) {
    let icon = match mode_icon {
        ModeIcon::Draw => MODE_ICON_DRAW_PENCIL,
        ModeIcon::Manipulate => MODE_ICON_MANIPULATE_CURSOR,
    };

    let scale = MODE_ICON_SCALE;
    let icon_w = MODE_ICON_SIZE * scale;
    let icon_h = MODE_ICON_SIZE * scale;
    let x = rect.x as usize + (rect.w as usize).saturating_sub(icon_w) / 2;
    let y = rect.y as usize + (rect.h as usize).saturating_sub(icon_h) / 2;

    for (row_idx, row_bits) in icon.iter().enumerate() {
        for col_idx in 0..MODE_ICON_SIZE {
            if (row_bits >> ((MODE_ICON_SIZE - 1) - col_idx)) & 1 == 1 {
                draw_rect_rgba(
                    frame,
                    width as usize,
                    height as usize,
                    x + col_idx * scale,
                    y + row_idx * scale,
                    scale,
                    scale,
                    BUTTON_TEXT_COLOR,
                );
            }
        }
    }
}

fn draw_centered_label(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    text: &str,
    scale: usize,
    color: [u8; 4],
) {
    let text_width = measure_text(text, scale);
    let text_height = 7 * scale;
    let x = rect.x as usize + (rect.w as usize).saturating_sub(text_width) / 2;
    let y = rect.y as usize + (rect.h as usize).saturating_sub(text_height) / 2;
    draw_text(
        frame,
        width as usize,
        height as usize,
        x,
        y,
        text,
        scale,
        color,
    );
}

fn measure_text(text: &str, scale: usize) -> usize {
    let mut width = 0usize;
    for ch in text.chars() {
        width += glyph_width(ch) * scale;
    }
    width
}

fn draw_text(
    frame: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    text: &str,
    scale: usize,
    color: [u8; 4],
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        draw_glyph(frame, width, height, cursor_x, y, ch, scale, color);
        cursor_x += glyph_width(ch) * scale;
    }
}

fn draw_glyph(
    frame: &mut [u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    ch: char,
    scale: usize,
    color: [u8; 4],
) {
    let glyph = glyph_pattern(ch);
    for (row_idx, row_bits) in glyph.iter().enumerate() {
        for col_idx in 0..5 {
            if (row_bits >> (4 - col_idx)) & 1 == 1 {
                draw_rect_rgba(
                    frame,
                    width,
                    height,
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
        '+' => [
            0b00100, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00100,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '/' => [
            0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000,
        ],
        '\\' => [
            0b10000, 0b01000, 0b00100, 0b00010, 0b00001, 0b00000, 0b00000,
        ],
        _ => [
            0b11111, 0b10001, 0b00110, 0b00100, 0b00110, 0b10001, 0b11111,
        ],
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_rect_rgba(
    frame: &mut [u8],
    frame_width: usize,
    frame_height: usize,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: [u8; 4],
) {
    let x_end = x.saturating_add(w).min(frame_width);
    let y_end = y.saturating_add(h).min(frame_height);
    for yy in y..y_end {
        let row = yy * frame_width * 4;
        for xx in x..x_end {
            let idx = row + xx * 4;
            frame[idx] = color[0];
            frame[idx + 1] = color[1];
            frame[idx + 2] = color[2];
            frame[idx + 3] = color[3];
        }
    }
}
