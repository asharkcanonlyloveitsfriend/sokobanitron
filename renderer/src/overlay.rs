use crate::pixels::{fill_rect, stroke_rect};
use font8x8::{BASIC_FONTS, UnicodeFonts};

pub(crate) fn draw_win_overlay(frame: &mut [u8], width: u32, height: u32) {
    let panel_w = (width as f32 * 0.55) as u32;
    let panel_h = (height as f32 * 0.24) as u32;
    let panel_x = ((width - panel_w) / 2) as i32;
    let panel_y = ((height - panel_h) / 2) as i32;

    fill_rect(frame, width, height, panel_x, panel_y, panel_w, panel_h, [8, 12, 20, 220]);
    stroke_rect(
        frame,
        width,
        height,
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        [255, 255, 255, 255],
    );

    let title_scale = (panel_h / 18).max(2);
    let title = "You win.";
    let title_px_w = (title.chars().count() as u32) * 8 * title_scale;
    let title_x = panel_x + ((panel_w.saturating_sub(title_px_w)) / 2) as i32;
    let title_y = panel_y + ((panel_h as i32 - (8 * title_scale as i32)) / 2);

    draw_text(
        frame,
        width,
        height,
        title_x,
        title_y,
        title,
        title_scale,
        [255, 255, 255, 255],
    );
}

fn draw_text(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    x: i32,
    y: i32,
    text: &str,
    scale: u32,
    color: [u8; 4],
) {
    for (i, ch) in text.chars().enumerate() {
        if let Some(glyph) = BASIC_FONTS.get(ch) {
            let gx = x + (i as i32 * 8 * scale as i32);
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..8u8 {
                    if (bits >> col) & 1 == 0 {
                        continue;
                    }
                    fill_rect(
                        frame,
                        frame_width,
                        frame_height,
                        gx + ((7 - col as i32) * scale as i32),
                        y + (row as i32 * scale as i32),
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
    }
}
