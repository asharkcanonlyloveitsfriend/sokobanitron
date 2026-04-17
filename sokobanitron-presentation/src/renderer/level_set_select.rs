use crate::layout::level_select_scrollbar::ScrollbarState;
use crate::layout::{
    ScreenRect, UI_BUTTON_MARGIN, level_set_rows_per_page, level_set_select_clamp_start,
    level_set_select_indices, level_set_select_row_rects, level_set_select_start_index,
};
use crate::screen_requests::{LevelSetListEntry, LevelSetSelectScreenRequest};

use super::{
    Renderer, RendererTheme, draw_text, level_select_scrollbar, measure_text_width,
    pixels::fill_rect,
};

const TITLE_SCALE: usize = 3;
const TITLE_SPACING: usize = 1;
const PROGRESS_TEXT_SCALE: usize = 2;
const PROGRESS_TEXT_SPACING: usize = 1;
impl Renderer {
    pub fn draw_level_set_select_menu_contents(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        screen: &LevelSetSelectScreenRequest,
    ) {
        if screen.entries.is_empty() {
            return;
        }

        let page_start = level_set_select_clamp_start(screen.entries.len(), screen.page_start);
        let row_rects = level_set_select_row_rects(width, height);
        let indices = level_set_select_indices(screen.entries.len(), page_start);

        for (slot_index, level_set_index) in indices.into_iter().enumerate() {
            let Some(level_set_index) = level_set_index else {
                continue;
            };
            let Some(entry) = screen.entries.get(level_set_index) else {
                continue;
            };
            let rect = row_rects[slot_index];
            draw_level_set_row(frame, width, height, rect, entry, self.theme);
            draw_row_separator(frame, width, height, rect, self.theme);
            if screen.active_level_set == Some(level_set_index) {
                draw_selection_brackets(frame, width, height, rect, self.theme);
            }
        }

        level_select_scrollbar::draw(
            frame,
            width,
            height,
            self.theme,
            ScrollbarState {
                level_count: screen.entries.len(),
                visible_count: level_set_rows_per_page().min(screen.entries.len()).max(1),
                page_start,
                return_start: level_set_select_start_index(
                    screen.entries.len(),
                    screen.active_level_set,
                ),
            },
        );
    }
}

fn draw_level_set_row(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    entry: &LevelSetListEntry,
    theme: RendererTheme,
) {
    let horizontal_pad = UI_BUTTON_MARGIN.max(10) as usize;
    let progress_text = format!(
        "{}/{}",
        entry.completed_puzzle_count, entry.total_puzzle_count
    );
    let progress_text_width =
        measure_text_width(&progress_text, PROGRESS_TEXT_SCALE, PROGRESS_TEXT_SPACING);
    let title_y = rect.y as usize
        + (rect.h as usize).saturating_sub(crate::renderer::PIXEL_FONT_HEIGHT * TITLE_SCALE) / 2;
    let progress_y = rect.y as usize
        + (rect.h as usize)
            .saturating_sub(crate::renderer::PIXEL_FONT_HEIGHT * PROGRESS_TEXT_SCALE)
            / 2;
    let progress_x = rect
        .x
        .saturating_add(rect.w)
        .saturating_sub(horizontal_pad as u32)
        .saturating_sub(progress_text_width as u32) as usize;

    let title_left = rect.x as usize + horizontal_pad;
    let title_right = progress_x.saturating_sub(horizontal_pad);
    if title_right > title_left {
        let title = fit_title_to_width(
            &entry.title.to_ascii_uppercase(),
            title_right - title_left,
            TITLE_SCALE,
            TITLE_SPACING,
        );
        draw_text(
            frame,
            width,
            height,
            title_left,
            title_y,
            &title,
            TITLE_SCALE,
            TITLE_SPACING,
            theme.gray_2,
        );
    }

    draw_text(
        frame,
        width,
        height,
        progress_x,
        progress_y,
        &progress_text,
        PROGRESS_TEXT_SCALE,
        PROGRESS_TEXT_SPACING,
        theme.gray_2,
    );
}

fn fit_title_to_width(text: &str, max_width: usize, scale: usize, spacing: usize) -> String {
    if measure_text_width(text, scale, spacing) <= max_width {
        return text.to_string();
    }

    const ELLIPSIS: &str = "...";
    let ellipsis_width = measure_text_width(ELLIPSIS, scale, spacing);
    if ellipsis_width >= max_width {
        return ELLIPSIS.to_string();
    }

    let mut fitted = String::new();
    for ch in text.chars() {
        let next = format!("{fitted}{ch}");
        if measure_text_width(&next, scale, spacing) + ellipsis_width > max_width {
            break;
        }
        fitted.push(ch);
    }
    fitted.push_str(ELLIPSIS);
    fitted
}

fn draw_row_separator(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    theme: RendererTheme,
) {
    fill_rect(
        frame,
        width,
        height,
        rect.x as i32 + UI_BUTTON_MARGIN as i32,
        rect.y.saturating_add(rect.h.saturating_sub(1)) as i32,
        rect.w.saturating_sub(UI_BUTTON_MARGIN.saturating_mul(2)),
        1,
        theme.gray_10,
    );
}

fn draw_selection_brackets(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    rect: ScreenRect,
    theme: RendererTheme,
) {
    let x = rect.x as i32;
    let y = rect.y as i32;
    let w = rect.w;
    let h = rect.h;
    let len = (w.min(h) / 6).max(8) as i32;
    let thickness = 4;
    let color = theme.gray_3;
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y,
        len as u32,
        thickness as u32,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y,
        thickness as u32,
        len as u32,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x + w as i32 - len,
        y,
        len as u32,
        thickness as u32,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x + w as i32 - 1,
        y,
        thickness as u32,
        len as u32,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y + h as i32 - 1,
        len as u32,
        thickness as u32,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y + h as i32 - len,
        thickness as u32,
        len as u32,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x + w as i32 - len,
        y + h as i32 - 1,
        len as u32,
        thickness as u32,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x + w as i32 - 1,
        y + h as i32 - len,
        thickness as u32,
        len as u32,
        color,
    );
}
