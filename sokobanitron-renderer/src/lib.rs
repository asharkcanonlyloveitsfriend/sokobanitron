mod background;
mod controls;
mod entities;
mod menu;
mod overlay;
mod pixels;
mod sprites;
mod tiles;
mod trail;
mod viewport;

use image::RgbaImage;
use sokobanitron_gameplay::{BoardView, TileKind};
use std::collections::HashMap;

pub use controls::{
    BOARD_HORIZONTAL_MARGIN, BOARD_VERTICAL_MARGIN, ControlsButtonAction, ControlsButtonRects,
    ControlsUiMode, ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE, UI_MENU_BUTTON_HEIGHT,
    board_viewport_margins, controls_button_action_at, controls_button_rects, draw_controls_ui,
};
pub use menu::{
    MenuNavAction, level_select_menu_clamp_start, level_select_menu_indices,
    level_select_menu_nav_action_at, level_select_menu_nav_button_rects,
    level_select_menu_slot_rects, level_select_menu_start_for_nav, level_select_menu_start_index,
    level_select_menu_step_start, level_select_menu_target_at,
};
pub use viewport::{BoardViewport, BoardViewportOptions};
pub type Rgba = [u8; 4];

const BG_SPACE_PNG: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/bg_space.png"));

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

#[derive(Debug, Clone, Copy)]
pub struct RendererTheme {
    pub floor_fill: Rgba,
    pub floor_stroke: Rgba,
    pub goal_fill: Rgba,
    pub box_primary: Rgba,
    pub box_highlight: Rgba,
    pub box_shadow: Rgba,
    pub selected_box_primary: Rgba,
    pub selected_box_highlight: Rgba,
    pub selected_box_shadow: Rgba,
    pub player_body: Rgba,
    pub player_highlight: Rgba,
    pub player_eye: Rgba,
    pub player_limb: Rgba,
    pub win_panel_fill: Rgba,
    pub win_panel_stroke: Rgba,
    pub win_text: Rgba,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RendererOverrides {
    pub floor_fill: Option<Rgba>,
    pub floor_stroke: Option<Rgba>,
    pub goal_fill: Option<Rgba>,
    pub box_primary: Option<Rgba>,
    pub box_highlight: Option<Rgba>,
    pub box_shadow: Option<Rgba>,
    pub selected_box_primary: Option<Rgba>,
    pub selected_box_highlight: Option<Rgba>,
    pub selected_box_shadow: Option<Rgba>,
    pub player_body: Option<Rgba>,
    pub player_highlight: Option<Rgba>,
    pub player_eye: Option<Rgba>,
    pub player_limb: Option<Rgba>,
    pub win_panel_fill: Option<Rgba>,
    pub win_panel_stroke: Option<Rgba>,
    pub win_text: Option<Rgba>,
}

impl Default for RendererTheme {
    fn default() -> Self {
        Self {
            floor_fill: [255, 255, 255, 255],
            floor_stroke: [240, 240, 240, 255],
            goal_fill: [224, 224, 224, 255],
            box_primary: [120, 129, 144, 255],
            box_highlight: [156, 163, 175, 255],
            box_shadow: [75, 85, 99, 255],
            selected_box_primary: [95, 103, 117, 255],
            selected_box_highlight: [123, 133, 150, 255],
            selected_box_shadow: [74, 81, 96, 255],
            player_body: [156, 163, 175, 255],
            player_highlight: [249, 250, 251, 255],
            player_eye: [2, 6, 23, 255],
            player_limb: [107, 114, 128, 255],
            win_panel_fill: [8, 12, 20, 220],
            win_panel_stroke: [255, 255, 255, 255],
            win_text: [255, 255, 255, 255],
        }
    }
}

impl RendererTheme {
    pub fn apply_overrides(mut self, overrides: RendererOverrides) -> Self {
        if let Some(v) = overrides.floor_fill {
            self.floor_fill = v;
        }
        if let Some(v) = overrides.floor_stroke {
            self.floor_stroke = v;
        }
        if let Some(v) = overrides.goal_fill {
            self.goal_fill = v;
        }
        if let Some(v) = overrides.box_primary {
            self.box_primary = v;
        }
        if let Some(v) = overrides.box_highlight {
            self.box_highlight = v;
        }
        if let Some(v) = overrides.box_shadow {
            self.box_shadow = v;
        }
        if let Some(v) = overrides.selected_box_primary {
            self.selected_box_primary = v;
        }
        if let Some(v) = overrides.selected_box_highlight {
            self.selected_box_highlight = v;
        }
        if let Some(v) = overrides.selected_box_shadow {
            self.selected_box_shadow = v;
        }
        if let Some(v) = overrides.player_body {
            self.player_body = v;
        }
        if let Some(v) = overrides.player_highlight {
            self.player_highlight = v;
        }
        if let Some(v) = overrides.player_eye {
            self.player_eye = v;
        }
        if let Some(v) = overrides.player_limb {
            self.player_limb = v;
        }
        if let Some(v) = overrides.win_panel_fill {
            self.win_panel_fill = v;
        }
        if let Some(v) = overrides.win_panel_stroke {
            self.win_panel_stroke = v;
        }
        if let Some(v) = overrides.win_text {
            self.win_text = v;
        }
        self
    }
}

pub struct Renderer {
    theme: RendererTheme,
    source_background: RgbaImage,
    cached_background: Vec<u8>,
    cached_width: u32,
    cached_height: u32,
    box_bitmap_cache: HashMap<u32, Vec<u8>>,
    selected_box_bitmap_cache: HashMap<u32, Vec<u8>>,
    player_bitmap_cache: HashMap<u32, Vec<u8>>,
    player_blink_bitmap_cache: HashMap<u32, Vec<u8>>,
}

impl Renderer {
    pub fn new() -> Self {
        Self::with_theme(RendererTheme::default())
    }

