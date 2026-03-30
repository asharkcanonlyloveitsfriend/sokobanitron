//! App-owned editor interaction policy and top-level transitions.
//!
//! This module translates pointer input into editor commands plus app-level
//! overlay/screen changes. It does not own editor-domain mutation semantics and it does
//! not own rendering.

use crate::app::state::{AppOverlay, AppScreen, AppState};
use crate::shared::{
    MOUSE_POINTER_ID, PointerContact, PointerEvent, PointerGesture, PointerId, PointerPhase,
};
use presentation::hit_test::ControlsButtonAction;
use sokobanitron_level_editor::{EditableTile, EditorCommand, EditorMode, LevelEditor};
use std::time::{Duration, Instant};

use super::hit_test::{
    EditorControlSlot, EditorSurfaceTarget, build_editor_surface_model, editor_surface_target_at,
};
use super::paint_mode::PaintMode;
use super::view::{
    ActiveEditorStroke, EditorUiState, can_zoom_in, can_zoom_out, reset_editor_interaction_state,
    zoom_in, zoom_out,
};

const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(325);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorUiAction {
    SavePuzzle,
}

pub fn editor_cursor_moved(app_state: &mut AppState, editor: &mut LevelEditor, x: f64, y: f64) {
    let x = x.round() as i32;
    let y = y.round() as i32;
    app_state.editor.interaction.cursor_position = Some((x, y));
    if app_state
        .editor
        .interaction
        .pointer
        .is_active_pointer(MOUSE_POINTER_ID)
    {
        handle_editor_pointer_event(
            app_state,
            editor,
            PointerEvent::new(
                MOUSE_POINTER_ID,
                PointerPhase::Moved,
                x as f64,
                y as f64,
                Instant::now(),
            ),
        );
    }
}

pub fn editor_mouse_pressed(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    x: f64,
    y: f64,
) -> Option<EditorUiAction> {
    app_state.editor.interaction.cursor_position = Some((x.round() as i32, y.round() as i32));
    handle_editor_pointer_event(
        app_state,
        editor,
        PointerEvent::new(
            MOUSE_POINTER_ID,
            PointerPhase::Started,
            x,
            y,
            Instant::now(),
        ),
    )
}

pub fn editor_mouse_released(app_state: &mut AppState) {
    let Some((x, y)) = app_state.editor.interaction.cursor_position.or_else(|| {
        app_state
            .editor
            .interaction
            .pointer
            .active_position()
            .map(|position| (position.x, position.y))
    }) else {
        app_state.editor.interaction.pointer.reset();
        app_state.editor.interaction.active_stroke = None;
        return;
    };
    let Some(gesture) = app_state
        .editor
        .interaction
        .pointer
        .handle_event(PointerEvent::new(
            MOUSE_POINTER_ID,
            PointerPhase::Ended,
            x as f64,
            y as f64,
            Instant::now(),
        ))
    else {
        return;
    };
    match gesture {
        PointerGesture::Ended(contact) | PointerGesture::Cancelled(contact) => {
            clear_active_stroke(app_state, contact.id);
        }
        PointerGesture::Tap(tap) => {
            clear_active_stroke(app_state, tap.id);
        }
        PointerGesture::Started(_)
        | PointerGesture::DragStarted(_)
        | PointerGesture::DragMoved(_) => {}
    }
}

pub fn editor_touch(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    id: u64,
    phase: PointerPhase,
    x: f64,
    y: f64,
) -> Option<EditorUiAction> {
    handle_editor_pointer_event(
        app_state,
        editor,
        PointerEvent::new(id, phase, x, y, Instant::now()),
    )
}

fn handle_editor_pointer_event(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    event: PointerEvent,
) -> Option<EditorUiAction> {
    let gesture = app_state.editor.interaction.pointer.handle_event(event)?;
    handle_editor_gesture(app_state, editor, gesture)
}

