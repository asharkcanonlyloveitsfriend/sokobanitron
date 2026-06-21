//! App-owned editor frame shaping.
//!
//! This module converts `LevelEditor` snapshots plus app-side viewport state into
//! renderer request structs. It must not mutate editor domain state or draw pixels.

use crate::app::presentation::FrameRequest;
use crate::app::state::AppState;
use presentation::layout::ScreenRect;
use presentation::screen_requests::{
    EditorCountOverlay, EditorMenuScreenRequest, EditorModeIndicator, EditorModeMenuScreenRequest,
    EditorScreenRequest, EditorWarningKind, EditorWarningOverlay,
};
use sokobanitron_gameplay::BoardCell;
use sokobanitron_level_editor::{EditorMode, LevelEditor, Tile};

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
                primary_action_label: "PLAY",
                show_save_button: can_save_editor_puzzle(editor),
            },
        }
    } else if app_state.is_editor_mode_menu_open() {
        FrameRequest::EditorModeMenu {
            screen: EditorModeMenuScreenRequest {
                editor: build_editor_screen_request(app_state, editor, false),
                can_enter_play: editor.can_enter_play(),
            },
        }
    } else {
        FrameRequest::Editor {
            screen: build_editor_screen_request(app_state, editor, false),
        }
    }
}

pub fn build_sleep_editor_frame_request(
    app_state: &AppState,
    editor: &LevelEditor,
) -> FrameRequest {
    FrameRequest::Editor {
        screen: build_editor_screen_request(app_state, editor, true),
    }
}

fn build_editor_screen_request(
    app_state: &AppState,
    editor: &LevelEditor,
    sleeping_player: bool,
) -> EditorScreenRequest {
    let snapshot = editor.snapshot();
    let visible = build_visible_window(&app_state.editor, editor);
    EditorScreenRequest {
        move_counts: build_count_overlays(&visible, editor),
        warnings: build_warning_overlays(&visible, editor),
        board: visible.board,
        viewport: visible.viewport,
        mode_indicator: match snapshot.mode {
            EditorMode::Draw => EditorModeIndicator::Draw,
            EditorMode::Move => EditorModeIndicator::Move,
            EditorMode::Play => EditorModeIndicator::Play,
        },
        puzzle_solved: editor.view_is_solved(),
        can_zoom_out: !app_state.supports_multi_touch
            && matches!(snapshot.mode, EditorMode::Draw)
            && can_zoom_out(&app_state.editor),
        can_zoom_in: !app_state.supports_multi_touch
            && matches!(snapshot.mode, EditorMode::Draw)
            && can_zoom_in(&app_state.editor, editor),
        sleeping_player,
    }
}

fn build_warning_overlays(
    visible: &VisibleBoardWindow,
    editor: &LevelEditor,
) -> Vec<EditorWarningOverlay> {
    let boxes = editor.world().box_positions();
    let goals = editor.world().goal_positions();
    match boxes.len().cmp(&goals.len()) {
        std::cmp::Ordering::Greater => {
            let extra_count = boxes.len() - goals.len();
            let boxes_not_on_goals = boxes
                .into_iter()
                .filter(|&(world_x, world_y)| {
                    !matches!(editor.world().tile(world_x, world_y), Tile::Goal)
                })
                .collect::<Vec<_>>();
            let first_extra = boxes_not_on_goals.len().saturating_sub(extra_count);
            boxes_not_on_goals
                .into_iter()
                .skip(first_extra)
                .filter_map(|(world_x, world_y)| {
                    visible_cell(visible, world_x, world_y).map(|cell| EditorWarningOverlay {
                        cell,
                        kind: EditorWarningKind::Box,
                    })
                })
                .collect()
        }
        std::cmp::Ordering::Less => {
            let extra_count = goals.len() - boxes.len();
            let unboxed_goals = goals
                .into_iter()
                .filter(|&(world_x, world_y)| !editor.world().has_box(world_x, world_y))
                .collect::<Vec<_>>();
            let first_extra = unboxed_goals.len().saturating_sub(extra_count);
            unboxed_goals
                .into_iter()
                .skip(first_extra)
                .filter_map(|(world_x, world_y)| {
                    visible_cell(visible, world_x, world_y).map(|cell| EditorWarningOverlay {
                        cell,
                        kind: EditorWarningKind::Goal,
                    })
                })
                .collect()
        }
        std::cmp::Ordering::Equal => Vec::new(),
    }
}

fn visible_cell(visible: &VisibleBoardWindow, world_x: i32, world_y: i32) -> Option<BoardCell> {
    let local_x = world_x - visible.world_origin_x;
    let local_y = world_y - visible.world_origin_y;
    if local_x < 0
        || local_y < 0
        || local_x >= visible.board.width() as i32
        || local_y >= visible.board.height() as i32
    {
        return None;
    }
    Some(BoardCell::new(local_x as u32, local_y as u32))
}

