use crate::config;

#[derive(Clone, Copy)]
pub struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl Rect {
    pub fn contains(self, px: usize, py: usize) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonAction {
    Previous,
    Next,
    Restart,
    Undo,
}

fn top_left_button_rect() -> Rect {
    Rect {
        x: config::UI_BUTTON_MARGIN,
        y: config::UI_BUTTON_MARGIN,
        w: config::UI_BUTTON_SIZE,
        h: config::UI_BUTTON_SIZE,
    }
}

fn top_right_button_rect() -> Rect {
    Rect {
        x: config::WIDTH.saturating_sub(config::UI_BUTTON_MARGIN + config::UI_BUTTON_SIZE),
        y: config::UI_BUTTON_MARGIN,
        w: config::UI_BUTTON_SIZE,
        h: config::UI_BUTTON_SIZE,
    }
}

fn bottom_left_button_rect() -> Rect {
    Rect {
        x: config::UI_BUTTON_MARGIN,
        y: config::HEIGHT.saturating_sub(config::UI_BUTTON_MARGIN + config::UI_BUTTON_SIZE),
        w: config::UI_BUTTON_SIZE,
        h: config::UI_BUTTON_SIZE,
    }
}

fn bottom_right_button_rect() -> Rect {
    Rect {
        x: config::WIDTH.saturating_sub(config::UI_BUTTON_MARGIN + config::UI_BUTTON_SIZE),
        y: config::HEIGHT.saturating_sub(config::UI_BUTTON_MARGIN + config::UI_BUTTON_SIZE),
        w: config::UI_BUTTON_SIZE,
        h: config::UI_BUTTON_SIZE,
    }
}

pub fn button_action_at(px: usize, py: usize) -> Option<ButtonAction> {
    if top_left_button_rect().contains(px, py) {
        return Some(ButtonAction::Previous);
    }
    if top_right_button_rect().contains(px, py) {
        return Some(ButtonAction::Next);
    }
    if bottom_left_button_rect().contains(px, py) {
        return Some(ButtonAction::Restart);
    }
    if bottom_right_button_rect().contains(px, py) {
        return Some(ButtonAction::Undo);
    }
    None
}

pub fn draw_controls_ui(frame: &mut [u8]) {
    draw_button(frame, top_left_button_rect(), "<");
    draw_button(frame, top_right_button_rect(), ">");
    draw_button(frame, bottom_left_button_rect(), "R");
    draw_button(frame, bottom_right_button_rect(), "U");
}

pub fn draw_level_flash_overlay(frame: &mut [u8], level_number: usize) {
    let text = level_number.to_string();
    let mut best_scale = 1usize;
    let mut best_spacing = 1usize;
    let max_w = config::WIDTH.saturating_mul(9) / 10;
    let max_h = config::HEIGHT.saturating_mul(9) / 10;

    for scale in (1..=512usize).rev() {
        let spacing = (scale / 3).max(1);
        let w = measure_text(&text, scale, spacing);
        let h = 7usize.saturating_mul(scale);
        if w <= max_w && h <= max_h {
            best_scale = scale;
            best_spacing = spacing;
            break;
        }
    }

    let text_w = measure_text(&text, best_scale, best_spacing);
    let text_h = 7usize.saturating_mul(best_scale);
    let x = (config::WIDTH.saturating_sub(text_w)) / 2;
    let y = (config::HEIGHT.saturating_sub(text_h)) / 2;
    draw_outlined_text(frame, x, y, &text, best_scale, best_spacing);
}

fn draw_outlined_text(
    frame: &mut [u8],
    x: usize,
    y: usize,
    text: &str,
    scale: usize,
    spacing: usize,
) {
    let outline = (scale / 9).max(1);
    let outline_color = [0, 0, 0, 255];
    let fill_color = [245, 245, 245, 255];

    let deltas = [
        (-(outline as isize), 0isize),
        (outline as isize, 0isize),
        (0isize, -(outline as isize)),
        (0isize, outline as isize),
        (-(outline as isize), -(outline as isize)),
        (outline as isize, -(outline as isize)),
        (-(outline as isize), outline as isize),
        (outline as isize, outline as isize),
    ];

    for (dx, dy) in deltas {
        let ox = x.saturating_add_signed(dx);
        let oy = y.saturating_add_signed(dy);
        draw_text(frame, ox, oy, text, scale, spacing, outline_color);
    }

    draw_text(frame, x, y, text, scale, spacing, fill_color);
}

fn draw_button(frame: &mut [u8], rect: Rect, label: &str) {
    draw_centered_label(
        frame,
        rect,
        label,
        config::UI_TEXT_SCALE,
        0,
        [220, 220, 220, 255],
    );
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
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        '>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
        ' ' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
        _ => [0b11111, 0b10001, 0b00110, 0b00100, 0b00110, 0b10001, 0b11111],
    }
}

fn draw_rect_rgba(frame: &mut [u8], x: usize, y: usize, w: usize, h: usize, color: [u8; 4]) {
    let x_end = (x + w).min(config::WIDTH);
    let y_end = (y + h).min(config::HEIGHT);

    for yy in y..y_end {
        let row = yy * config::WIDTH * 4;
        for xx in x..x_end {
            let idx = row + xx * 4;
            frame[idx] = color[0];
            frame[idx + 1] = color[1];
            frame[idx + 2] = color[2];
            frame[idx + 3] = color[3];
        }
    }
}