    pub fn with_overrides(overrides: RendererOverrides) -> Self {
        Self::with_theme(RendererTheme::default().apply_overrides(overrides))
    }

    pub fn with_theme(theme: RendererTheme) -> Self {
        let source_background = image::load_from_memory(BG_SPACE_PNG)
            .expect("failed to decode bg_space.png")
            .into_rgba8();

        Self {
            theme,
            source_background,
            cached_background: Vec::new(),
            cached_width: 0,
            cached_height: 0,
            box_bitmap_cache: HashMap::new(),
            selected_box_bitmap_cache: HashMap::new(),
            player_bitmap_cache: HashMap::new(),
            player_blink_bitmap_cache: HashMap::new(),
        }
    }

    pub fn draw(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
    ) {
        self.draw_with_box_trail(frame, width, height, board, viewport, None);
    }

    pub fn draw_background_only(&mut self, frame: &mut [u8], width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.ensure_cached_background(width, height);
        frame.copy_from_slice(&self.cached_background);
    }

    pub fn draw_board_on_frame(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        draw_player: bool,
        draw_win_overlay: bool,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        self.draw_floor_tiles(frame, width, height, board, viewport);
        self.draw_boxes(frame, width, height, board, viewport);
        if draw_player {
            self.draw_player(frame, width, height, board, viewport);
        }
        if draw_win_overlay && board.is_won() {
            self.draw_win_overlay(frame, width, height);
        }
    }

    pub fn draw_with_box_trail(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        box_trail: Option<&[(u32, u32)]>,
    ) {
        self.draw_with_box_trail_options(
            frame, width, height, board, viewport, box_trail, true, true,
        );
    }

    pub fn draw_with_box_trail_options(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        box_trail: Option<&[(u32, u32)]>,
        draw_player: bool,
        draw_win_overlay: bool,
    ) {
        self.draw_with_box_trail_progress_options(
            frame,
            width,
            height,
            board,
            viewport,
            box_trail,
            None,
            draw_player,
            draw_win_overlay,
        );
    }

    pub fn draw_with_box_trail_progress_options(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        box_trail: Option<&[(u32, u32)]>,
        box_trail_consumed_segments: Option<f32>,
        draw_player: bool,
        draw_win_overlay: bool,
    ) {
        self.draw_with_box_trail_progress_effects(
            frame,
            width,
            height,
            board,
            viewport,
            box_trail,
            box_trail_consumed_segments,
            draw_player,
            false,
            draw_win_overlay,
        );
    }

    pub fn draw_with_box_trail_progress_effects(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        box_trail: Option<&[(u32, u32)]>,
        box_trail_consumed_segments: Option<f32>,
        draw_player: bool,
        draw_player_blink: bool,
        draw_win_overlay: bool,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        self.ensure_cached_background(width, height);
        frame.copy_from_slice(&self.cached_background);
        self.draw_floor_tiles(frame, width, height, board, viewport);
        if let Some(path) = box_trail {
            if let Some(consumed) = box_trail_consumed_segments {
                self.draw_box_trail_with_progress(frame, width, height, viewport, path, consumed);
            } else {
                self.draw_box_trail(frame, width, height, viewport, path);
            }
        }
        self.draw_boxes(frame, width, height, board, viewport);
        if draw_player {
            self.draw_player(frame, width, height, board, viewport);
            if draw_player_blink {
                self.draw_player_blink(frame, width, height, board, viewport);
            }
        }
        if draw_win_overlay && board.is_won() {
            self.draw_win_overlay(frame, width, height);
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
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