fn build_count_overlays(
    visible: &VisibleBoardWindow,
    editor: &LevelEditor,
) -> Vec<EditorCountOverlay> {
    let mut overlays = Vec::new();
    for count in editor.box_move_counts() {
        let Some(cell) = visible_cell(visible, count.world_x, count.world_y) else {
            continue;
        };
        let (cell_x, cell_y, cell_w, cell_h) = visible.viewport.cell_to_screen_rect(cell);
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

#[cfg(test)]
mod tests {
    use super::{build_current_editor_frame_request, build_sleep_editor_frame_request};
    use crate::app::presentation::FrameRequest;
    use crate::app::state::{AppOverlay, AppScreen, AppState};
    use presentation::screen_requests::{EditorModeIndicator, EditorWarningKind};
    use sokobanitron_gameplay::TileKind;
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
    fn editor_mode_menu_wraps_current_editor_frame_state() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        app_state.ui.overlay = Some(AppOverlay::EditorModeMenu);
        let editor = LevelEditor::new();

        let FrameRequest::EditorModeMenu { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor mode menu frame");
        };

        assert_eq!(screen.editor.mode_indicator, EditorModeIndicator::Draw);
        assert!(!screen.can_enter_play);
    }

    #[test]
    fn sleep_editor_frame_returns_plain_editor_frame_and_marks_player_sleeping() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        app_state.ui.overlay = Some(AppOverlay::EditorModeMenu);
        let mut editor = LevelEditor::new();
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 0,
            cell_y: 0,
        });

        let FrameRequest::Editor { screen } = build_sleep_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor frame");
        };

        assert_eq!(screen.mode_indicator, EditorModeIndicator::Move);
        assert!(screen.sleeping_player);
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
    fn solved_play_mode_frame_includes_box_move_counts() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        let editor = validated_editor();

        let FrameRequest::Editor { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor frame");
        };

        assert!(screen.puzzle_solved);
        assert_eq!(screen.move_counts.len(), 1);
        assert_eq!(screen.move_counts[0].count, 1);
    }

    #[test]
    fn draw_and_move_mode_frames_include_validated_box_move_counts() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        let mut editor = validated_editor();

        editor.apply_command(EditorCommand::SetMode(EditorMode::Draw));
        let FrameRequest::Editor { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor frame");
        };
        assert_eq!(screen.mode_indicator, EditorModeIndicator::Draw);
        assert_eq!(screen.move_counts.len(), 1);
        assert_eq!(screen.move_counts[0].count, 1);

        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        let FrameRequest::Editor { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor frame");
        };
        assert_eq!(screen.mode_indicator, EditorModeIndicator::Move);
        assert_eq!(screen.move_counts.len(), 1);
        assert_eq!(screen.move_counts[0].count, 1);
    }

    #[test]
    fn draw_mutation_removes_validated_box_move_counts() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        let mut editor = validated_editor();
        editor.apply_command(EditorCommand::SetMode(EditorMode::Draw));

        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 3,
            cell_y: 0,
            tool: DrawTool::Void,
        });

        let FrameRequest::Editor { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor frame");
        };
        assert!(screen.move_counts.is_empty());
    }

    #[test]
    fn editor_frame_marks_extra_boxes() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        let mut editor = LevelEditor::new();
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 0,
            cell_y: 0,
            tool: DrawTool::Box,
        });
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 1,
            cell_y: 0,
            tool: DrawTool::GoalWithBox,
        });

        let FrameRequest::Editor { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor frame");
        };

        assert_eq!(screen.warnings.len(), 1);
        assert_eq!(screen.warnings[0].kind, EditorWarningKind::Box);
        assert!(screen.board.has_box(screen.warnings[0].cell));
        assert_ne!(screen.board.tile(screen.warnings[0].cell), TileKind::Goal);
    }

    #[test]
    fn editor_frame_marks_extra_goals() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        let mut editor = LevelEditor::new();
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 0,
            cell_y: 0,
            tool: DrawTool::Goal,
        });
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 1,
            cell_y: 0,
            tool: DrawTool::GoalWithBox,
        });

        let FrameRequest::Editor { screen } =
            build_current_editor_frame_request(&app_state, &editor)
        else {
            panic!("expected editor frame");
        };

        assert_eq!(screen.warnings.len(), 1);
        assert_eq!(screen.warnings[0].kind, EditorWarningKind::Goal);
        assert_eq!(screen.board.tile(screen.warnings[0].cell), TileKind::Goal);
        assert!(!screen.board.has_box(screen.warnings[0].cell));
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
