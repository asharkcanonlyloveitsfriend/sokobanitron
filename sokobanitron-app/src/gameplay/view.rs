//! App-owned gameplay view state and viewport shaping.
//!
//! This module keeps gameplay surface sizing on the app side of the boundary so both hit-testing
//! and rendering can use the same device-agnostic viewport computation.

use crate::persistence::LevelSetCatalogEntry;
use crate::shared::{DoubleTapTracker, TouchPointerState};
use presentation::layout::{
    BOARD_VERTICAL_MARGIN, BoardViewport, fit_board_viewport_for_controls_capped,
};
use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};
use std::time::Duration;

const DEFAULT_GAMEPLAY_WIDTH: u32 = 670;
const DEFAULT_GAMEPLAY_HEIGHT: u32 = 891;
const DEFAULT_GAMEPLAY_MAX_CELL_SIZE: u32 = 260;
const ZOOM_AXIS_TILE_REDUCTION_HALF_TILES: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayUiState {
    pub surface_width: u32,
    pub surface_height: u32,
    pub max_cell_size: u32,
    pub viewport: GameplayViewportState,
    pub level_sets: Vec<LevelSetCatalogEntry>,
    pub active_level_set: Option<usize>,
    pub(crate) interaction: GameplayInteractionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GameplayViewportState {
    pub zoomed_in: bool,
    pub zoom_origin_x: u32,
    pub zoom_origin_y: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GameplayInteractionState {
    pub(crate) touch: TouchPointerState,
    pub(crate) double_tap: DoubleTapTracker<BoardCell>,
    pub(crate) double_tap_window: Duration,
}

impl Default for GameplayInteractionState {
    fn default() -> Self {
        Self {
            touch: TouchPointerState::default(),
            double_tap: DoubleTapTracker::default(),
            double_tap_window: Duration::from_millis(325),
        }
    }
}

impl Default for GameplayUiState {
    fn default() -> Self {
        Self {
            surface_width: DEFAULT_GAMEPLAY_WIDTH,
            surface_height: DEFAULT_GAMEPLAY_HEIGHT,
            max_cell_size: DEFAULT_GAMEPLAY_MAX_CELL_SIZE,
            viewport: GameplayViewportState::default(),
            level_sets: Vec::new(),
            active_level_set: None,
            interaction: GameplayInteractionState::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayVisibleBoardWindow {
    pub board: BoardView,
    pub viewport: BoardViewport,
    pub board_origin_x: u32,
    pub board_origin_y: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NonVoidBounds {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZoomAxisWindow {
    crop_origin: u32,
    crop_extent: u32,
    core_extent: u32,
    has_leading_continuation: bool,
    has_trailing_continuation: bool,
    at_leading_bound: bool,
    at_trailing_bound: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ZoomSpec {
    target_cell_size: u32,
    visible_cols: u32,
    visible_rows: u32,
}

impl GameplayVisibleBoardWindow {
    pub fn screen_to_world_cell(&self, screen_x: f64, screen_y: f64) -> Option<BoardCell> {
        self.viewport
            .screen_to_cell(screen_x, screen_y, &self.board)
            .map(|cell| BoardCell::new(cell.x + self.board_origin_x, cell.y + self.board_origin_y))
    }

    pub fn world_to_local_cell(&self, cell: BoardCell) -> Option<BoardCell> {
        let max_x = self.board_origin_x + self.board.width();
        let max_y = self.board_origin_y + self.board.height();
        if cell.x < self.board_origin_x
            || cell.y < self.board_origin_y
            || cell.x >= max_x
            || cell.y >= max_y
        {
            return None;
        }

        Some(BoardCell::new(
            cell.x - self.board_origin_x,
            cell.y - self.board_origin_y,
        ))
    }
}

pub fn resize_gameplay_surface(gameplay: &mut GameplayUiState, width: u32, height: u32) {
    gameplay.surface_width = width.max(1);
    gameplay.surface_height = height.max(1);
}

pub fn set_gameplay_touch_slop(gameplay: &mut GameplayUiState, tap_slop_px: i32) {
    gameplay.interaction.touch.set_tap_slop(tap_slop_px);
}

pub fn set_gameplay_double_tap_window(gameplay: &mut GameplayUiState, window: Duration) {
    gameplay.interaction.double_tap_window = window;
}

pub fn set_gameplay_max_cell_size(gameplay: &mut GameplayUiState, max_cell_size: u32) {
    gameplay.max_cell_size = max_cell_size.max(1);
}

pub fn set_gameplay_level_sets(
    gameplay: &mut GameplayUiState,
    level_sets: Vec<LevelSetCatalogEntry>,
    active_level_set: Option<usize>,
) {
    gameplay.active_level_set = active_level_set.filter(|&index| index < level_sets.len());
    gameplay.level_sets = level_sets;
}

pub fn set_gameplay_zoomed_in(
    gameplay: &mut GameplayUiState,
    board: &BoardView,
    zoom_origin_x: u32,
    zoom_origin_y: u32,
) {
    if !gameplay_can_zoom_in(gameplay, board) {
        return;
    }

    gameplay.viewport.zoomed_in = true;
    let (clamped_origin_x, clamped_origin_y) =
        clamp_zoom_origin(gameplay, board, zoom_origin_x, zoom_origin_y);
    gameplay.viewport.zoom_origin_x = clamped_origin_x;
    gameplay.viewport.zoom_origin_y = clamped_origin_y;
}

pub fn set_gameplay_zoomed_out(gameplay: &mut GameplayUiState) {
    gameplay.viewport.zoomed_in = false;
    gameplay.viewport.zoom_origin_x = 0;
    gameplay.viewport.zoom_origin_y = 0;
}

pub fn pan_gameplay_zoom_by_swipe(
    gameplay: &mut GameplayUiState,
    board: &BoardView,
    delta_x: i32,
    delta_y: i32,
) {
    if !gameplay.viewport.zoomed_in {
        return;
    }

    let (visible_cols, visible_rows, _) = zoom_window_dimensions(gameplay, board);
    let bounds = non_void_bounds(board).unwrap_or_else(|| full_board_bounds(board));
    let left_bound = leftmost_useful_origin(board.width(), visible_cols, bounds.min_x);
    let right_bound = rightmost_useful_origin_for_max(
        board.width(),
        visible_cols,
        bounds.max_x.saturating_add(1),
    );
    let top_bound = leftmost_useful_origin(board.height(), visible_rows, bounds.min_y);
    let bottom_bound = rightmost_useful_origin_for_max(
        board.height(),
        visible_rows,
        bounds.max_y.saturating_add(1),
    );

    let shift_x = swipe_pan_axis_step(
        delta_x,
        gameplay.surface_width.max(1),
        gameplay.viewport.zoom_origin_x,
        left_bound,
        right_bound,
    );
    let shift_y = swipe_pan_axis_step(
        delta_y,
        gameplay
            .surface_height
            .saturating_sub(BOARD_VERTICAL_MARGIN)
            .max(1),
        gameplay.viewport.zoom_origin_y,
        top_bound,
        bottom_bound,
    );
    let zoom_origin_x = gameplay
        .viewport
        .zoom_origin_x
        .saturating_add_signed(-shift_x);
    let zoom_origin_y = gameplay
        .viewport
        .zoom_origin_y
        .saturating_add_signed(-shift_y);
    let (clamped_origin_x, clamped_origin_y) =
        clamp_zoom_origin(gameplay, board, zoom_origin_x, zoom_origin_y);
    gameplay.viewport.zoom_origin_x = clamped_origin_x;
    gameplay.viewport.zoom_origin_y = clamped_origin_y;
}

pub fn gameplay_zoom_origin_for_focus(
    gameplay: &GameplayUiState,
    board: &BoardView,
    focus: BoardCell,
) -> (u32, u32) {
    let (visible_cols, visible_rows, _) = zoom_window_dimensions(gameplay, board);
    let bounds = non_void_bounds(board).unwrap_or_else(|| full_board_bounds(board));
    (
        zoom_origin_for_focus_axis(
            focus.x,
            visible_cols,
            board.width(),
            bounds.min_x,
            bounds.max_x.saturating_add(1),
        ),
        zoom_origin_for_focus_axis(
            focus.y,
            visible_rows,
            board.height(),
            bounds.min_y,
            bounds.max_y.saturating_add(1),
        ),
    )
}

pub fn build_gameplay_visible_window(
    gameplay: &GameplayUiState,
    board: &BoardView,
) -> GameplayVisibleBoardWindow {
    if !gameplay.viewport.zoomed_in {
        return GameplayVisibleBoardWindow {
            board: board.clone(),
            viewport: fit_board_viewport_for_controls_capped(
                gameplay.surface_width,
                gameplay.surface_height,
                board,
                gameplay.max_cell_size,
            ),
            board_origin_x: 0,
            board_origin_y: 0,
        };
    }

    let (visible_cols, visible_rows, target_cell_size) = zoom_window_dimensions(gameplay, board);
    let bounds = non_void_bounds(board).unwrap_or_else(|| full_board_bounds(board));
    let (core_origin_x, core_origin_y) = clamp_zoom_origin(
        gameplay,
        board,
        gameplay.viewport.zoom_origin_x,
        gameplay.viewport.zoom_origin_y,
    );
    let x_axis = build_zoom_axis_window(
        core_origin_x,
        visible_cols,
        board.width(),
        bounds.min_x,
        bounds.max_x.saturating_add(1),
    );
    let y_axis = build_zoom_axis_window(
        core_origin_y,
        visible_rows,
        board.height(),
        bounds.min_y,
        bounds.max_y.saturating_add(1),
    );
    let board = crop_board(
        board,
        x_axis.crop_origin,
        y_axis.crop_origin,
        x_axis.crop_extent,
        y_axis.crop_extent,
    );
    let viewport = build_zoomed_gameplay_viewport(gameplay, target_cell_size, x_axis, y_axis);

    GameplayVisibleBoardWindow {
        board,
        viewport,
        board_origin_x: x_axis.crop_origin,
        board_origin_y: y_axis.crop_origin,
    }
}

fn zoom_window_dimensions(gameplay: &GameplayUiState, board: &BoardView) -> (u32, u32, u32) {
    let spec = zoom_spec(gameplay, board);
    (spec.visible_cols, spec.visible_rows, spec.target_cell_size)
}

fn gameplay_can_zoom_in(gameplay: &GameplayUiState, board: &BoardView) -> bool {
    let base_viewport = fit_board_viewport_for_controls_capped(
        gameplay.surface_width,
        gameplay.surface_height,
        board,
        gameplay.max_cell_size,
    );
    zoom_spec(gameplay, board).target_cell_size > base_viewport.cell_size
}

fn zoom_spec(gameplay: &GameplayUiState, board: &BoardView) -> ZoomSpec {
    let base_viewport = fit_board_viewport_for_controls_capped(
        gameplay.surface_width,
        gameplay.surface_height,
        board,
        gameplay.max_cell_size,
    );
    let target_cell_size = zoom_target_cell_size(gameplay, board, &base_viewport);
    let visible_cols = (gameplay.surface_width / target_cell_size)
        .max(1)
        .min(board.width().max(1));
    let available_height = gameplay
        .surface_height
        .saturating_sub(BOARD_VERTICAL_MARGIN);
    let visible_rows = (available_height / target_cell_size)
        .max(1)
        .min(board.height().max(1));

    ZoomSpec {
        target_cell_size,
        visible_cols,
        visible_rows,
    }
}

fn zoom_target_cell_size(
    gameplay: &GameplayUiState,
    board: &BoardView,
    base_viewport: &BoardViewport,
) -> u32 {
    if base_viewport.cell_size >= gameplay.max_cell_size {
        return base_viewport.cell_size;
    }

    let available_height = gameplay
        .surface_height
        .saturating_sub(BOARD_VERTICAL_MARGIN)
        .max(1);
    let width_fit_cell_size = gameplay.surface_width / board.width().max(1);
    let height_fit_cell_size = available_height / board.height().max(1);
    let (axis_extent_tiles, axis_window_px) = if width_fit_cell_size <= height_fit_cell_size {
        (board.width().max(1), gameplay.surface_width.max(1))
    } else {
        (board.height().max(1), available_height)
    };

    let target_visible_half_tiles = axis_extent_tiles
        .saturating_mul(2)
        .saturating_sub(ZOOM_AXIS_TILE_REDUCTION_HALF_TILES)
        .max(1);
    let target_cell_size = axis_window_px
        .saturating_mul(2)
        .checked_div(target_visible_half_tiles)
        .unwrap_or(1)
        .max(1)
        .min(gameplay.max_cell_size);

    target_cell_size.max(base_viewport.cell_size)
}

fn swipe_pan_axis_step(
    delta: i32,
    surface_extent: u32,
    current_origin: u32,
    min_origin: u32,
    max_origin: u32,
) -> i32 {
    if delta == 0 || surface_extent == 0 {
        return 0;
    }

    let toward_min = delta.is_positive();
    let available = if toward_min {
        current_origin.saturating_sub(min_origin)
    } else {
        max_origin.saturating_sub(current_origin)
    };
    if available == 0 {
        return 0;
    }

    let swipe_extent = delta.unsigned_abs().min(surface_extent);
    let magnitude = (available as u64 * swipe_extent as u64).div_ceil(surface_extent as u64);
    let magnitude = magnitude.max(1) as i32;
    if toward_min { magnitude } else { -magnitude }
}

fn clamp_zoom_origin(
    gameplay: &GameplayUiState,
    board: &BoardView,
    zoom_origin_x: u32,
    zoom_origin_y: u32,
) -> (u32, u32) {
    let (visible_cols, visible_rows, _) = zoom_window_dimensions(gameplay, board);
    let bounds = non_void_bounds(board).unwrap_or_else(|| full_board_bounds(board));
    (
        clamp_zoom_origin_axis(
            zoom_origin_x,
            visible_cols,
            board.width(),
            bounds.min_x,
            bounds.max_x.saturating_add(1),
        ),
        clamp_zoom_origin_axis(
            zoom_origin_y,
            visible_rows,
            board.height(),
            bounds.min_y,
            bounds.max_y.saturating_add(1),
        ),
    )
}

fn build_zoom_axis_window(
    core_origin: u32,
    core_extent: u32,
    board_extent: u32,
    content_min: u32,
    content_max_exclusive: u32,
) -> ZoomAxisWindow {
    let left_bound = leftmost_useful_origin(board_extent, core_extent, content_min);
    let right_bound =
        rightmost_useful_origin_for_max(board_extent, core_extent, content_max_exclusive);
    let content_extent = content_max_exclusive.saturating_sub(content_min);
    let has_leading_continuation = content_extent > core_extent && core_origin > left_bound;
    let has_trailing_continuation = content_extent > core_extent && core_origin < right_bound;

    ZoomAxisWindow {
        crop_origin: core_origin.saturating_sub(u32::from(has_leading_continuation)),
        crop_extent: core_extent
            .saturating_add(u32::from(has_leading_continuation))
            .saturating_add(u32::from(has_trailing_continuation)),
        core_extent,
        has_leading_continuation,
        has_trailing_continuation,
        at_leading_bound: core_origin == left_bound,
        at_trailing_bound: core_origin == right_bound,
    }
}

fn build_zoomed_gameplay_viewport(
    gameplay: &GameplayUiState,
    target_cell_size: u32,
    x_axis: ZoomAxisWindow,
    y_axis: ZoomAxisWindow,
) -> BoardViewport {
    let available_height = gameplay
        .surface_height
        .saturating_sub(BOARD_VERTICAL_MARGIN)
        .max(1);
    let cell_size = target_cell_size
        .min(fit_zoom_cell_size(
            gameplay.surface_width,
            x_axis.core_extent,
            x_axis.has_leading_continuation,
            x_axis.has_trailing_continuation,
        ))
        .min(fit_zoom_cell_size(
            available_height,
            y_axis.core_extent,
            y_axis.has_leading_continuation,
            y_axis.has_trailing_continuation,
        ))
        .max(1);

    let left_clip_px = continuation_clip_px(cell_size, x_axis.has_leading_continuation);
    let right_clip_px = continuation_clip_px(cell_size, x_axis.has_trailing_continuation);
    let top_clip_px = continuation_clip_px(cell_size, y_axis.has_leading_continuation);
    let bottom_clip_px = continuation_clip_px(cell_size, y_axis.has_trailing_continuation);

    let board_pixel_width = x_axis.crop_extent.saturating_mul(cell_size);
    let board_pixel_height = y_axis.crop_extent.saturating_mul(cell_size);
    let visible_width = board_pixel_width
        .saturating_sub(left_clip_px)
        .saturating_sub(right_clip_px);
    let visible_height = board_pixel_height
        .saturating_sub(top_clip_px)
        .saturating_sub(bottom_clip_px);

    let visible_left = aligned_zoom_visible_offset(
        gameplay.surface_width,
        visible_width,
        x_axis.at_leading_bound,
        x_axis.at_trailing_bound,
    );
    let visible_top = aligned_zoom_visible_offset(
        available_height,
        visible_height,
        y_axis.at_leading_bound,
        y_axis.at_trailing_bound,
    );

    BoardViewport {
        origin_x: visible_left - left_clip_px as i32,
        origin_y: BOARD_VERTICAL_MARGIN as i32 + visible_top - top_clip_px as i32,
        cell_size,
        board_pixel_width,
        board_pixel_height,
        outer_margin_tiles: 0,
    }
}

fn fit_zoom_cell_size(
    window_px: u32,
    core_extent: u32,
    has_leading_continuation: bool,
    has_trailing_continuation: bool,
) -> u32 {
    let visible_half_cells = core_extent
        .saturating_mul(2)
        .saturating_add(u32::from(has_leading_continuation))
        .saturating_add(u32::from(has_trailing_continuation))
        .max(1);
    window_px
        .saturating_mul(2)
        .checked_div(visible_half_cells)
        .unwrap_or(1)
        .max(1)
}

fn continuation_clip_px(cell_size: u32, has_continuation: bool) -> u32 {
    if has_continuation { cell_size / 2 } else { 0 }
}

fn aligned_zoom_visible_offset(
    surface_extent: u32,
    visible_extent: u32,
    at_leading_bound: bool,
    at_trailing_bound: bool,
) -> i32 {
    if at_leading_bound && at_trailing_bound {
        (surface_extent as i32 - visible_extent as i32) / 2
    } else if at_leading_bound {
        0
    } else if at_trailing_bound {
        surface_extent as i32 - visible_extent as i32
    } else {
        (surface_extent as i32 - visible_extent as i32) / 2
    }
}

fn zoom_origin_for_focus_axis(
    focus: u32,
    visible_extent: u32,
    board_extent: u32,
    content_min: u32,
    content_max_exclusive: u32,
) -> u32 {
    let full_max_origin = board_extent.saturating_sub(visible_extent);
    let desired_origin = focus
        .saturating_sub(visible_extent / 2)
        .min(full_max_origin);
    let content_extent = content_max_exclusive.saturating_sub(content_min);

    if content_extent <= visible_extent {
        let left_aligned = leftmost_useful_origin(board_extent, visible_extent, content_min);
        let right_aligned =
            rightmost_useful_origin_for_max(board_extent, visible_extent, content_max_exclusive);
        let midpoint_twice = content_min.saturating_add(content_max_exclusive);
        if focus.saturating_mul(2) >= midpoint_twice {
            right_aligned
        } else {
            left_aligned
        }
    } else {
        desired_origin.clamp(
            leftmost_useful_origin(board_extent, visible_extent, content_min),
            rightmost_useful_origin_for_max(board_extent, visible_extent, content_max_exclusive),
        )
    }
}

fn clamp_zoom_origin_axis(
    origin: u32,
    visible_extent: u32,
    board_extent: u32,
    content_min: u32,
    content_max_exclusive: u32,
) -> u32 {
    let full_max_origin = board_extent.saturating_sub(visible_extent);
    let desired_origin = origin.min(full_max_origin);
    let content_extent = content_max_exclusive.saturating_sub(content_min);
    let left_aligned = leftmost_useful_origin(board_extent, visible_extent, content_min);
    let right_aligned =
        rightmost_useful_origin_for_max(board_extent, visible_extent, content_max_exclusive);

    if content_extent <= visible_extent {
        if desired_origin.abs_diff(right_aligned) <= desired_origin.abs_diff(left_aligned) {
            right_aligned
        } else {
            left_aligned
        }
    } else {
        desired_origin.clamp(left_aligned, right_aligned)
    }
}

fn leftmost_useful_origin(board_extent: u32, visible_extent: u32, content_min: u32) -> u32 {
    content_min.min(board_extent.saturating_sub(visible_extent))
}

fn rightmost_useful_origin_for_max(
    board_extent: u32,
    visible_extent: u32,
    content_max_exclusive: u32,
) -> u32 {
    content_max_exclusive
        .saturating_sub(visible_extent)
        .min(board_extent.saturating_sub(visible_extent))
}

fn non_void_bounds(board: &BoardView) -> Option<NonVoidBounds> {
    let mut bounds: Option<NonVoidBounds> = None;
    for cell in board.cells() {
        if board.tile(cell) == TileKind::Void {
            continue;
        }
        bounds = Some(match bounds {
            Some(bounds) => NonVoidBounds {
                min_x: bounds.min_x.min(cell.x),
                min_y: bounds.min_y.min(cell.y),
                max_x: bounds.max_x.max(cell.x),
                max_y: bounds.max_y.max(cell.y),
            },
            None => NonVoidBounds {
                min_x: cell.x,
                min_y: cell.y,
                max_x: cell.x,
                max_y: cell.y,
            },
        });
    }
    bounds
}

fn full_board_bounds(board: &BoardView) -> NonVoidBounds {
    NonVoidBounds {
        min_x: 0,
        min_y: 0,
        max_x: board.width().saturating_sub(1),
        max_y: board.height().saturating_sub(1),
    }
}

fn crop_board(
    board: &BoardView,
    origin_x: u32,
    origin_y: u32,
    width: u32,
    height: u32,
) -> BoardView {
    let mut tiles = Vec::with_capacity((width * height) as usize);
    let mut boxes = Vec::with_capacity((width * height) as usize);
    let mut player = None;
    let mut selected_box = None;

    for local_y in 0..height {
        for local_x in 0..width {
            let world = BoardCell::new(origin_x + local_x, origin_y + local_y);
            let local = BoardCell::new(local_x, local_y);
            tiles.push(board.tile(world));
            boxes.push(board.has_box(world));
            if board.player() == Some(world) {
                player = Some(local);
            }
            if board.selected_box() == Some(world) {
                selected_box = Some(local);
            }
        }
    }

    BoardView::new(
        width,
        height,
        tiles,
        boxes,
        player,
        selected_box,
        board.is_solved(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_GAMEPLAY_MAX_CELL_SIZE, GameplayUiState, build_gameplay_visible_window,
        gameplay_zoom_origin_for_focus, pan_gameplay_zoom_by_swipe, set_gameplay_level_sets,
        set_gameplay_max_cell_size, set_gameplay_zoomed_in, zoom_window_dimensions,
    };
    use crate::persistence::{LevelSetCatalogEntry, LevelSetKind};
    use presentation::layout::BOARD_VERTICAL_MARGIN;
    use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};

    fn sample_level_sets() -> Vec<LevelSetCatalogEntry> {
        vec![
            LevelSetCatalogEntry {
                kind: LevelSetKind::Imported,
                title: "Set 1".to_string(),
                completed_puzzle_count: 0,
                total_puzzle_count: 10,
            },
            LevelSetCatalogEntry {
                kind: LevelSetKind::Imported,
                title: "Set 2".to_string(),
                completed_puzzle_count: 2,
                total_puzzle_count: 12,
            },
        ]
    }

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

    fn board_with_floor_rect(
        width: u32,
        height: u32,
        min_x: u32,
        min_y: u32,
        max_x: u32,
        max_y: u32,
    ) -> BoardView {
        let mut tiles = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
                    tiles.push(TileKind::Floor);
                } else {
                    tiles.push(TileKind::Void);
                }
            }
        }

        BoardView::new(
            width,
            height,
            tiles,
            vec![false; (width * height) as usize],
            None,
            None,
            false,
        )
    }

    #[test]
    fn set_gameplay_level_sets_keeps_valid_active_index() {
        let mut gameplay = GameplayUiState::default();

        set_gameplay_level_sets(&mut gameplay, sample_level_sets(), Some(1));

        assert_eq!(gameplay.active_level_set, Some(1));
        assert_eq!(gameplay.level_sets.len(), 2);
    }

    #[test]
    fn set_gameplay_level_sets_keeps_none_for_non_empty_catalog() {
        let mut gameplay = GameplayUiState::default();

        set_gameplay_level_sets(&mut gameplay, sample_level_sets(), None);

        assert_eq!(gameplay.active_level_set, None);
        assert_eq!(gameplay.level_sets.len(), 2);
    }

    #[test]
    fn set_gameplay_level_sets_drops_out_of_bounds_active_index() {
        let mut gameplay = GameplayUiState::default();

        set_gameplay_level_sets(&mut gameplay, sample_level_sets(), Some(99));

        assert_eq!(gameplay.active_level_set, None);
        assert_eq!(gameplay.level_sets.len(), 2);
    }

    #[test]
    fn gameplay_defaults_to_standard_max_cell_size() {
        let gameplay = GameplayUiState::default();

        assert_eq!(gameplay.max_cell_size, DEFAULT_GAMEPLAY_MAX_CELL_SIZE);
    }

    #[test]
    fn gameplay_viewport_respects_configured_max_cell_size() {
        let mut gameplay = GameplayUiState::default();
        let board = board_with_tile(1, 1, TileKind::Floor);
        set_gameplay_max_cell_size(&mut gameplay, 12);

        let viewport = build_gameplay_visible_window(&gameplay, &board).viewport;

        assert_eq!(viewport.cell_size, 12);
    }

    #[test]
    fn zoomed_window_only_reduces_width_limited_levels_by_about_two_and_a_half_tiles() {
        let mut gameplay = GameplayUiState {
            surface_width: 320,
            surface_height: 480,
            ..Default::default()
        };
        let board = board_with_tile(15, 8, TileKind::Floor);
        let normal = build_gameplay_visible_window(&gameplay, &board);
        let (visible_cols, _, _) = zoom_window_dimensions(&gameplay, &board);
        let (origin_x, origin_y) =
            gameplay_zoom_origin_for_focus(&gameplay, &board, BoardCell::new(7, 4));
        set_gameplay_zoomed_in(&mut gameplay, &board, origin_x, origin_y);

        let zoomed = build_gameplay_visible_window(&gameplay, &board);

        assert!(zoomed.viewport.cell_size > normal.viewport.cell_size);
        assert_eq!(visible_cols, 12);
    }

    #[test]
    fn zoomed_window_only_reduces_height_limited_levels_by_about_two_and_a_half_tiles() {
        let gameplay = GameplayUiState {
            surface_width: 320,
            surface_height: 480,
            ..Default::default()
        };
        let board = board_with_tile(8, 20, TileKind::Floor);

        let (_, visible_rows, _) = zoom_window_dimensions(&gameplay, &board);

        assert_eq!(visible_rows, 17);
    }

    #[test]
    fn zoom_in_is_noop_when_base_viewport_is_already_at_max_cell_size() {
        let mut gameplay = GameplayUiState {
            surface_width: 320,
            surface_height: 480,
            ..Default::default()
        };
        set_gameplay_max_cell_size(&mut gameplay, 40);
        let board = board_with_tile(4, 4, TileKind::Floor);

        set_gameplay_zoomed_in(&mut gameplay, &board, 1, 1);

        assert!(!gameplay.viewport.zoomed_in);
    }

    #[test]
    fn zoomed_window_crops_board_around_focus() {
        let mut gameplay = GameplayUiState {
            surface_width: 320,
            surface_height: 480,
            ..Default::default()
        };
        let board = board_with_tile(10, 12, TileKind::Floor);
        let (origin_x, origin_y) =
            gameplay_zoom_origin_for_focus(&gameplay, &board, BoardCell::new(7, 9));
        set_gameplay_zoomed_in(&mut gameplay, &board, origin_x, origin_y);

        let window = build_gameplay_visible_window(&gameplay, &board);

        assert!(window.board.width() < board.width() || window.board.height() < board.height());
        assert!(window.world_to_local_cell(BoardCell::new(7, 9)).is_some());
    }

    #[test]
    fn zoom_origin_clamps_to_board_bounds() {
        let mut gameplay = GameplayUiState {
            surface_width: 320,
            surface_height: 480,
            ..Default::default()
        };
        let board = board_with_tile(4, 4, TileKind::Floor);
        set_gameplay_zoomed_in(&mut gameplay, &board, 99, 99);

        let window = build_gameplay_visible_window(&gameplay, &board);

        assert!(window.board_origin_x <= board.width());
        assert!(window.board_origin_y <= board.height());
    }

    #[test]
    fn zoom_origin_biases_to_non_void_right_edge() {
        let gameplay = GameplayUiState {
            surface_width: 320,
            surface_height: 480,
            ..Default::default()
        };
        let board = board_with_floor_rect(16, 10, 4, 2, 14, 7);

        let (origin_x, _) =
            gameplay_zoom_origin_for_focus(&gameplay, &board, BoardCell::new(14, 4));

        assert_eq!(origin_x, 2);
    }

    #[test]
    fn zoomed_window_right_aligns_when_pinned_to_right_edge() {
        let mut gameplay = GameplayUiState {
            surface_width: 320,
            surface_height: 480,
            ..Default::default()
        };
        let board = board_with_floor_rect(12, 10, 4, 2, 8, 7);
        let (origin_x, origin_y) =
            gameplay_zoom_origin_for_focus(&gameplay, &board, BoardCell::new(8, 4));
        set_gameplay_zoomed_in(&mut gameplay, &board, origin_x, origin_y);

        let window = build_gameplay_visible_window(&gameplay, &board);

        assert_eq!(
            window.viewport.origin_x + window.viewport.board_pixel_width as i32,
            gameplay.surface_width as i32
        );
        assert_eq!(
            window
                .board
                .tile(BoardCell::new(window.board.width() - 1, 2)),
            TileKind::Floor
        );
    }

    #[test]
    fn zoomed_window_left_and_top_align_when_pinned_to_top_left() {
        let mut gameplay = GameplayUiState {
            surface_width: 320,
            surface_height: 480,
            ..Default::default()
        };
        let board = board_with_floor_rect(12, 10, 3, 2, 9, 8);
        let (origin_x, origin_y) =
            gameplay_zoom_origin_for_focus(&gameplay, &board, BoardCell::new(3, 2));
        set_gameplay_zoomed_in(&mut gameplay, &board, origin_x, origin_y);

        let window = build_gameplay_visible_window(&gameplay, &board);
        let available_height = gameplay.surface_height as i32 - BOARD_VERTICAL_MARGIN as i32;
        let expected_origin_y = BOARD_VERTICAL_MARGIN as i32
            + (available_height - window.viewport.board_pixel_height as i32) / 2;

        assert_eq!(window.viewport.origin_x, 0);
        assert_eq!(window.viewport.origin_y, expected_origin_y);
        assert_eq!(window.board.tile(BoardCell::new(0, 0)), TileKind::Void);
        assert_eq!(window.board_origin_x, 3);
        assert_eq!(window.board_origin_y, 0);
    }

    #[test]
    fn zoomed_window_keeps_horizontal_margins_when_full_width_still_fits() {
        let mut gameplay = GameplayUiState {
            surface_width: 320,
            surface_height: 480,
            ..Default::default()
        };
        let board = board_with_tile(8, 20, TileKind::Floor);
        let (origin_x, origin_y) =
            gameplay_zoom_origin_for_focus(&gameplay, &board, BoardCell::new(4, 10));
        set_gameplay_zoomed_in(&mut gameplay, &board, origin_x, origin_y);

        let window = build_gameplay_visible_window(&gameplay, &board);
        let expected_origin_x =
            (gameplay.surface_width as i32 - window.viewport.board_pixel_width as i32) / 2;

        assert_eq!(window.board.width(), board.width());
        assert_eq!(window.board_origin_x, 0);
        assert_eq!(window.viewport.origin_x, expected_origin_x);
    }

    #[test]
    fn pan_gameplay_zoom_by_swipe_distance_scales_with_swipe_length() {
        let mut short_swipe = GameplayUiState {
            surface_width: 320,
            surface_height: 480,
            ..Default::default()
        };
        let mut long_swipe = short_swipe.clone();
        let board = board_with_tile(16, 16, TileKind::Floor);
        set_gameplay_zoomed_in(&mut short_swipe, &board, 6, 6);
        set_gameplay_zoomed_in(&mut long_swipe, &board, 6, 6);

        pan_gameplay_zoom_by_swipe(&mut short_swipe, &board, 40, 0);
        pan_gameplay_zoom_by_swipe(&mut long_swipe, &board, 320, 0);

        assert!(short_swipe.viewport.zoom_origin_x < 6);
        assert!(long_swipe.viewport.zoom_origin_x < short_swipe.viewport.zoom_origin_x);
        assert_eq!(long_swipe.viewport.zoom_origin_x, 0);
    }
}
