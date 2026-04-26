//! Layout and geometry owned by the presentation layer.

mod controls;
mod editor;
mod level_select;
pub(crate) mod level_select_scrollbar;
mod level_set_select;
mod overlay;
mod viewport;

use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};

pub use controls::{
    BOARD_HORIZONTAL_MARGIN, BOARD_VERTICAL_MARGIN, ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE,
    UI_MENU_BUTTON_HEIGHT, board_viewport_margins, bottom_left_corner_button_rect,
    bottom_right_corner_button_rect, top_left_level_button_rect,
    top_menu_toggle_button_expanded_hit_rect, top_menu_toggle_button_visible_rect,
};
pub use editor::{
    editor_bottom_left_button_rect, editor_bottom_right_button_rect, editor_mode_button_rect,
    editor_mode_menu_option_rects, editor_mode_menu_rect, editor_viewport_size,
};
pub(crate) use level_select::menu_slots_per_page;
pub use level_select::{
    level_select_menu_clamp_start, level_select_menu_indices, level_select_menu_slot_rects,
    level_select_menu_start_index, level_select_menu_step_start,
};
pub(crate) use level_set_select::level_set_rows_per_page;
pub use level_set_select::{
    level_set_select_clamp_start, level_set_select_indices, level_set_select_row_rects,
    level_set_select_start_index, level_set_select_step_start,
};
pub use overlay::{
    gameplay_menu_level_set_button_rect, overlay_primary_action_button_rect,
    overlay_secondary_action_button_rect,
};
pub use viewport::{BoardViewport, BoardViewportOptions};

pub fn fit_board_viewport_for_controls(
    width: u32,
    height: u32,
    board: &BoardView,
) -> BoardViewport {
    fit_board_viewport_for_controls_capped(width, height, board, u32::MAX)
}

pub fn fit_board_viewport_for_controls_capped(
    width: u32,
    height: u32,
    board: &BoardView,
    max_cell_size: u32,
) -> BoardViewport {
    let board_cols = board.width().max(1);
    let board_rows = board.height().max(1);
    let top_safe_margin = BOARD_VERTICAL_MARGIN;
    let forbidden = [to_pixel_rect(top_left_level_button_rect())];
    let visible_cells = non_void_cells(board);

    let max_cell_w = width / board_cols;
    let max_cell_h = height.saturating_sub(top_safe_margin) / board_rows;
    let max_candidate_cell_size = clamp_cell_size(max_cell_w.min(max_cell_h), max_cell_size);

    for cell_size in (1..=max_candidate_cell_size).rev() {
        let board_pixel_width = board_cols * cell_size;
        let board_pixel_height = board_rows * cell_size;

        if board_pixel_width > width {
            continue;
        }
        if board_pixel_height > height.saturating_sub(top_safe_margin) {
            continue;
        }

        let origin_x = (width.saturating_sub(board_pixel_width) / 2) as i32;
        let centered_origin_y = {
            let below_top = height.saturating_sub(top_safe_margin);
            top_safe_margin + below_top.saturating_sub(board_pixel_height) / 2
        };
        let top_aligned_origin_y = top_safe_margin;

        let centered_overlaps = overlaps_forbidden_buttons(
            origin_x,
            centered_origin_y as i32,
            cell_size,
            &visible_cells,
            &forbidden,
        );
        let top_aligned_overlaps = overlaps_forbidden_buttons(
            origin_x,
            top_aligned_origin_y as i32,
            cell_size,
            &visible_cells,
            &forbidden,
        );

        let origin_y = if !centered_overlaps {
            centered_origin_y
        } else if !top_aligned_overlaps {
            top_aligned_origin_y
        } else {
            continue;
        };

        return BoardViewport {
            origin_x,
            origin_y: origin_y as i32,
            cell_size,
            board_pixel_width,
            board_pixel_height,
            outer_margin_tiles: 0,
        };
    }

    let mut viewport = BoardViewport::fit_to_window_with_options(
        width.max(1),
        height.saturating_sub(top_safe_margin).max(1),
        board,
        BoardViewportOptions::fill_available_space(),
    );
    clamp_viewport_cell_size(
        &mut viewport,
        width.max(1),
        height.saturating_sub(top_safe_margin).max(1),
        board,
        max_cell_size,
    );
    viewport.origin_y += top_safe_margin as i32;
    viewport
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PixelRect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl PixelRect {
    fn intersects(self, other: PixelRect) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }
}

fn to_pixel_rect(rect: ScreenRect) -> PixelRect {
    PixelRect {
        left: rect.x as i32,
        top: rect.y as i32,
        right: rect.x.saturating_add(rect.w) as i32,
        bottom: rect.y.saturating_add(rect.h) as i32,
    }
}

fn non_void_cells(board: &BoardView) -> Vec<BoardCell> {
    let mut cells = Vec::new();
    for cell in board.cells() {
        if board.tile(cell) != TileKind::Void {
            cells.push(cell);
        }
    }
    cells
}

