use crate::{Renderer, pixels::fill_rect};

const TOP_TEXT: &str = "YOU";
const BOTTOM_TEXT: &str = "WIN";
const FONT_W: u32 = 5;
const FONT_H: u32 = 7;
const FONT_SPACING: u32 = 1;

// Dot-matrix style tuned to mimic the "DIRECT" sample:
// - each lit font pixel is expanded into a block of dots (bold)
// - each dot has a white core with a black border
const STROKE_SCALE: u32 = 3;
const DOT_WHITE: u32 = 10;
const DOT_BORDER: u32 = 2;
const DOT_SIZE: u32 = DOT_WHITE + DOT_BORDER * 2;
const DOT_STEP: u32 = DOT_SIZE - DOT_BORDER; // shared border keeps the same proportions at 2x
const LETTER_OUTLINE_THICKNESS: i32 = 2;
const CELL_STEP: u32 = STROKE_SCALE * DOT_STEP;
const MERGED_WHITE_SIZE: u32 = DOT_WHITE + STROKE_SCALE.saturating_sub(1) * DOT_STEP;
const OVERLAY_FILL_COLOR: [u8; 4] = [239, 239, 239, 255];
const OVERLAY_BORDER_COLOR: [u8; 4] = [0, 0, 0, 255];

const LINE_GAP_DOTS: u32 = 5;

impl Renderer {
    pub(crate) fn draw_win_overlay(&self, frame: &mut [u8], width: u32, height: u32) {
        let top_w = text_width_px(TOP_TEXT);
        let top_h = text_height_px();
        let bottom_w = text_width_px(BOTTOM_TEXT);
        let bottom_h = text_height_px();

        let content_w = top_w.max(bottom_w);
        let line_gap = LINE_GAP_DOTS * DOT_STEP;
        let content_h = top_h.saturating_add(line_gap).saturating_add(bottom_h);

        let content_x = (width.saturating_sub(content_w) / 2) as i32;
        let content_y = (height.saturating_sub(content_h) / 2) as i32;
        let top_x = content_x + ((content_w.saturating_sub(top_w) / 2) as i32);
        let top_y = content_y;
        let bottom_x = content_x + ((content_w.saturating_sub(bottom_w) / 2) as i32);
        let bottom_y = top_y + top_h as i32 + line_gap as i32;

        draw_text(frame, width, height, top_x, top_y, TOP_TEXT);
        draw_text(frame, width, height, bottom_x, bottom_y, BOTTOM_TEXT);
    }
}

fn text_cells(text: &str) -> (u32, u32) {
    if text.is_empty() {
        return (0, FONT_H);
    }
    let chars = text.chars().count() as u32;
    (
        chars * FONT_W + chars.saturating_sub(1) * FONT_SPACING,
        FONT_H,
    )
}

fn text_width_px(text: &str) -> u32 {
    let (cells_w, _) = text_cells(text);
    if cells_w == 0 {
        0
    } else {
        cells_w * CELL_STEP + 1
    }
}

fn text_height_px() -> u32 {
    FONT_H * CELL_STEP + 1
}

fn draw_text(frame: &mut [u8], frame_width: u32, frame_height: u32, x: i32, y: i32, text: &str) {
    let mut cursor_x = x;

    for ch in text.chars() {
        let glyph = glyph_pattern(ch.to_ascii_uppercase());

        // Pass 1: draw an outer black outline around each lit pixel.
        visit_lit_pixels(&glyph, cursor_x, y, |px, py| {
            draw_scaled_dot_outline(frame, frame_width, frame_height, px, py);
        });

        // Pass 2: draw the merged white fill on top of the black outline.
        visit_lit_pixels(&glyph, cursor_x, y, |px, py| {
            draw_scaled_dot_pixel(frame, frame_width, frame_height, px, py);
        });

        cursor_x += ((FONT_W + FONT_SPACING) * CELL_STEP) as i32;
    }
}

fn visit_lit_pixels<F>(glyph: &[u8; FONT_H as usize], x: i32, y: i32, mut f: F)
where
    F: FnMut(i32, i32),
{
    for (row, bits) in glyph.iter().enumerate() {
        for col in 0..FONT_W as usize {
            if (bits >> (FONT_W as usize - 1 - col)) & 1 == 1 {
                let px = x + (col as i32 * CELL_STEP as i32);
                let py = y + (row as i32 * CELL_STEP as i32);
                f(px, py);
            }
        }
    }
}

fn draw_scaled_dot_outline(frame: &mut [u8], frame_width: u32, frame_height: u32, x: i32, y: i32) {
    for sy in 0..STROKE_SCALE {
        for sx in 0..STROKE_SCALE {
            let cx = x + (sx * DOT_STEP) as i32;
            let cy = y + (sy * DOT_STEP) as i32;
            for oy in -LETTER_OUTLINE_THICKNESS..=LETTER_OUTLINE_THICKNESS {
                for ox in -LETTER_OUTLINE_THICKNESS..=LETTER_OUTLINE_THICKNESS {
                    if ox == 0 && oy == 0 {
                        continue;
                    }
                    let dx = cx + ox * DOT_STEP as i32;
                    let dy = cy + oy * DOT_STEP as i32;
                    draw_black_dot(frame, frame_width, frame_height, dx, dy);
                }
            }
        }
    }
}

fn draw_scaled_dot_pixel(frame: &mut [u8], frame_width: u32, frame_height: u32, x: i32, y: i32) {
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x + DOT_BORDER as i32,
        y + DOT_BORDER as i32,
        MERGED_WHITE_SIZE,
        MERGED_WHITE_SIZE,
        OVERLAY_FILL_COLOR,
    );
}

fn draw_black_dot(frame: &mut [u8], frame_width: u32, frame_height: u32, x: i32, y: i32) {
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y,
        DOT_SIZE,
        DOT_SIZE,
        OVERLAY_BORDER_COLOR,
    );
}

fn glyph_pattern(ch: char) -> [u8; 7] {
    match ch {
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        ' ' => [0, 0, 0, 0, 0, 0, 0],
        _ => [
            0b11111, 0b10001, 0b00110, 0b00100, 0b00110, 0b10001, 0b11111,
        ],
    }
}
