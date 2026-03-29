//! Layout and geometry owned by the presentation layer.

mod controls;
mod editor;
mod level_select;
pub(crate) mod level_select_scrollbar;
mod level_set_select;
mod overlay;
mod viewport;

use sokobanitron_gameplay::{BoardView, TileKind};

pub use controls::{
    BOARD_HORIZONTAL_MARGIN, BOARD_VERTICAL_MARGIN, ControlsButtonRects, ControlsUiMode,
    ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE, UI_MENU_BUTTON_HEIGHT, board_viewport_margins,
    bottom_left_corner_button_rect, bottom_right_corner_button_rect, controls_button_rects,
    top_left_level_button_rect, top_menu_toggle_button_hit_rect, top_menu_toggle_button_rect,
};
pub use editor::{
    editor_bottom_left_button_rect, editor_bottom_right_button_rect, editor_viewport_size,
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
pub use overlay::{gameplay_menu_level_set_button_rect, overlay_primary_action_button_rect};
pub use viewport::{BoardViewport, BoardViewportOptions};

pub fn fit_board_viewport_for_controls(
    width: u32,
    height: u32,
    board: &BoardView,
) -> BoardViewport {
    let board_cols = board.width().max(1);
    let board_rows = board.height().max(1);
    let top_safe_margin = BOARD_VERTICAL_MARGIN;
    let side_margin_cap = UI_BUTTON_SIZE;
    let controls: ControlsButtonRects = controls_button_rects(width, height);
    let forbidden = [
        to_pixel_rect(top_left_level_button_rect()),
        to_pixel_rect(controls.restart),
        to_pixel_rect(controls.undo),
    ];
    let visible_cells = non_void_cells(board);

    let max_cell_w = width / board_cols;
    let max_cell_h = height.saturating_sub(top_safe_margin) / board_rows;
    let max_cell_size = max_cell_w.min(max_cell_h).max(1);

    for cell_size in (1..=max_cell_size).rev() {
        let side_margin = side_margin_cap.min(cell_size);
        let board_pixel_width = board_cols * cell_size;
        let board_pixel_height = board_rows * cell_size;

        if board_pixel_width > width.saturating_sub(side_margin * 2) {
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

fn non_void_cells(board: &BoardView) -> Vec<(u32, u32)> {
    let mut cells = Vec::new();
    for y in 0..board.height() {
        for x in 0..board.width() {
            if board.tile(x, y) != TileKind::Void {
                cells.push((x, y));
            }
        }
    }
    cells
}

fn overlaps_forbidden_buttons(
    origin_x: i32,
    origin_y: i32,
    cell_size: u32,
    non_void_cells: &[(u32, u32)],
    forbidden: &[PixelRect],
) -> bool {
    let cell_size = cell_size as i32;
    non_void_cells.iter().any(|(x, y)| {
        let left = origin_x + (*x as i32 * cell_size);
        let top = origin_y + (*y as i32 * cell_size);
        let tile_rect = PixelRect {
            left,
            top,
            right: left + cell_size,
            bottom: top + cell_size,
        };
        forbidden.iter().any(|rect| tile_rect.intersects(*rect))
    })
}

#[cfg(test)]
mod tests {
    use super::{
        PixelRect, UI_BUTTON_SIZE, controls_button_rects, fit_board_viewport_for_controls,
        overlaps_forbidden_buttons, to_pixel_rect,
    };
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
    fn fitted_viewport_avoids_bottom_button_overlap_for_non_void_tiles() {
        let board = board_with_tile(12, 10, TileKind::Floor);
        let viewport = fit_board_viewport_for_controls(670, 905, &board);
        let controls = controls_button_rects(670, 905);
        let forbidden: [PixelRect; 2] = [
            to_pixel_rect(controls.restart),
            to_pixel_rect(controls.undo),
        ];
        let solid_cells = (0..board.height())
            .flat_map(|y| (0..board.width()).map(move |x| (x, y)))
            .collect::<Vec<_>>();

        assert!(!overlaps_forbidden_buttons(
            viewport.origin_x,
            viewport.origin_y,
            viewport.cell_size,
            &solid_cells,
            &forbidden,
        ));
    }

    #[test]
    fn small_boards_keep_capped_side_margin() {
        let board = board_with_tile(4, 4, TileKind::Floor);
        let viewport = fit_board_viewport_for_controls(670, 905, &board);
        assert!((viewport.origin_x as u32) >= UI_BUTTON_SIZE);
    }
}