fn handle_editor_gesture(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    gesture: PointerGesture,
) -> Option<EditorUiAction> {
    match gesture {
        PointerGesture::Started(contact) => {
            let surface = build_editor_surface_model(app_state, editor);
            let (screen_x, screen_y) = contact.position.as_f64();
            let target = editor_surface_target_at(&surface, screen_x, screen_y);
            if app_state.is_editor_menu_open() {
                return handle_editor_menu_target(app_state, target);
            }
            handle_editor_started_target(app_state, editor, contact, target);
            None
        }
        PointerGesture::DragStarted(contact) | PointerGesture::DragMoved(contact) => {
            continue_editor_drag(app_state, editor, contact);
            None
        }
        PointerGesture::Ended(contact) | PointerGesture::Cancelled(contact) => {
            clear_active_stroke(app_state, contact.id);
            None
        }
        PointerGesture::Tap(tap) => {
            clear_active_stroke(app_state, tap.id);
            None
        }
    }
}

fn handle_editor_menu_target(
    app_state: &mut AppState,
    target: Option<EditorSurfaceTarget>,
) -> Option<EditorUiAction> {
    match target {
        Some(EditorSurfaceTarget::TopMenuToggle) => {
            close_editor_menu(app_state);
            None
        }
        Some(EditorSurfaceTarget::OverlayPrimaryAction) => {
            leave_editor_for_gameplay(app_state);
            None
        }
        Some(EditorSurfaceTarget::OverlaySecondaryAction) => {
            close_editor_menu(app_state);
            Some(EditorUiAction::SavePuzzle)
        }
        _ => None,
    }
}

fn handle_editor_started_target(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    contact: PointerContact,
    target: Option<EditorSurfaceTarget>,
) {
    match target {
        Some(EditorSurfaceTarget::TopMenuToggle) => open_editor_menu(app_state),
        Some(EditorSurfaceTarget::ModeToggle) => {
            editor.apply_command(EditorCommand::ToggleMode);
            app_state.editor.interaction.double_tap.clear();
            app_state.editor.interaction.active_stroke = None;
        }
        Some(EditorSurfaceTarget::ControlSlot(slot)) => {
            apply_editor_control_slot(app_state, editor, slot);
        }
        Some(EditorSurfaceTarget::BoardCell { world_x, world_y }) => {
            begin_editor_board_interaction(app_state, editor, contact, world_x, world_y);
        }
        Some(EditorSurfaceTarget::OverlayPrimaryAction)
        | Some(EditorSurfaceTarget::OverlaySecondaryAction)
        | None => {}
    }
}

fn apply_editor_control_slot(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    slot: EditorControlSlot,
) {
    match editor.mode() {
        EditorMode::Draw => match resolve_zoom_action(&app_state.editor, editor, slot) {
            Some(ZoomAction::ZoomIn) => zoom_in(&mut app_state.editor, editor),
            Some(ZoomAction::ZoomOut) => zoom_out(&mut app_state.editor),
            None => {}
        },
        EditorMode::Manipulate => match resolve_manipulate_action(editor, slot) {
            Some(ControlsButtonAction::Undo) => {
                editor.apply_command(EditorCommand::Undo);
            }
            Some(ControlsButtonAction::Restart) => {
                editor.apply_command(EditorCommand::RestartToGoals);
            }
            _ => {}
        },
    }
}

fn begin_editor_board_interaction(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    contact: PointerContact,
    world_x: i32,
    world_y: i32,
) {
    match editor.mode() {
        EditorMode::Draw => {
            let mode =
                resolve_paint_mode(&mut app_state.editor, editor, world_x, world_y, contact.at);
            editor.apply_command(mode.to_command(world_x, world_y));
            app_state.editor.interaction.active_stroke = Some(ActiveEditorStroke {
                pointer_id: contact.id,
                mode,
            });
        }
        EditorMode::Manipulate => match editor.world().tile(world_x, world_y) {
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
        },
    }
}

