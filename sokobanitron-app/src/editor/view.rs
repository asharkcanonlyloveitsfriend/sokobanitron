//! App-owned editor view state and layout math.
//!
//! This module deliberately stays on the app side of the boundary:
//! it shapes editor domain state into a visible window and viewport, but it does not
//! own editor commands, undo/history, or any drawing logic.

use presentation::layout::BoardViewport;
use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};
use sokobanitron_level_editor::{EditorMode, LevelEditor, NonVoidBounds};

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
    pub double_tap: DoubleTapTracker<EditorDoubleTapTarget>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EditorDoubleTapTarget {
    Draw(i32, i32),
    PlayNonPlayer(i32, i32),
    PlayPlayer(i32, i32),
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
    editor.interaction.cursor_position = None;
    editor.interaction.touch.reset();
    editor.interaction.active_stroke = None;
    editor.interaction.double_tap.clear();
}

pub fn reset_editor_view_state(editor: &mut EditorUiState) {
    editor.viewport.view_center_x = 0;
    editor.viewport.view_center_y = 0;
    editor.viewport.zoom_steps = 0;
    reset_editor_interaction_state(editor);
}

pub(crate) fn can_save_editor_puzzle(editor: &LevelEditor) -> bool {
    editor.can_save()
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

    let mut tiles = Vec::with_capacity((board_cols * board_rows) as usize);
    let mut boxes = Vec::with_capacity((board_cols * board_rows) as usize);
    let mut selected_box_local = None;
    let mut player_local = None;
    for y in 0..board_rows {
        for x in 0..board_cols {
            let world_x = world_origin_x + x as i32;
            let world_y = world_origin_y + y as i32;
            let tile = editor.view_tile(world_x, world_y);
            let has_box = editor.view_has_box(world_x, world_y);
            if editor.view_player() == Some((world_x, world_y)) {
                player_local = Some(BoardCell::new(x, y));
            }
            match tile {
                sokobanitron_level_editor::Tile::Void => tiles.push(TileKind::Void),
                sokobanitron_level_editor::Tile::Floor => tiles.push(TileKind::Floor),
                sokobanitron_level_editor::Tile::Goal => tiles.push(TileKind::Goal),
            }
            boxes.push(has_box);
            if has_box
                && editor.view_selected_box() == Some((world_x, world_y))
                && matches!(editor.mode(), EditorMode::Move | EditorMode::Play)
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

    let (center_x, center_y) = center_for_bounds(bounds);
    bounds_fit_with_center(bounds, center_x, center_y, target_cols, target_rows)
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

fn center_for_bounds(bounds: NonVoidBounds) -> (i32, i32) {
    (
        (bounds.min_x + bounds.max_x).div_euclid(2),
        (bounds.min_y + bounds.max_y).div_euclid(2),
    )
}

fn recenter_on_walkable_tiles(ui: &mut EditorUiState, editor: &LevelEditor) {
    if let Some(bounds) = editor.world().non_void_bounds() {
        let (center_x, center_y) = center_for_bounds(bounds);
        ui.viewport.view_center_x = center_x;
        ui.viewport.view_center_y = center_y;
    }
}

pub(crate) fn zoom_in(ui: &mut EditorUiState, editor: &LevelEditor) {
    if !can_zoom_in(ui, editor) {
        return;
    }

    ui.viewport.zoom_steps -= 1;
    recenter_on_walkable_tiles(ui, editor);
}

pub(crate) fn zoom_out(ui: &mut EditorUiState, editor: &LevelEditor) {
    if can_zoom_out(ui) {
        ui.viewport.zoom_steps += 1;
        recenter_on_walkable_tiles(ui, editor);
    }
}

#[cfg(test)]
mod tests {
    use super::{EditorUiState, build_visible_window, zoom_in, zoom_out};
    use sokobanitron_gameplay::BoardCell;
    use sokobanitron_level_editor::{DrawTool, EditorCommand, EditorMode, LevelEditor};

    fn editor_with_asymmetric_non_void_bounds() -> LevelEditor {
        let mut editor = LevelEditor::new();
        let existing_cells = editor
            .snapshot()
            .board
            .cells
            .iter()
            .map(|cell| (cell.world_x, cell.world_y))
            .collect::<Vec<_>>();

        for (cell_x, cell_y) in existing_cells {
            editor.apply_command(EditorCommand::PaintCell {
                cell_x,
                cell_y,
                tool: DrawTool::Void,
            });
        }

        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 10,
            cell_y: 4,
            tool: DrawTool::Floor,
        });
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 12,
            cell_y: 6,
            tool: DrawTool::Floor,
        });
        editor
    }

    #[test]
    fn move_mode_selected_box_does_not_hide_player() {
        let ui = EditorUiState::default();
        let mut editor = LevelEditor::new();
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 1,
            cell_y: 0,
            tool: DrawTool::GoalWithBox,
        });
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 1,
            cell_y: 0,
        });

        let visible = build_visible_window(&ui, &editor);
        let player_x = (0 - visible.world_origin_x) as u32;
        let player_y = (0 - visible.world_origin_y) as u32;
        let selected_x = (1 - visible.world_origin_x) as u32;
        let selected_y = (0 - visible.world_origin_y) as u32;

        assert_eq!(
            visible.board.player(),
            Some(BoardCell::new(player_x, player_y))
        );
        assert_eq!(
            visible.board.selected_box(),
            Some(BoardCell::new(selected_x, selected_y))
        );
    }

    #[test]
    fn zoom_in_recenters_on_non_void_bounds() {
        let mut ui = EditorUiState::default();
        ui.viewport.view_center_x = -20;
        ui.viewport.view_center_y = -20;
        let editor = editor_with_asymmetric_non_void_bounds();

        zoom_in(&mut ui, &editor);

        assert_eq!(ui.viewport.zoom_steps, -1);
        assert_eq!(ui.viewport.view_center_x, 11);
        assert_eq!(ui.viewport.view_center_y, 5);
    }

    #[test]
    fn zoom_out_recenters_on_non_void_bounds() {
        let mut ui = EditorUiState::default();
        ui.viewport.zoom_steps = -1;
        ui.viewport.view_center_x = -20;
        ui.viewport.view_center_y = -20;
        let editor = editor_with_asymmetric_non_void_bounds();

        zoom_out(&mut ui, &editor);

        assert_eq!(ui.viewport.zoom_steps, 0);
        assert_eq!(ui.viewport.view_center_x, 11);
        assert_eq!(ui.viewport.view_center_y, 5);
    }
}
