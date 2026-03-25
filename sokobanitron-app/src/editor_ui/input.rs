//! App-owned editor interaction policy and top-level transitions.
//!
//! This module translates pointer input into editor commands plus app-level
//! overlay/screen changes. It does not own editor-domain mutation semantics and it does
//! not own rendering.

use crate::AppState;
use crate::overlay::is_editor_menu_open;
use crate::ui_state::{AppOverlay, AppScreen};
use renderer::{
    ControlsButtonAction, overlay_primary_action_button_contains, top_left_level_button_rect,
    top_menu_toggle_button_contains,
};
use sokobanitron_level_editor::{EditableTile, EditorCommand, EditorMode, LevelEditor};
use std::time::{Duration, Instant};

use super::paint_mode::PaintMode;
use super::view::{
    EditorUiState, LastTap, can_zoom_in, can_zoom_out, reset_editor_interaction_state,
    world_cell_at_screen_position, zoom_in, zoom_in_button_rect, zoom_out, zoom_out_button_rect,
};

const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(325);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorPointerPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

pub fn editor_cursor_moved(app_state: &mut AppState, editor: &mut LevelEditor, x: f64, y: f64) {
    let x = x.round() as i32;
    let y = y.round() as i32;
    app_state.editor.interaction.cursor_position = Some((x, y));
    if let Some(mode) = app_state.editor.interaction.mouse_paint_mode {
        continue_paint_stroke(&mut app_state.editor, editor, x as f64, y as f64, mode);
    }
}

pub fn editor_mouse_pressed(app_state: &mut AppState, editor: &mut LevelEditor, x: f64, y: f64) {
    app_state.editor.interaction.cursor_position = Some((x.round() as i32, y.round() as i32));
    if handle_menu_interaction(app_state, x, y) {
        return;
    }

    if top_menu_toggle_button_contains(x, y, app_state.editor.viewport.surface_width) {
        open_editor_menu(app_state);
        return;
    }

    app_state.editor.interaction.mouse_paint_mode =
        begin_paint_stroke(&mut app_state.editor, editor, x, y);
}

pub fn editor_mouse_released(app_state: &mut AppState) {
    app_state.editor.interaction.mouse_paint_mode = None;
}

pub fn editor_touch(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    id: u64,
    phase: EditorPointerPhase,
    x: f64,
    y: f64,
) {
    if is_editor_menu_open(app_state) {
        if matches!(phase, EditorPointerPhase::Started) {
            let _ = handle_menu_interaction(app_state, x, y);
        }
        return;
    }

    if matches!(phase, EditorPointerPhase::Started)
        && top_menu_toggle_button_contains(x, y, app_state.editor.viewport.surface_width)
    {
        open_editor_menu(app_state);
        return;
    }

    match phase {
        EditorPointerPhase::Started => {
            if app_state.editor.interaction.active_touch_paint.is_none() {
                app_state.editor.interaction.active_touch_paint =
                    begin_paint_stroke(&mut app_state.editor, editor, x, y).map(|mode| (id, mode));
            }
        }
        EditorPointerPhase::Moved => {
            if let Some((active_id, mode)) = app_state.editor.interaction.active_touch_paint
                && active_id == id
            {
                continue_paint_stroke(&mut app_state.editor, editor, x, y, mode);
            }
        }
        EditorPointerPhase::Ended | EditorPointerPhase::Cancelled => {
            if app_state
                .editor
                .interaction
                .active_touch_paint
                .is_some_and(|(active_id, _)| active_id == id)
            {
                app_state.editor.interaction.active_touch_paint = None;
            }
        }
    }
}

fn handle_menu_interaction(app_state: &mut AppState, x: f64, y: f64) -> bool {
    if !is_editor_menu_open(app_state) {
        return false;
    }

    if top_menu_toggle_button_contains(x, y, app_state.editor.viewport.surface_width) {
        close_editor_menu(app_state);
        return true;
    }

    if overlay_primary_action_button_contains(
        x,
        y,
        app_state.editor.viewport.surface_width,
        app_state.editor.viewport.surface_height,
    ) {
        leave_editor_for_gameplay(app_state);
        return true;
    }

    true
}

fn open_editor_menu(app_state: &mut AppState) {
    app_state.ui.overlay = Some(AppOverlay::EditorMenu);
    reset_editor_interaction_state(&mut app_state.editor);
}

fn close_editor_menu(app_state: &mut AppState) {
    app_state.ui.overlay = None;
    reset_editor_interaction_state(&mut app_state.editor);
}

fn leave_editor_for_gameplay(app_state: &mut AppState) {
    app_state.ui.screen = AppScreen::Gameplay;
    app_state.ui.overlay = None;
    reset_editor_interaction_state(&mut app_state.editor);
}

