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

pub fn restart_button_rect() -> Rect {
    Rect {
        x: (config::WIDTH - config::RESTART_BUTTON_WIDTH) / 2,
        y: config::HEIGHT - config::FOOTER_HEIGHT
            + (config::FOOTER_HEIGHT - config::RESTART_BUTTON_HEIGHT) / 2,
        w: config::RESTART_BUTTON_WIDTH,
        h: config::RESTART_BUTTON_HEIGHT,
    }
}

pub fn draw_restart_ui(frame: &mut [u8]) {
    let footer_top = config::HEIGHT - config::FOOTER_HEIGHT;
    draw_rect_rgba(
        frame,
        0,
        footer_top,
        config::WIDTH,
        config::FOOTER_HEIGHT,
        [232, 232, 232, 255],
    );

    let button = restart_button_rect();
    draw_rect_rgba(frame, button.x, button.y, button.w, button.h, [255, 255, 255, 255]);
    draw_rect_outline_rgba(frame, button, 5, [0, 0, 0, 255]);
    draw_centered_label(
        frame,
        button,
        "RESTART",
        config::UI_TEXT_SCALE,
        config::UI_TEXT_SPACING,
        [0, 0, 0, 255],
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
