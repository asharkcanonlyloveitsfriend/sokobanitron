//! App-owned editor frame shaping.
//!
//! This module converts `LevelEditor` snapshots plus app-side viewport state into
//! renderer request structs. It must not mutate editor domain state or draw pixels.

use crate::app::presentation::FrameRequest;
use crate::app::state::AppState;
use presentation::assets::UiIcon;
use presentation::layout::ScreenRect;
use presentation::screen_requests::{
    EditorCountOverlay, EditorHintOverlay, EditorMenuScreenRequest, EditorScreenRequest,
};
use sokobanitron_level_editor::{EditorMode, EditorSnapshot, LevelEditor};

use super::view::{
    VisibleBoardWindow, build_visible_window, can_save_editor_puzzle, can_zoom_in, can_zoom_out,
};

pub fn build_current_editor_frame_request(
    app_state: &AppState,
    editor: &LevelEditor,
) -> FrameRequest {
    if app_state.is_editor_menu_open() {
        FrameRequest::EditorMenu {
            screen: EditorMenuScreenRequest {
                primary_action_icon: UiIcon::Select,
                show_save_button: can_save_editor_puzzle(editor),
            },
        }
    } else {
        let snapshot = editor.snapshot();
        let visible = build_visible_window(&app_state.editor, editor);
        FrameRequest::Editor {
            screen: EditorScreenRequest {
                move_counts: build_count_overlays(&visible, &snapshot),
                pull_destination_hints: build_hint_overlays(&visible, &snapshot),
                board: visible.board,
                viewport: visible.viewport,
                draw_mode_active: matches!(snapshot.mode, EditorMode::Draw),
                can_zoom_out: can_zoom_out(&app_state.editor),
                can_zoom_in: can_zoom_in(&app_state.editor, editor),
                can_undo: snapshot.can_undo,
                can_restart: snapshot.can_restart,
            },
        }
    }
}

fn build_count_overlays(
    visible: &VisibleBoardWindow,
    snapshot: &EditorSnapshot,
) -> Vec<EditorCountOverlay> {
    let mut overlays = Vec::new();
    for count in &snapshot.move_counts {
        let local_x = count.world_x - visible.world_origin_x;
        let local_y = count.world_y - visible.world_origin_y;
        if local_x < 0
            || local_y < 0
            || local_x >= visible.board.width() as i32
            || local_y >= visible.board.height() as i32
        {
            continue;
        }
        let (cell_x, cell_y, cell_w, cell_h) = visible
            .viewport
            .cell_to_screen_rect(local_x as u32, local_y as u32);
        let inset = (cell_w / 24).max(1);
        let box_x = cell_x + inset as i32;
        let box_y = cell_y + inset as i32;
        if box_x < 0 || box_y < 0 {
            continue;
        }
        let rect = ScreenRect {
            x: box_x as u32,
            y: box_y as u32,
            w: cell_w.saturating_sub(inset * 2),
            h: cell_h.saturating_sub(inset * 2),
        };
        if rect.w == 0 || rect.h == 0 {
            continue;
        }
        overlays.push(EditorCountOverlay {
            rect,
            count: count.count,
        });
    }
    overlays
}

fn build_hint_overlays(
    visible: &VisibleBoardWindow,
    snapshot: &EditorSnapshot,
) -> Vec<EditorHintOverlay> {
    if !matches!(snapshot.mode, EditorMode::Move) || snapshot.selected_box.is_none() {
        return Vec::new();
    }

    let width = visible.board.width() as i32;
    let height = visible.board.height() as i32;
    let mut overlays = Vec::new();
    for hint in &snapshot.pull_destination_hints {
        let local_x = hint.world_x - visible.world_origin_x;
        let local_y = hint.world_y - visible.world_origin_y;
        if local_x < 0 || local_y < 0 || local_x >= width || local_y >= height {
            continue;
        }
        let (cell_x, cell_y, cell_w, cell_h) = visible
            .viewport
            .cell_to_screen_rect(local_x as u32, local_y as u32);
        if cell_x < 0 || cell_y < 0 {
            continue;
        }
        overlays.push(EditorHintOverlay {
            rect: ScreenRect {
                x: cell_x as u32,
                y: cell_y as u32,
                w: cell_w,
                h: cell_h,
            },
            state: hint.state,
        });
    }
    overlays
}

#[cfg(test)]
mod tests {
    use super::build_current_editor_frame_request;
    use crate::app::presentation::FrameRequest;
    use crate::app::state::{AppOverlay, AppScreen, AppState};
    use sokobanitron_level_editor::{DrawTool, EditorCommand, EditorMode, LevelEditor};

    #[test]
    fn editor_menu_hides_save_without_off_goal_box() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        app_state.ui.overlay = Some(AppOverlay::EditorMenu);
        let editor = LevelEditor::new();

        let FrameRequest::EditorMenu { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor menu frame");
        };

        assert!(!screen.show_save_button);
    }

    #[test]
    fn editor_menu_shows_save_with_off_goal_box() {
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

        let FrameRequest::EditorMenu { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor menu frame");
        };

        assert!(screen.show_save_button);
    }
}
