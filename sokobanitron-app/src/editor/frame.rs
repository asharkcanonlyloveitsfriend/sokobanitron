//! App-owned editor frame shaping.
//!
//! This module converts `LevelEditor` snapshots plus app-side viewport state into
//! renderer request structs. It must not mutate editor domain state or draw pixels.

use crate::app::presentation::FrameRequest;
use crate::app::state::AppState;
use presentation::screen_requests::{
    EditorMenuScreenRequest, EditorModeIndicator, EditorScreenRequest,
};
use sokobanitron_level_editor::{EditorMode, LevelEditor};

use super::view::{build_visible_window, can_save_editor_puzzle, can_zoom_in, can_zoom_out};

pub fn build_current_editor_frame_request(
    app_state: &AppState,
    editor: &LevelEditor,
) -> FrameRequest {
    if app_state.is_editor_menu_open() {
        FrameRequest::EditorMenu {
            screen: EditorMenuScreenRequest {
                primary_action_label: "PLAY",
                show_save_button: can_save_editor_puzzle(editor),
            },
        }
    } else {
        let snapshot = editor.snapshot();
        let visible = build_visible_window(&app_state.editor, editor);
        FrameRequest::Editor {
            screen: EditorScreenRequest {
                board: visible.board,
                viewport: visible.viewport,
                mode_indicator: match snapshot.mode {
                    EditorMode::Draw => EditorModeIndicator::Draw,
                    EditorMode::Move => EditorModeIndicator::Move,
                    EditorMode::Play => EditorModeIndicator::Play,
                },
                can_zoom_out: !app_state.supports_multi_touch
                    && matches!(snapshot.mode, EditorMode::Draw)
                    && can_zoom_out(&app_state.editor),
                can_zoom_in: !app_state.supports_multi_touch
                    && matches!(snapshot.mode, EditorMode::Draw)
                    && can_zoom_in(&app_state.editor, editor),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_current_editor_frame_request;
    use crate::app::presentation::FrameRequest;
    use crate::app::state::{AppOverlay, AppScreen, AppState};
    use presentation::screen_requests::EditorModeIndicator;
    use sokobanitron_level_editor::{DrawTool, EditorCommand, EditorMode, LevelEditor};

    fn validated_editor() -> LevelEditor {
        let mut editor = LevelEditor::new();
        for x in 0..=3 {
            editor.apply_command(EditorCommand::PaintCell {
                cell_x: x,
                cell_y: 0,
                tool: DrawTool::Floor,
            });
        }
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 2,
            cell_y: 0,
            tool: DrawTool::GoalWithBox,
        });
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 2,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::ToggleMode);
        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 1,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 2,
            cell_y: 0,
        });
        editor
    }

    #[test]
    fn editor_menu_hides_save_until_level_has_been_solved_in_play_mode() {
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
    fn editor_menu_shows_save_after_validated_solution() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        app_state.ui.overlay = Some(AppOverlay::EditorMenu);
        let editor = validated_editor();

        let FrameRequest::EditorMenu { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor menu frame");
        };

        assert!(screen.show_save_button);
    }

    #[test]
    fn play_mode_frame_uses_play_indicator() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        let mut editor = validated_editor();
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::ToggleMode);

        let FrameRequest::Editor { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor frame");
        };

        assert_eq!(screen.mode_indicator, EditorModeIndicator::Play);
    }

    #[test]
    fn desktop_editor_frame_exposes_draw_mode_zoom_controls() {
        let app_state = AppState {
            supports_multi_touch: false,
            ..AppState::default()
        };
        let editor = LevelEditor::new();

        let FrameRequest::Editor { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor frame");
        };

        assert_eq!(screen.mode_indicator, EditorModeIndicator::Draw);
        assert!(screen.can_zoom_out);
        assert!(screen.can_zoom_in);
    }

    #[test]
    fn touch_capable_editor_frame_hides_draw_mode_zoom_controls() {
        let app_state = AppState {
            supports_multi_touch: true,
            ..AppState::default()
        };
        let editor = LevelEditor::new();

        let FrameRequest::Editor { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor frame");
        };

        assert_eq!(screen.mode_indicator, EditorModeIndicator::Draw);
        assert!(!screen.can_zoom_out);
        assert!(!screen.can_zoom_in);
    }
}