fn begin_paint_stroke(
    ui: &mut EditorUiState,
    editor: &mut LevelEditor,
    screen_x: f64,
    screen_y: f64,
) -> Option<PaintMode> {
    if top_left_level_button_rect().contains(screen_x, screen_y) {
        editor.apply_command(EditorCommand::ToggleMode);
        ui.interaction.last_tap = None;
        ui.interaction.mouse_paint_mode = None;
        ui.interaction.active_touch_paint = None;
        return None;
    }

    match editor.mode() {
        EditorMode::Draw => {
            if let Some(action) = zoom_button_action(ui, editor, screen_x, screen_y) {
                match action {
                    ZoomAction::ZoomIn => zoom_in(ui, editor),
                    ZoomAction::ZoomOut => zoom_out(ui),
                }
                return None;
            }
            let (world_x, world_y) = world_cell_at_screen_position(ui, editor, screen_x, screen_y)?;
            let mode = resolve_paint_mode(ui, editor, world_x, world_y);
            editor.apply_command(mode.to_command(world_x, world_y));
            Some(mode)
        }
        EditorMode::Manipulate => {
            if let Some(action) = manipulate_button_action(ui, editor, screen_x, screen_y) {
                match action {
                    ControlsButtonAction::Undo => {
                        editor.apply_command(EditorCommand::Undo);
                    }
                    ControlsButtonAction::Restart => {
                        editor.apply_command(EditorCommand::RestartToGoals);
                    }
                    ControlsButtonAction::ShowMenu => {}
                }
                return None;
            }
            let (world_x, world_y) = world_cell_at_screen_position(ui, editor, screen_x, screen_y)?;
            match editor.world().tile(world_x, world_y) {
                EditableTile::Box | EditableTile::BoxOnGoal => {
                    editor.apply_command(EditorCommand::SelectBox {
                        cell_x: world_x,
                        cell_y: world_y,
                    });
                }
                EditableTile::Void => {
                    editor.apply_command(EditorCommand::ClearSelection);
                }
                _ => {
                    editor.apply_command(EditorCommand::MoveSelectedBoxTo {
                        cell_x: world_x,
                        cell_y: world_y,
                    });
                }
            }
            None
        }
    }
}

fn continue_paint_stroke(
    ui: &mut EditorUiState,
    editor: &mut LevelEditor,
    screen_x: f64,
    screen_y: f64,
    mode: PaintMode,
) {
    if !matches!(editor.mode(), EditorMode::Draw) {
        return;
    }
    if let Some((world_x, world_y)) = world_cell_at_screen_position(ui, editor, screen_x, screen_y)
    {
        editor.apply_command(mode.to_command(world_x, world_y));
    }
}

fn resolve_paint_mode(
    ui: &mut EditorUiState,
    editor: &LevelEditor,
    world_x: i32,
    world_y: i32,
) -> PaintMode {
    let current_tile = editor.world().tile(world_x, world_y);
    if current_tile == EditableTile::BoxOnGoal {
        ui.interaction.last_tap = None;
        return PaintMode::SetVoid;
    }

    let now = Instant::now();
    let is_double_tap = ui.interaction.last_tap.is_some_and(|last| {
        last.world_x == world_x
            && last.world_y == world_y
            && now.duration_since(last.at) <= DOUBLE_TAP_WINDOW
    });

    if is_double_tap {
        ui.interaction.last_tap = None;
        PaintMode::SetBoxOnGoal
    } else {
        ui.interaction.last_tap = Some(LastTap {
            world_x,
            world_y,
            at: now,
        });
        PaintMode::from_start_tile(current_tile)
    }
}

fn zoom_button_action(
    ui: &EditorUiState,
    editor: &LevelEditor,
    screen_x: f64,
    screen_y: f64,
) -> Option<ZoomAction> {
    if can_zoom_out(ui)
        && zoom_out_button_rect(ui.viewport.surface_height).contains(screen_x, screen_y)
    {
        return Some(ZoomAction::ZoomOut);
    }
    if can_zoom_in(ui, editor)
        && zoom_in_button_rect(ui.viewport.surface_width, ui.viewport.surface_height)
            .contains(screen_x, screen_y)
    {
        return Some(ZoomAction::ZoomIn);
    }
    None
}

fn manipulate_button_action(
    ui: &EditorUiState,
    editor: &LevelEditor,
    screen_x: f64,
    screen_y: f64,
) -> Option<ControlsButtonAction> {
    if editor.can_undo()
        && zoom_out_button_rect(ui.viewport.surface_height).contains(screen_x, screen_y)
    {
        return Some(ControlsButtonAction::Undo);
    }
    if editor.can_restart()
        && zoom_in_button_rect(ui.viewport.surface_width, ui.viewport.surface_height)
            .contains(screen_x, screen_y)
    {
        return Some(ControlsButtonAction::Restart);
    }
    None
}

#[derive(Clone, Copy)]
enum ZoomAction {
    ZoomIn,
    ZoomOut,
}