fn continue_editor_drag(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    contact: PointerContact,
) {
    if !matches!(editor.mode(), EditorMode::Draw) {
        return;
    }
    let Some(active) = app_state.editor.interaction.active_stroke else {
        return;
    };
    if active.pointer_id != contact.id {
        return;
    }

    let surface = build_editor_surface_model(app_state, editor);
    let (screen_x, screen_y) = contact.position.as_f64();
    let target = editor_surface_target_at(&surface, screen_x, screen_y);
    if let Some(EditorSurfaceTarget::BoardCell { world_x, world_y }) = target {
        editor.apply_command(active.mode.to_command(world_x, world_y));
    }
}

fn clear_active_stroke(app_state: &mut AppState, pointer_id: PointerId) {
    if app_state
        .editor
        .interaction
        .active_stroke
        .is_some_and(|active| active.pointer_id == pointer_id)
    {
        app_state.editor.interaction.active_stroke = None;
    }
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

fn resolve_paint_mode(
    ui: &mut EditorUiState,
    editor: &LevelEditor,
    world_x: i32,
    world_y: i32,
    at: Instant,
) -> PaintMode {
    let current_tile = editor.world().tile(world_x, world_y);
    if current_tile == EditableTile::BoxOnGoal {
        ui.interaction.double_tap.clear();
        return PaintMode::Void;
    }

    if ui
        .interaction
        .double_tap
        .register_tap((world_x, world_y), at, DOUBLE_TAP_WINDOW)
    {
        PaintMode::BoxOnGoal
    } else {
        PaintMode::from_start_tile(current_tile)
    }
}

fn resolve_zoom_action(
    ui: &EditorUiState,
    editor: &LevelEditor,
    slot: EditorControlSlot,
) -> Option<ZoomAction> {
    match slot {
        EditorControlSlot::BottomLeft if can_zoom_out(ui) => Some(ZoomAction::ZoomOut),
        EditorControlSlot::BottomRight if can_zoom_in(ui, editor) => Some(ZoomAction::ZoomIn),
        _ => None,
    }
}

fn resolve_manipulate_action(
    editor: &LevelEditor,
    slot: EditorControlSlot,
) -> Option<ControlsButtonAction> {
    match slot {
        EditorControlSlot::BottomLeft if editor.can_undo() => Some(ControlsButtonAction::Undo),
        EditorControlSlot::BottomRight if editor.can_restart() => {
            Some(ControlsButtonAction::Restart)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum ZoomAction {
    ZoomIn,
    ZoomOut,
}

#[cfg(test)]
mod tests {
    use super::{EditorUiAction, editor_mouse_pressed};
    use crate::app::state::{AppOverlay, AppScreen, AppState};
    use presentation::layout::overlay_secondary_action_button_rect;
    use sokobanitron_level_editor::{DrawTool, EditorCommand, EditorMode, LevelEditor};

    #[test]
    fn save_button_returns_action_and_closes_editor_menu() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        app_state.ui.overlay = Some(AppOverlay::EditorMenu);
        let mut editor = LevelEditor::new();
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 2,
            cell_y: 0,
            tool: DrawTool::Floor,
        });
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 0,
            cell_y: 0,
            tool: DrawTool::BoxOnGoal,
        });
        editor.apply_command(EditorCommand::SetMode(EditorMode::Manipulate));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });
        let rect = overlay_secondary_action_button_rect(
            app_state.editor.viewport.surface_width,
            app_state.editor.viewport.surface_height,
        );

        let action = editor_mouse_pressed(
            &mut app_state,
            &mut editor,
            (rect.x + rect.w / 2) as f64,
            (rect.y + rect.h / 2) as f64,
        );

        assert_eq!(action, Some(EditorUiAction::SavePuzzle));
        assert!(!app_state.is_editor_menu_open());
    }
}