fn overlaps_forbidden_buttons(
    origin_x: i32,
    origin_y: i32,
    cell_size: u32,
    non_void_cells: &[BoardCell],
    forbidden: &[PixelRect],
) -> bool {
    let cell_size = cell_size as i32;
    non_void_cells.iter().any(|cell| {
        let left = origin_x + (cell.x as i32 * cell_size);
        let top = origin_y + (cell.y as i32 * cell_size);
        let tile_rect = PixelRect {
            left,
            top,
            right: left + cell_size,
            bottom: top + cell_size,
        };
        forbidden.iter().any(|rect| tile_rect.intersects(*rect))
    })
}

fn clamp_cell_size(cell_size: u32, max_cell_size: u32) -> u32 {
    cell_size.min(max_cell_size.max(1)).max(1)
}

fn clamp_viewport_cell_size(
    viewport: &mut BoardViewport,
    window_width: u32,
    window_height: u32,
    board: &BoardView,
    max_cell_size: u32,
) {
    let clamped_cell_size = clamp_cell_size(viewport.cell_size, max_cell_size);
    if clamped_cell_size == viewport.cell_size {
        return;
    }

    let margin = viewport.outer_margin_tiles;
    let cols = board
        .width()
        .saturating_add(margin.saturating_mul(2))
        .max(1);
    let rows = board
        .height()
        .saturating_add(margin.saturating_mul(2))
        .max(1);
    let board_pixel_width = cols * clamped_cell_size;
    let board_pixel_height = rows * clamped_cell_size;

    viewport.origin_x = ((window_width as i32) - (board_pixel_width as i32)) / 2;
    viewport.origin_y = ((window_height as i32) - (board_pixel_height as i32)) / 2;
    viewport.cell_size = clamped_cell_size;
    viewport.board_pixel_width = board_pixel_width;
    viewport.board_pixel_height = board_pixel_height;
}

#[cfg(test)]
mod tests {
    use super::{
        BOARD_VERTICAL_MARGIN, PixelRect, fit_board_viewport_for_controls,
        fit_board_viewport_for_controls_capped, overlaps_forbidden_buttons, to_pixel_rect,
        top_left_level_button_rect,
    };
    use crate::layout::{BoardViewport, BoardViewportOptions};
    use sokobanitron_gameplay::{BoardView, TileKind};

    fn board_with_tile(width: u32, height: u32, tile: TileKind) -> BoardView {
        let len = (width * height) as usize;
        BoardView::new(
            width,
            height,
            vec![tile; len],
            vec![false; len],
            None,
            None,
            false,
        )
    }

    #[test]
    fn fitted_viewport_avoids_level_button_overlap_for_non_void_tiles() {
        let board = board_with_tile(12, 10, TileKind::Floor);
        let viewport = fit_board_viewport_for_controls(670, 905, &board);
        let forbidden: [PixelRect; 1] = [to_pixel_rect(top_left_level_button_rect())];
        let solid_cells = board.cells().collect::<Vec<_>>();

        assert!(!overlaps_forbidden_buttons(
            viewport.origin_x,
            viewport.origin_y,
            viewport.cell_size,
            &solid_cells,
            &forbidden,
        ));
    }

    #[test]
    fn wide_boards_can_use_side_space_when_only_top_ui_matters() {
        let board = board_with_tile(8, 8, TileKind::Floor);
        let viewport = fit_board_viewport_for_controls(670, 905, &board);
        assert!((viewport.origin_x as u32) < 76);
    }

    #[test]
    fn fitted_viewport_respects_max_cell_size() {
        let board = board_with_tile(1, 1, TileKind::Floor);
        let viewport = fit_board_viewport_for_controls_capped(670, 905, &board, 42);

        assert_eq!(viewport.cell_size, 42);
    }

    #[test]
    fn fallback_viewport_clamps_to_max_cell_size() {
        let width: u32 = 300;
        let height: u32 = 140;
        let max_cell_size: u32 = 1;
        let board = board_with_tile(140, 20, TileKind::Floor);
        let solid_cells = board.cells().collect::<Vec<_>>();
        let forbidden: [PixelRect; 1] = [to_pixel_rect(top_left_level_button_rect())];
        let centered_origin_x = (width.saturating_sub(board.width() * max_cell_size) / 2) as i32;
        let centered_origin_y = {
            let below_top = height.saturating_sub(BOARD_VERTICAL_MARGIN);
            BOARD_VERTICAL_MARGIN + below_top.saturating_sub(board.height() * max_cell_size) / 2
        };

        assert!(overlaps_forbidden_buttons(
            centered_origin_x,
            centered_origin_y as i32,
            max_cell_size,
            &solid_cells,
            &forbidden,
        ));
        assert!(overlaps_forbidden_buttons(
            centered_origin_x,
            BOARD_VERTICAL_MARGIN as i32,
            max_cell_size,
            &solid_cells,
            &forbidden,
        ));

        let uncapped_fallback = BoardViewport::fit_to_window_with_options(
            width,
            height.saturating_sub(BOARD_VERTICAL_MARGIN).max(1),
            &board,
            BoardViewportOptions::fill_available_space(),
        );
        assert!(uncapped_fallback.cell_size > max_cell_size);

        let viewport = fit_board_viewport_for_controls_capped(width, height, &board, max_cell_size);

        assert_eq!(viewport.cell_size, max_cell_size);
        assert_eq!(viewport.board_pixel_width, board.width() * max_cell_size);
        assert_eq!(viewport.board_pixel_height, board.height() * max_cell_size);
    }
}
