use crate::{Renderer, pixels::fill_rect};
use font8x8::{BASIC_FONTS, UnicodeFonts};

impl Renderer {
    pub(crate) fn draw_win_overlay(&self, frame: &mut [u8], width: u32, height: u32) {
        let line1 = "YOU";
        let line2 = "WIN";
        let max_w = width.saturating_mul(9) / 10;
        let max_h_total = height / 2;

        let mut best_scale = 1u32;
        let mut best_gap = 1u32;
        for scale in (1u32..=256).rev() {
            let gap = (scale / 2).max(1);
            let w = text_width(line1, scale).max(text_width(line2, scale));
            let h_total = 8u32
                .saturating_mul(scale)
                .saturating_mul(2)
                .saturating_add(gap);
            if w <= max_w && h_total <= max_h_total {
                best_scale = scale;
                best_gap = gap;
                break;
            }
        }

        let line_h = 8u32.saturating_mul(best_scale);
        let total_h = line_h.saturating_mul(2).saturating_add(best_gap);
        let y0 = ((height.saturating_sub(total_h)) / 2) as i32;
        let x1 = ((width.saturating_sub(text_width(line1, best_scale))) / 2) as i32;
        let x2 = ((width.saturating_sub(text_width(line2, best_scale))) / 2) as i32;
        let y2 = y0 + line_h as i32 + best_gap as i32;

        let outline = ((best_scale / 9).max(1)).saturating_mul(3);
        let deltas = [
            (-(outline as i32), 0),
            (outline as i32, 0),
            (0, -(outline as i32)),
            (0, outline as i32),
            (-(outline as i32), -(outline as i32)),
            (outline as i32, -(outline as i32)),
            (-(outline as i32), outline as i32),
            (outline as i32, outline as i32),
        ];

        for (dx, dy) in deltas {
            draw_text(
                frame,
                width,
                height,
                x1 + dx,
                y0 + dy,
                line1,
                best_scale,
                [0, 0, 0, 255],
            );
            draw_text(
                frame,
                width,
                height,
                x2 + dx,
                y2 + dy,
                line2,
                best_scale,
                [0, 0, 0, 255],
            );
        }

        draw_text(
            frame,
            width,
            height,
            x1,
            y0,
            line1,
            best_scale,
            self.theme.win_text,
        );
        draw_text(
            frame,
            width,
            height,
            x2,
            y2,
            line2,
            best_scale,
            self.theme.win_text,
        );
    }
}

fn text_width(text: &str, scale: u32) -> u32 {
    text.chars().count() as u32 * 8 * scale
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
