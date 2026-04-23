//! App-owned editor view state and layout math.
//!
//! This module deliberately stays on the app side of the boundary:
//! it shapes editor domain state into a visible window and viewport, but it does not
//! own editor commands, undo/history, or any drawing logic.

use presentation::layout::BoardViewport;
use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};
use sokobanitron_level_editor::{EditorMode, LevelEditor, NonVoidBounds, Tile};

use crate::shared::{DoubleTapTracker, PointerId, TouchPointerState, TouchStartPolicy};
use std::time::Duration;

use super::paint_mode::PaintMode;

const GRID_MARGIN_TILES: u32 = 1;
const BASE_VISIBLE_COLS: u32 = 9;
const MIN_VISIBLE_COLS: u32 = 5;
const MAX_VISIBLE_COLS: u32 = 25;
const DEFAULT_EDITOR_WIDTH: u32 = 670;
const DEFAULT_EDITOR_HEIGHT: u32 = 891;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorUiState {
    pub viewport: EditorViewportState,
    pub(crate) interaction: EditorInteractionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorViewportState {
    pub surface_width: u32,
    pub surface_height: u32,
    pub view_center_x: i32,
    pub view_center_y: i32,
    pub zoom_steps: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditorInteractionState {
    pub cursor_position: Option<(i32, i32)>,
    pub touch: TouchPointerState,
    pub active_stroke: Option<ActiveEditorStroke>,
    pub double_tap: DoubleTapTracker<(i32, i32)>,
    pub double_tap_window: Duration,
}

impl Default for EditorInteractionState {
    fn default() -> Self {
        Self {
            cursor_position: None,
            touch: TouchPointerState::with_touch_start_policy(TouchStartPolicy::Deferred),
            active_stroke: None,
            double_tap: DoubleTapTracker::default(),
            double_tap_window: Duration::from_millis(325),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActiveEditorStroke {
    pub pointer_id: PointerId,
    pub mode: PaintMode,
}

impl Default for EditorUiState {
    fn default() -> Self {
        Self {
            viewport: EditorViewportState {
                surface_width: DEFAULT_EDITOR_WIDTH,
                surface_height: DEFAULT_EDITOR_HEIGHT,
                view_center_x: 0,
                view_center_y: 0,
                zoom_steps: 0,
            },
            interaction: EditorInteractionState::default(),
        }
    }
}

pub fn resize_editor_surface(editor: &mut EditorUiState, width: u32, height: u32) {
    editor.viewport.surface_width = width.max(1);
    editor.viewport.surface_height = height.max(1);
    editor.viewport.zoom_steps = clamp_zoom_steps(editor.viewport.zoom_steps);
}

pub fn set_editor_touch_slop(editor: &mut EditorUiState, tap_slop_px: i32) {
    editor.interaction.touch.set_tap_slop(tap_slop_px);
}

pub fn set_editor_double_tap_window(editor: &mut EditorUiState, window: Duration) {
    editor.interaction.double_tap_window = window;
}

pub fn reset_editor_interaction_state(editor: &mut EditorUiState) {
    editor.interaction.touch.reset();
    editor.interaction.active_stroke = None;
}

pub(crate) fn can_save_editor_puzzle(editor: &LevelEditor) -> bool {
    editor
        .world()
        .box_positions()
        .into_iter()
        .any(|(x, y)| editor.world().tile(x, y) == Tile::Floor)
}

#[derive(Debug)]
pub(crate) struct VisibleBoardWindow {
    pub board: BoardView,
    pub viewport: BoardViewport,
    pub world_origin_x: i32,
    pub world_origin_y: i32,
}

impl VisibleBoardWindow {
    pub(crate) fn screen_to_world_cell(&self, screen_x: f64, screen_y: f64) -> Option<(i32, i32)> {
        self.viewport
            .screen_to_cell(screen_x, screen_y, &self.board)
            .map(|cell| {
                (
                    self.world_origin_x + cell.x as i32,
                    self.world_origin_y + cell.y as i32,
                )
            })
    }
}

pub(crate) fn build_visible_window(ui: &EditorUiState, editor: &LevelEditor) -> VisibleBoardWindow {
    let (board_cols, board_rows) = board_dimensions_for_steps(
        ui.viewport.zoom_steps,
        ui.viewport.surface_width,
        ui.viewport.surface_height,
    );
    let cell_size = compute_cell_size(ui.viewport.surface_width, board_cols);
    let world_origin_x = ui.viewport.view_center_x - (board_cols as i32 / 2);
    let world_origin_y = ui.viewport.view_center_y - (board_rows as i32 / 2);
    let hide_player = editor.selected_box().is_some();

    let mut tiles = Vec::with_capacity((board_cols * board_rows) as usize);
    let mut boxes = Vec::with_capacity((board_cols * board_rows) as usize);
    let mut selected_box_local = None;
    let mut player_local = None;
    for y in 0..board_rows {
        for x in 0..board_cols {
            let world_x = world_origin_x + x as i32;
            let world_y = world_origin_y + y as i32;
            let tile = editor.world().tile(world_x, world_y);
            let has_box = editor.world().has_box(world_x, world_y);
            if !hide_player && editor.world().player() == Some((world_x, world_y)) {
                player_local = Some(BoardCell::new(x, y));
            }
            match tile {
                sokobanitron_level_editor::Tile::Void => tiles.push(TileKind::Void),
                sokobanitron_level_editor::Tile::Floor => tiles.push(TileKind::Floor),
                sokobanitron_level_editor::Tile::Goal => tiles.push(TileKind::Goal),
            }
            boxes.push(has_box);
            if has_box
                && editor.selected_box() == Some((world_x, world_y))
                && matches!(editor.mode(), EditorMode::Move)
            {
                selected_box_local = Some(BoardCell::new(x, y));
            }
        }
    }
    let board = BoardView::new(
        board_cols,
        board_rows,
        tiles,
        boxes,
        player_local,
        selected_box_local,
        false,
    );

    let board_pixel_width = (board_cols + GRID_MARGIN_TILES.saturating_mul(2)) * cell_size;
    let board_pixel_height = (board_rows + GRID_MARGIN_TILES.saturating_mul(2)) * cell_size;
    let viewport = BoardViewport {
        origin_x: ((ui.viewport.surface_width as i32) - (board_pixel_width as i32)) / 2,
        origin_y: ((ui.viewport.surface_height as i32) - (board_pixel_height as i32)) / 2,
        cell_size,
        board_pixel_width,
        board_pixel_height,
        outer_margin_tiles: GRID_MARGIN_TILES,
    };

    VisibleBoardWindow {
        board,
        viewport,
        world_origin_x,
        world_origin_y,
    }
}

fn board_dimensions_for_steps(
    zoom_steps: i32,
    surface_width: u32,
    surface_height: u32,
) -> (u32, u32) {
    let steps = clamp_zoom_steps(zoom_steps);
    let cols_i32 = (BASE_VISIBLE_COLS as i32 + steps * 2).max(1);
    let cols = cols_i32 as u32;
    let cell_size = compute_cell_size(surface_width, cols);
    let rows = compute_visible_rows(surface_height, cell_size);
    (cols, rows)
}

fn compute_cell_size(surface_width: u32, board_cols: u32) -> u32 {
    let cols_with_margins = board_cols + GRID_MARGIN_TILES.saturating_mul(2);
    (surface_width / cols_with_margins).max(1)
}

fn compute_visible_rows(surface_height: u32, cell_size: u32) -> u32 {
    let total_rows = (surface_height / cell_size).max(1);
    let mut board_rows = total_rows
        .saturating_sub(GRID_MARGIN_TILES.saturating_mul(2))
        .max(1);
    if board_rows.is_multiple_of(2) {
        board_rows = board_rows.saturating_sub(1).max(1);
    }
    board_rows
}

fn min_zoom_in_steps() -> i32 {
    -((BASE_VISIBLE_COLS.saturating_sub(MIN_VISIBLE_COLS) / 2) as i32)
}

fn max_zoom_out_steps() -> i32 {
    (MAX_VISIBLE_COLS.saturating_sub(BASE_VISIBLE_COLS) / 2) as i32
}

fn clamp_zoom_steps(steps: i32) -> i32 {
    steps.clamp(min_zoom_in_steps(), max_zoom_out_steps())
}

pub(crate) fn can_zoom_out(ui: &EditorUiState) -> bool {
    ui.viewport.zoom_steps < max_zoom_out_steps()
}

pub(crate) fn can_zoom_in(ui: &EditorUiState, editor: &LevelEditor) -> bool {
    if ui.viewport.zoom_steps <= min_zoom_in_steps() {
        return false;
    }

    let target_steps = ui.viewport.zoom_steps - 1;
    let (target_cols, target_rows) = board_dimensions_for_steps(
        target_steps,
        ui.viewport.surface_width,
        ui.viewport.surface_height,
    );
    let Some(bounds) = editor.world().non_void_bounds() else {
        return true;
    };

    if bounds_fit_with_center(
        bounds,
        ui.viewport.view_center_x,
        ui.viewport.view_center_y,
        target_cols,
        target_rows,
    ) {
        return true;
    }

    centered_view_for_bounds(
        bounds,
        ui.viewport.view_center_x,
        ui.viewport.view_center_y,
        target_cols,
        target_rows,
    )
    .is_some()
}

fn bounds_fit_with_center(
    bounds: NonVoidBounds,
    center_x: i32,
    center_y: i32,
    cols: u32,
    rows: u32,
) -> bool {
    let origin_x = center_x - cols as i32 / 2;
    let origin_y = center_y - rows as i32 / 2;
    let max_x = origin_x + cols as i32 - 1;
    let max_y = origin_y + rows as i32 - 1;
    bounds.min_x >= origin_x
        && bounds.max_x <= max_x
        && bounds.min_y >= origin_y
        && bounds.max_y <= max_y
}

fn centered_view_for_bounds(
    bounds: NonVoidBounds,
    view_center_x: i32,
    view_center_y: i32,
    cols: u32,
    rows: u32,
) -> Option<(i32, i32)> {
    let half_cols = cols as i32 / 2;
    let half_rows = rows as i32 / 2;

    let min_center_x = bounds.max_x - half_cols;
    let max_center_x = bounds.min_x + half_cols;
    if min_center_x > max_center_x {
        return None;
    }

    let min_center_y = bounds.max_y - half_rows;
    let max_center_y = bounds.min_y + half_rows;
    if min_center_y > max_center_y {
        return None;
    }

    Some((
        view_center_x.clamp(min_center_x, max_center_x),
        view_center_y.clamp(min_center_y, max_center_y),
    ))
}

pub(crate) fn zoom_in(ui: &mut EditorUiState, editor: &LevelEditor) {
    if !can_zoom_in(ui, editor) {
        return;
    }

    let target_steps = ui.viewport.zoom_steps - 1;
    let (target_cols, target_rows) = board_dimensions_for_steps(
        target_steps,
        ui.viewport.surface_width,
        ui.viewport.surface_height,
    );
    if let Some(bounds) = editor.world().non_void_bounds() {
        if bounds_fit_with_center(
            bounds,
            ui.viewport.view_center_x,
            ui.viewport.view_center_y,
            target_cols,
            target_rows,
        ) {
            ui.viewport.zoom_steps = target_steps;
        } else if let Some((center_x, center_y)) = centered_view_for_bounds(
            bounds,
            ui.viewport.view_center_x,
            ui.viewport.view_center_y,
            target_cols,
            target_rows,
        ) {
            ui.viewport.view_center_x = center_x;
            ui.viewport.view_center_y = center_y;
            ui.viewport.zoom_steps = target_steps;
        }
    } else {
        ui.viewport.zoom_steps = target_steps;
    }
}

pub(crate) fn zoom_out(ui: &mut EditorUiState) {
    if can_zoom_out(ui) {
        ui.viewport.zoom_steps += 1;
    }
}
