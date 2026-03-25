use crate::layout::ScreenRect;

pub const PIXEL_FONT_HEIGHT: usize = 7;

pub fn measure_text_width(text: &str, scale: usize, spacing: usize) -> usize {
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

#[allow(clippy::too_many_arguments)]
pub fn draw_centered_text_in_rect(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    text: &str,
    scale: usize,
    spacing: usize,
    color: [u8; 4],
) {
    let text_width = measure_text_width(text, scale, spacing);
    let text_height = PIXEL_FONT_HEIGHT * scale;
    let x = rect.x as usize + (rect.w as usize).saturating_sub(text_width) / 2;
    let y = rect.y as usize + (rect.h as usize).saturating_sub(text_height) / 2;
    draw_text(frame, width, height, x, y, text, scale, spacing, color);
}

#[allow(clippy::too_many_arguments)]
pub fn draw_icon_bits_in_rect(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    icon_rows: &[u16],
    icon_size: usize,
    scale: usize,
    color: [u8; 4],
) {
    let icon_w = icon_size * scale;
    let icon_h = icon_size * scale;
    let x = rect.x as usize + (rect.w as usize).saturating_sub(icon_w) / 2;
    let y = rect.y as usize + (rect.h as usize).saturating_sub(icon_h) / 2;

    for (row_idx, row_bits) in icon_rows.iter().take(icon_size).enumerate() {
        for col_idx in 0..icon_size {
            if (row_bits >> ((icon_size - 1) - col_idx)) & 1 == 1 {
                draw_rect_rgba(
                    frame,
                    width as usize,
                    height as usize,
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

#[allow(clippy::too_many_arguments)]
fn draw_text(
    frame: &mut [u8],
    width: u32,
    height: u32,
    x: usize,
    y: usize,
    text: &str,
    scale: usize,
    spacing: usize,
    color: [u8; 4],
) {
    let mut cursor_x = x;
    for ch in text.chars() {
        draw_glyph(frame, width, height, cursor_x, y, ch, scale, color);
        cursor_x += glyph_width(ch) * scale + spacing;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_glyph(
    frame: &mut [u8],
    width: u32,
    height: u32,
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
                    width as usize,
                    height as usize,
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

fn glyph_pattern(ch: char) -> [u8; PIXEL_FONT_HEIGHT] {
    match ch {
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
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
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        '/' => [
            0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b00000, 0b00000,
        ],
        '\\' => [
            0b10000, 0b01000, 0b00100, 0b00010, 0b00001, 0b00000, 0b00000,
        ],
        '+' => [
            0b00100, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00100,
        ],
        '-' => [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
        '?' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100,
        ],
        ' ' => [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
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
