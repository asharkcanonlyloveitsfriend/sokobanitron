//! App-owned editor interaction policy and top-level transitions.
//!
//! This module translates pointer input into editor commands plus app-level
//! overlay/screen changes. It does not own editor-domain mutation semantics and it does
//! not own rendering.

use crate::app::state::{AppOverlay, AppScreen, AppState};
use crate::shared::{
    MOUSE_POINTER_ID, PointerContact, PointerEvent, PointerGesture, PointerId, PointerPhase,
};
use presentation::hit_test::{
    ControlsButtonAction, overlay_primary_action_button_contains, top_menu_toggle_button_contains,
};
use presentation::layout::top_left_level_button_rect;
use sokobanitron_level_editor::{EditableTile, EditorCommand, EditorMode, LevelEditor};
use std::time::{Duration, Instant};

use super::paint_mode::PaintMode;
use super::view::{
    ActiveEditorStroke, EditorUiState, can_zoom_in, can_zoom_out, reset_editor_interaction_state,
    world_cell_at_screen_position, zoom_in, zoom_in_button_rect, zoom_out, zoom_out_button_rect,
};

const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(325);

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

pub fn editor_mouse_pressed(app_state: &mut AppState, editor: &mut LevelEditor, x: f64, y: f64) {
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
    );
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
) {
    handle_editor_pointer_event(
        app_state,
        editor,
        PointerEvent::new(id, phase, x, y, Instant::now()),
    );
}

fn handle_editor_pointer_event(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    event: PointerEvent,
) {
    let Some(gesture) = app_state.editor.interaction.pointer.handle_event(event) else {
        return;
    };
    handle_editor_gesture(app_state, editor, gesture);
}

fn handle_editor_gesture(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    gesture: PointerGesture,
) {
    match gesture {
        PointerGesture::Started(contact) => {
            let target = classify_editor_hit_target(app_state, editor, contact);
            if app_state.is_editor_menu_open() {
                handle_editor_menu_target(app_state, target);
                return;
            }
            handle_editor_started_target(app_state, editor, contact, target);
        }
        PointerGesture::DragStarted(contact) | PointerGesture::DragMoved(contact) => {
            continue_editor_drag(app_state, editor, contact);
        }
        PointerGesture::Ended(contact) | PointerGesture::Cancelled(contact) => {
            clear_active_stroke(app_state, contact.id);
        }
        PointerGesture::Tap(tap) => {
            clear_active_stroke(app_state, tap.id);
        }
    }
}

fn handle_editor_menu_target(app_state: &mut AppState, target: EditorHitTarget) {
    match target {
        EditorHitTarget::TopMenuToggle => close_editor_menu(app_state),
        EditorHitTarget::OverlayPrimaryAction => leave_editor_for_gameplay(app_state),
        _ => {}
    }
}

fn handle_editor_started_target(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    contact: PointerContact,
    target: EditorHitTarget,
) {
    match target {
        EditorHitTarget::TopMenuToggle => open_editor_menu(app_state),
        EditorHitTarget::ModeToggle => {
            editor.apply_command(EditorCommand::ToggleMode);
            app_state.editor.interaction.double_tap.clear();
            app_state.editor.interaction.active_stroke = None;
        }
        EditorHitTarget::BottomLeftButton | EditorHitTarget::BottomRightButton => {
            apply_editor_corner_button(app_state, editor, target);
        }
        EditorHitTarget::BoardCell { world_x, world_y } => {
            begin_editor_board_interaction(app_state, editor, contact, world_x, world_y);
        }
        EditorHitTarget::OverlayPrimaryAction | EditorHitTarget::Background => {}
    }
}

fn apply_editor_corner_button(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    target: EditorHitTarget,
) {
    match editor.mode() {
        EditorMode::Draw => match resolve_zoom_action(&app_state.editor, editor, target) {
            Some(ZoomAction::ZoomIn) => zoom_in(&mut app_state.editor, editor),
            Some(ZoomAction::ZoomOut) => zoom_out(&mut app_state.editor),
            None => {}
        },
        EditorMode::Manipulate => match resolve_manipulate_action(editor, target) {
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

    let target = classify_editor_hit_target(app_state, editor, contact);
    if let EditorHitTarget::BoardCell { world_x, world_y } = target {
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

fn classify_editor_hit_target(
    app_state: &AppState,
    editor: &LevelEditor,
    contact: PointerContact,
) -> EditorHitTarget {
    let (screen_x, screen_y) = contact.position.as_f64();
    if top_left_level_button_rect().contains(screen_x, screen_y) {
        return EditorHitTarget::ModeToggle;
    }
    if top_menu_toggle_button_contains(screen_x, screen_y, app_state.editor.viewport.surface_width)
    {
        return EditorHitTarget::TopMenuToggle;
    }
    if overlay_primary_action_button_contains(
        screen_x,
        screen_y,
        app_state.editor.viewport.surface_width,
        app_state.editor.viewport.surface_height,
    ) {
        return EditorHitTarget::OverlayPrimaryAction;
    }
    if zoom_out_button_rect(app_state.editor.viewport.surface_height).contains(screen_x, screen_y) {
        return EditorHitTarget::BottomLeftButton;
    }
    if zoom_in_button_rect(
        app_state.editor.viewport.surface_width,
        app_state.editor.viewport.surface_height,
    )
    .contains(screen_x, screen_y)
    {
        return EditorHitTarget::BottomRightButton;
    }
    if let Some((world_x, world_y)) =
        world_cell_at_screen_position(&app_state.editor, editor, screen_x, screen_y)
    {
        return EditorHitTarget::BoardCell { world_x, world_y };
    }
    EditorHitTarget::Background
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
        return PaintMode::SetVoid;
    }

    if ui
        .interaction
        .double_tap
        .register_tap((world_x, world_y), at, DOUBLE_TAP_WINDOW)
    {
        PaintMode::SetBoxOnGoal
    } else {
        PaintMode::from_start_tile(current_tile)
    }
}

fn resolve_zoom_action(
    ui: &EditorUiState,
    editor: &LevelEditor,
    target: EditorHitTarget,
) -> Option<ZoomAction> {
    match target {
        EditorHitTarget::BottomLeftButton if can_zoom_out(ui) => Some(ZoomAction::ZoomOut),
        EditorHitTarget::BottomRightButton if can_zoom_in(ui, editor) => Some(ZoomAction::ZoomIn),
        _ => None,
    }
}

fn resolve_manipulate_action(
    editor: &LevelEditor,
    target: EditorHitTarget,
) -> Option<ControlsButtonAction> {
    match target {
        EditorHitTarget::BottomLeftButton if editor.can_undo() => Some(ControlsButtonAction::Undo),
        EditorHitTarget::BottomRightButton if editor.can_restart() => {
            Some(ControlsButtonAction::Restart)
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorHitTarget {
    ModeToggle,
    TopMenuToggle,
    OverlayPrimaryAction,
    BottomLeftButton,
    BottomRightButton,
    BoardCell { world_x: i32, world_y: i32 },
    Background,
}

#[derive(Clone, Copy)]
enum ZoomAction {
    ZoomIn,
    ZoomOut,
}
