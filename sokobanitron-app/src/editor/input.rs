//! App-owned editor interaction policy and top-level transitions.
//!
//! This module translates pointer input into editor commands plus app-level
//! overlay/screen changes. It does not own editor-domain mutation semantics and it does
//! not own rendering.

use crate::app::state::{AppOverlay, AppScreen, AppState};
use crate::shared::{
    MOUSE_POINTER_ID, PinchDirection, PinchGesture, PointerContact, PointerEvent, PointerGesture,
    PointerId, PointerPhase, TouchGestureUpdate,
};
use sokobanitron_level_editor::{EditorCommand, EditorMode, LevelEditor, Tile};
use std::time::Instant;

use super::hit_test::{
    EditorControlSlot, EditorSurfaceTarget, build_editor_surface_model, editor_surface_target_at,
};
use super::is_selectable_move_mode_box;
use super::paint_mode::PaintMode;
use super::view::{
    ActiveEditorStroke, EditorDoubleTapTarget, EditorUiState, reset_editor_interaction_state,
    zoom_in, zoom_out,
};

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
        .touch
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
            .touch
            .active_position()
            .map(|position| (position.x, position.y))
    }) else {
        app_state.editor.interaction.touch.reset();
        app_state.editor.interaction.active_stroke = None;
        return;
    };
    let Some(gesture) = app_state
        .editor
        .interaction
        .touch
        .handle_pointer_event(PointerEvent::new(
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
    let event = PointerEvent::new(id, phase, x, y, Instant::now());
    let touch_update = app_state.editor.interaction.touch.handle_touch_event(event);
    if touch_update.reset_screen_state {
        reset_editor_screen_touch_state(&mut app_state.editor);
    }
    if let Some(pinch) = touch_update.pinch {
        return handle_editor_pinch(app_state, editor, pinch);
    }
    if touch_update.suppress_screen_gestures {
        app_state.editor.interaction.double_tap.clear();
        return None;
    }

    handle_editor_touch_update(app_state, editor, touch_update)
}

fn handle_editor_pointer_event(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    event: PointerEvent,
) -> Option<EditorUiAction> {
    let gesture = app_state
        .editor
        .interaction
        .touch
        .handle_pointer_event(event)?;
    handle_editor_gesture(app_state, editor, gesture)
}

fn handle_editor_touch_update(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    update: TouchGestureUpdate,
) -> Option<EditorUiAction> {
    let gesture = update.gesture?;
    handle_editor_touch_gesture(app_state, editor, gesture, update.deferred_start)
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

fn handle_editor_touch_gesture(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    gesture: PointerGesture,
    deferred_start: Option<PointerContact>,
) -> Option<EditorUiAction> {
    match gesture {
        PointerGesture::Started(_) => None,
        PointerGesture::DragStarted(contact) | PointerGesture::DragMoved(contact) => {
            begin_touch_drag_if_needed(app_state, editor, deferred_start);
            continue_editor_drag(app_state, editor, contact);
            None
        }
        PointerGesture::Ended(contact) | PointerGesture::Cancelled(contact) => {
            clear_active_stroke(app_state, contact.id);
            None
        }
        PointerGesture::Tap(tap) => handle_editor_touch_tap(
            app_state,
            editor,
            deferred_start,
            PointerContact {
                id: tap.id,
                position: tap.position,
                at: tap.at,
            },
        ),
    }
}

fn handle_editor_pinch(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    pinch: PinchGesture,
) -> Option<EditorUiAction> {
    app_state.editor.interaction.double_tap.clear();
    if app_state.is_editor_menu_open() {
        return None;
    }

    match pinch.direction {
        PinchDirection::Out => zoom_in(&mut app_state.editor, editor),
        PinchDirection::In => zoom_out(&mut app_state.editor),
    }
    None
}

fn handle_editor_touch_tap(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    deferred_start: Option<PointerContact>,
    contact: PointerContact,
) -> Option<EditorUiAction> {
    let Some(start) = deferred_start else {
        clear_active_stroke(app_state, contact.id);
        return None;
    };

    let surface = build_editor_surface_model(app_state, editor);
    let (screen_x, screen_y) = start.position.as_f64();
    let target = editor_surface_target_at(&surface, screen_x, screen_y);
    let started_contact = PointerContact {
        id: start.id,
        position: start.position,
        at: start.at,
    };

    let action = if app_state.is_editor_menu_open() {
        handle_editor_menu_target(app_state, target)
    } else {
        handle_editor_started_target(app_state, editor, started_contact, target);
        None
    };
    clear_active_stroke(app_state, contact.id);
    action
}

fn begin_touch_drag_if_needed(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    deferred_start: Option<PointerContact>,
) {
    let Some(start) = deferred_start else {
        return;
    };
    if !matches!(editor.mode(), EditorMode::Draw) || app_state.is_editor_menu_open() {
        return;
    }

    let surface = build_editor_surface_model(app_state, editor);
    let (screen_x, screen_y) = start.position.as_f64();
    let target = editor_surface_target_at(&surface, screen_x, screen_y);
    if let Some(EditorSurfaceTarget::BoardCell { world_x, world_y }) = target {
        begin_editor_board_interaction(
            app_state,
            editor,
            PointerContact {
                id: start.id,
                position: start.position,
                at: start.at,
            },
            world_x,
            world_y,
        );
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
    match slot {
        EditorControlSlot::BottomLeft => zoom_out(&mut app_state.editor),
        EditorControlSlot::BottomRight => zoom_in(&mut app_state.editor, editor),
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
        EditorMode::Move => {
            if handle_move_mode_double_tap(app_state, editor, world_x, world_y, contact.at) {
                return;
            }
            if editor.world().has_box(world_x, world_y) {
                if !is_selectable_move_mode_box(editor, world_x, world_y) {
                    return;
                }
                editor.apply_command(EditorCommand::SelectBox {
                    cell_x: world_x,
                    cell_y: world_y,
                });
            } else if matches!(editor.world().tile(world_x, world_y), Tile::Void) {
                editor.apply_command(EditorCommand::ClearSelection);
            } else {
                editor.apply_command(EditorCommand::MoveSelectedBoxTo {
                    cell_x: world_x,
                    cell_y: world_y,
                });
            }
        }
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

fn reset_editor_screen_touch_state(ui: &mut EditorUiState) {
    ui.interaction.active_stroke = None;
    ui.interaction.double_tap.clear();
}

fn resolve_paint_mode(
    ui: &mut EditorUiState,
    editor: &LevelEditor,
    world_x: i32,
    world_y: i32,
    at: Instant,
) -> PaintMode {
    let current_tile = editor.world().tile(world_x, world_y);
    if editor.world().has_box(world_x, world_y) {
        ui.interaction.double_tap.clear();
        return PaintMode::Void;
    }

    if ui.interaction.double_tap.register_tap(
        EditorDoubleTapTarget::DrawCell(world_x, world_y),
        at,
        ui.interaction.double_tap_window,
    ) {
        PaintMode::GoalWithBox
    } else {
        PaintMode::from_start_tile(current_tile)
    }
}

fn handle_move_mode_double_tap(
    app_state: &mut AppState,
    editor: &mut LevelEditor,
    world_x: i32,
    world_y: i32,
    at: Instant,
) -> bool {
    let target = move_mode_double_tap_target(editor, world_x, world_y);
    let Some(target) = target else {
        app_state.editor.interaction.double_tap.clear();
        return false;
    };

    let is_double_tap = app_state.editor.interaction.double_tap.register_tap(
        target,
        at,
        app_state.editor.interaction.double_tap_window,
    );
    if is_double_tap {
        match target {
            EditorDoubleTapTarget::MovePlayer => {
                editor.apply_command(EditorCommand::RestartToGoals);
                return true;
            }
            EditorDoubleTapTarget::MoveBox(world_x, world_y) => {
                if editor.last_move_destination() == Some((world_x, world_y)) {
                    editor.apply_command(EditorCommand::Undo);
                    return true;
                }
            }
            EditorDoubleTapTarget::DrawCell(_, _) => {}
        }
    }

    matches!(target, EditorDoubleTapTarget::MovePlayer) && editor.selected_box().is_none()
}

fn move_mode_double_tap_target(
    editor: &LevelEditor,
    world_x: i32,
    world_y: i32,
) -> Option<EditorDoubleTapTarget> {
    if editor.world().player() == Some((world_x, world_y)) && editor.can_restart() {
        return Some(EditorDoubleTapTarget::MovePlayer);
    }
    if editor.world().has_box(world_x, world_y) && editor.can_undo() {
        return Some(EditorDoubleTapTarget::MoveBox(world_x, world_y));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        EditorUiAction, build_editor_surface_model, editor_mouse_pressed, editor_mouse_released,
        editor_touch,
    };
    use crate::app::state::{AppOverlay, AppScreen, AppState};
    use crate::shared::PointerPhase;
    use presentation::layout::{
        editor_bottom_right_button_rect, overlay_secondary_action_button_rect,
    };
    use sokobanitron_gameplay::BoardCell;
    use sokobanitron_level_editor::{DrawTool, EditorCommand, EditorMode, LevelEditor};

    fn screen_center_for_world_cell(
        app_state: &AppState,
        editor: &LevelEditor,
        world_x: i32,
        world_y: i32,
    ) -> (f64, f64) {
        let surface = build_editor_surface_model(app_state, editor);
        let local_x = (world_x - surface.visible_window.world_origin_x) as u32;
        let local_y = (world_y - surface.visible_window.world_origin_y) as u32;
        let (screen_x, screen_y, width, height) = surface
            .visible_window
            .viewport
            .cell_to_screen_rect(BoardCell::new(local_x, local_y));
        (
            (screen_x + (width / 2) as i32) as f64,
            (screen_y + (height / 2) as i32) as f64,
        )
    }

    fn move_mode_editor_with_history() -> (AppState, LevelEditor) {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        let mut editor = LevelEditor::new();
        for x in 0..=4 {
            editor.apply_command(EditorCommand::PaintCell {
                cell_x: x,
                cell_y: 0,
                tool: DrawTool::Floor,
            });
        }
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 0,
            cell_y: 0,
            tool: DrawTool::GoalWithBox,
        });
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 4,
            cell_y: 0,
            tool: DrawTool::GoalWithBox,
        });
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });
        (app_state, editor)
    }

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
            tool: DrawTool::GoalWithBox,
        });
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
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

    #[test]
    fn desktop_draw_mode_zoom_button_zooms_in() {
        let mut app_state = AppState {
            supports_multi_touch: false,
            ..AppState::default()
        };
        app_state.ui.screen = AppScreen::Editor;
        let mut editor = LevelEditor::new();
        let rect = editor_bottom_right_button_rect(
            app_state.editor.viewport.surface_width,
            app_state.editor.viewport.surface_height,
        );

        let action = editor_mouse_pressed(
            &mut app_state,
            &mut editor,
            (rect.x + rect.w / 2) as f64,
            (rect.y + rect.h / 2) as f64,
        );

        assert_eq!(action, None);
        assert_eq!(app_state.editor.viewport.zoom_steps, -1);
    }

    #[test]
    fn pinch_out_zooms_in_editor_in_draw_mode() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        let mut editor = LevelEditor::new();
        let before = editor.snapshot();

        editor_touch(
            &mut app_state,
            &mut editor,
            1,
            PointerPhase::Started,
            100.0,
            100.0,
        );
        editor_touch(
            &mut app_state,
            &mut editor,
            2,
            PointerPhase::Started,
            200.0,
            100.0,
        );
        editor_touch(
            &mut app_state,
            &mut editor,
            1,
            PointerPhase::Moved,
            70.0,
            100.0,
        );
        editor_touch(
            &mut app_state,
            &mut editor,
            2,
            PointerPhase::Moved,
            230.0,
            100.0,
        );
        editor_touch(
            &mut app_state,
            &mut editor,
            1,
            PointerPhase::Ended,
            70.0,
            100.0,
        );

        assert_eq!(app_state.editor.viewport.zoom_steps, -1);
        assert_eq!(editor.snapshot(), before);
    }

    #[test]
    fn pinch_in_zooms_out_editor_in_move_mode() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        app_state.editor.viewport.zoom_steps = -1;
        let mut editor = LevelEditor::new();
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));

        editor_touch(
            &mut app_state,
            &mut editor,
            1,
            PointerPhase::Started,
            80.0,
            100.0,
        );
        editor_touch(
            &mut app_state,
            &mut editor,
            2,
            PointerPhase::Started,
            220.0,
            100.0,
        );
        editor_touch(
            &mut app_state,
            &mut editor,
            1,
            PointerPhase::Moved,
            120.0,
            100.0,
        );
        editor_touch(
            &mut app_state,
            &mut editor,
            2,
            PointerPhase::Moved,
            180.0,
            100.0,
        );
        editor_touch(
            &mut app_state,
            &mut editor,
            1,
            PointerPhase::Ended,
            120.0,
            100.0,
        );

        assert_eq!(app_state.editor.viewport.zoom_steps, 0);
    }

    #[test]
    fn single_touch_tap_in_draw_mode_still_paints() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        let mut editor = LevelEditor::new();
        let before = editor.snapshot();
        let screen_x = (app_state.editor.viewport.surface_width / 2) as f64;
        let screen_y = (app_state.editor.viewport.surface_height / 2) as f64;

        editor_touch(
            &mut app_state,
            &mut editor,
            1,
            PointerPhase::Started,
            screen_x,
            screen_y,
        );
        editor_touch(
            &mut app_state,
            &mut editor,
            1,
            PointerPhase::Ended,
            screen_x,
            screen_y,
        );

        assert_ne!(editor.snapshot(), before);
    }

    #[test]
    fn double_clicking_player_in_move_mode_restarts() {
        let (mut app_state, mut editor) = move_mode_editor_with_history();
        let player = editor
            .world()
            .player()
            .expect("expected player after first move");
        let (screen_x, screen_y) =
            screen_center_for_world_cell(&app_state, &editor, player.0, player.1);

        editor_mouse_pressed(&mut app_state, &mut editor, screen_x, screen_y);
        editor_mouse_released(&mut app_state);
        editor_mouse_pressed(&mut app_state, &mut editor, screen_x, screen_y);
        editor_mouse_released(&mut app_state);

        assert!(!editor.can_restart());
        assert_eq!(editor.world().player(), None);
        assert!(editor.world().has_box(0, 0));
        assert!(editor.world().has_box(4, 0));
        assert!(!editor.world().has_box(1, 0));
    }

    #[test]
    fn double_clicking_last_moved_box_in_move_mode_undoes_last_move() {
        let (mut app_state, mut editor) = move_mode_editor_with_history();
        let (screen_x, screen_y) = screen_center_for_world_cell(&app_state, &editor, 1, 0);

        editor_mouse_pressed(&mut app_state, &mut editor, screen_x, screen_y);
        editor_mouse_released(&mut app_state);
        editor_mouse_pressed(&mut app_state, &mut editor, screen_x, screen_y);
        editor_mouse_released(&mut app_state);

        assert!(!editor.can_undo());
        assert!(editor.world().has_box(0, 0));
        assert!(editor.world().has_box(4, 0));
        assert!(!editor.world().has_box(1, 0));
    }

    #[test]
    fn double_clicking_non_last_box_in_move_mode_only_toggles_selection() {
        let (mut app_state, mut editor) = move_mode_editor_with_history();
        let (screen_x, screen_y) = screen_center_for_world_cell(&app_state, &editor, 4, 0);

        editor_mouse_pressed(&mut app_state, &mut editor, screen_x, screen_y);
        editor_mouse_released(&mut app_state);
        editor_mouse_pressed(&mut app_state, &mut editor, screen_x, screen_y);
        editor_mouse_released(&mut app_state);

        assert!(editor.can_undo());
        assert_eq!(editor.selected_box(), None);
        assert!(editor.world().has_box(1, 0));
        assert!(editor.world().has_box(4, 0));
    }
}
