use sokobanitron_gameplay::BoardView;

use crate::layout::{
    BoardViewport, BoardViewportOptions, UI_BUTTON_MARGIN, level_select_menu_clamp_start,
    level_select_menu_indices, level_select_menu_slot_rects, level_select_menu_start_index,
    level_select_scrollbar::ScrollbarState,
};

use super::{EntityVisualStyle, Renderer, level_select_scrollbar, pixels::fill_rect};

impl Renderer {
    pub fn draw_level_select_menu_contents(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        preview_boards: &[BoardView],
        resume_level: usize,
        page_start: usize,
    ) {
        if preview_boards.is_empty() {
            return;
        }
        let resume_level = resume_level.min(preview_boards.len().saturating_sub(1));
        let slots = level_select_menu_slot_rects(width, height);
        let page_start = level_select_menu_clamp_start(preview_boards.len(), page_start);
        let indices = level_select_menu_indices(preview_boards.len(), page_start);
        for (slot_idx, level_idx) in indices.into_iter().enumerate() {
            let Some(level_idx) = level_idx else {
                continue;
            };
            let Some(board) = preview_boards.get(level_idx) else {
                continue;
            };
            let (sx, sy, sw, sh) = slots[slot_idx];
            let pad = UI_BUTTON_MARGIN.max(8);
            let inner_w = sw.saturating_sub(pad * 2).max(1);
            let inner_h = sh.saturating_sub(pad * 2).max(1);
            let mut viewport = BoardViewport::fit_to_window_with_options(
                inner_w,
                inner_h,
                board,
                BoardViewportOptions::fill_available_space(),
            );
            viewport.origin_x += sx + pad as i32;
            viewport.origin_y += sy + pad as i32;
            self.draw_board_on_frame(
                frame,
                width,
                height,
                board,
                &viewport,
                true,
                EntityVisualStyle::Standard,
                false,
            );
            if level_idx == resume_level {
                draw_selection_brackets(frame, width, height, sx, sy, sw, sh);
            }
        }

        level_select_scrollbar::draw(
            frame,
            width,
            height,
            ScrollbarState {
                level_count: preview_boards.len(),
                visible_count: crate::layout::menu_slots_per_page(),
                page_start,
                return_start: level_select_menu_start_index(preview_boards.len(), resume_level),
            },
        );
    }
}

fn draw_selection_brackets(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) {
    let len = (w.min(h) / 6).max(8) as i32;
    let thickness = 4;
    let color = [200, 200, 200, 255];
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

#[cfg(test)]
mod tests {
    use super::Renderer;
    use sokobanitron_gameplay::{BoardView, TileKind};

    fn solved_board() -> BoardView {
        BoardView::new(
            3,
            3,
            vec![
                TileKind::Void,
                TileKind::Floor,
                TileKind::Void,
                TileKind::Floor,
                TileKind::Goal,
                TileKind::Floor,
                TileKind::Void,
                TileKind::Floor,
                TileKind::Void,
            ],
            vec![false, false, false, false, true, false, false, false, false],
            Some((1, 1)),
            None,
            true,
        )
    }

    #[test]
    fn level_select_thumbnails_do_not_opt_into_solved_visuals() {
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 128 * 128 * 4];

        renderer.draw_level_select_menu_contents(&mut frame, 128, 128, &[solved_board()], 0, 0);

        assert!(renderer.solved_box_bitmap_cache.is_empty());
        assert!(renderer.squint_player_bitmap_cache.is_empty());
    }
}
