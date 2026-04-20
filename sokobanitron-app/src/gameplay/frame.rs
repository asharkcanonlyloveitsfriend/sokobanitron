//! Gameplay screen request shaping.
//!
//! This module converts gameplay/app state into gameplay-facing `FrameRequest` values.
//! It should describe what the shared presentation layer needs to render gameplay, while staying
//! free of pixel drawing and platform timing concerns.
//!
//! Gameplay now follows the same basic rendering contract as the editor path: the app shapes a
//! request that includes the board scene and viewport, and clients hand that request to the shared
//! presentation layer.

use super::view::{GameplayVisibleBoardWindow, build_gameplay_visible_window};
use crate::app::presentation::{FrameRequest, PresentMode};
use crate::app::state::AppState;
use presentation::assets::UiIcon;
use presentation::screen_requests::{
    GameplayMenuScreenRequest, GameplayPresentationCause, GameplayPresentationUpdate,
    GameplayScreenMode, GameplayScreenRequest, LevelSelectScreenRequest, LevelSetListEntry,
    LevelSetSelectScreenRequest,
};
use sokobanitron_gameplay::{BoardView, GameplayController};

pub fn build_gameplay_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
    present_mode: PresentMode,
) -> FrameRequest {
    FrameRequest::Gameplay {
        update: build_gameplay_presentation_update(
            controller,
            app_state,
            GameplayScreenMode::Normal,
            GameplayPresentationCause::CurrentState,
        ),
        present_mode,
    }
}

pub fn build_sleep_gameplay_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
) -> FrameRequest {
    FrameRequest::Gameplay {
        update: build_gameplay_presentation_update(
            controller,
            app_state,
            GameplayScreenMode::Sleep,
            GameplayPresentationCause::CurrentState,
        ),
        present_mode: PresentMode::Full,
    }
}

pub(crate) fn build_gameplay_frame_request_with_cause(
    controller: &GameplayController,
    app_state: &AppState,
    cause: GameplayPresentationCause,
    present_mode: PresentMode,
) -> FrameRequest {
    FrameRequest::Gameplay {
        update: build_gameplay_presentation_update(
            controller,
            app_state,
            GameplayScreenMode::Normal,
            cause,
        ),
        present_mode,
    }
}

pub fn build_level_select_frame_request(
    page_start: usize,
    resume_level: usize,
    present_mode: PresentMode,
) -> FrameRequest {
    FrameRequest::LevelSelect {
        screen: LevelSelectScreenRequest {
            page_start,
            resume_level,
        },
        present_mode,
    }
}

pub fn build_level_set_select_frame_request(
    page_start: usize,
    app_state: &AppState,
    present_mode: PresentMode,
) -> FrameRequest {
    FrameRequest::LevelSetSelect {
        screen: LevelSetSelectScreenRequest {
            page_start,
            active_level_set: app_state.gameplay.active_level_set,
            entries: app_state
                .gameplay
                .level_sets
                .iter()
                .map(|entry| LevelSetListEntry {
                    title: entry.title.clone(),
                    completed_puzzle_count: entry.completed_puzzle_count,
                    total_puzzle_count: entry.total_puzzle_count,
                })
                .collect(),
        },
        present_mode,
    }
}

pub fn build_current_gameplay_board_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
) -> FrameRequest {
    build_gameplay_frame_request(controller, app_state, PresentMode::Full)
}

pub fn build_current_gameplay_screen_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
) -> FrameRequest {
    if let Some(page_start) = app_state.level_set_select_page_start() {
        build_level_set_select_frame_request(page_start, app_state, PresentMode::Full)
    } else if let Some(page_start) = app_state.level_select_page_start() {
        build_level_select_frame_request(page_start, controller.resume_level(), PresentMode::Full)
    } else if app_state.is_gameplay_menu_open() {
        FrameRequest::GameplayMenu {
            screen: GameplayMenuScreenRequest {
                primary_action_icon: app_state.editor_available.then_some(UiIcon::Draw),
                show_change_level_set: app_state.gameplay.level_sets.len() > 1,
            },
        }
    } else {
        build_current_gameplay_board_frame_request(controller, app_state)
    }
}

fn build_gameplay_presentation_update(
    controller: &GameplayController,
    app_state: &AppState,
    mode: GameplayScreenMode,
    cause: GameplayPresentationCause,
) -> GameplayPresentationUpdate {
    let board = controller.board();
    let visible_window = build_gameplay_visible_window(&app_state.gameplay, board);
    let cause = if gameplay_window_is_cropped(board, &visible_window) {
        localize_gameplay_presentation_cause(cause, &visible_window)
    } else {
        cause
    };
    GameplayPresentationUpdate {
        scene: GameplayScreenRequest {
            board: visible_window.board,
            viewport: visible_window.viewport,
            level_number: controller.current_level() + 1,
            mode,
        },
        cause,
    }
}

fn gameplay_window_is_cropped(
    board: &BoardView,
    visible_window: &GameplayVisibleBoardWindow,
) -> bool {
    visible_window.board_origin_x != 0
        || visible_window.board_origin_y != 0
        || visible_window.board.width() != board.width()
        || visible_window.board.height() != board.height()
}

fn localize_gameplay_presentation_cause(
    cause: GameplayPresentationCause,
    visible_window: &GameplayVisibleBoardWindow,
) -> GameplayPresentationCause {
    match cause {
        GameplayPresentationCause::CurrentState
        | GameplayPresentationCause::BoxMoveRejected
        | GameplayPresentationCause::PuzzleSolved { .. }
        | GameplayPresentationCause::UndoApplied
        | GameplayPresentationCause::Restarted => cause,
        GameplayPresentationCause::SelectionChanged { selected_box } => {
            GameplayPresentationCause::SelectionChanged {
                selected_box: selected_box
                    .and_then(|cell| visible_window.world_to_local_cell(cell)),
            }
        }
        GameplayPresentationCause::PlayerMoved { to } => visible_window
            .world_to_local_cell(to)
            .map(|to| GameplayPresentationCause::PlayerMoved { to })
            .unwrap_or(GameplayPresentationCause::CurrentState),
        GameplayPresentationCause::BoxMoved { path } => {
            let localized_path: Vec<_> = path
                .into_iter()
                .filter_map(|cell| visible_window.world_to_local_cell(cell))
                .collect();
            if localized_path.len() >= 2 {
                GameplayPresentationCause::BoxMoved {
                    path: localized_path,
                }
            } else {
                GameplayPresentationCause::CurrentState
            }
        }
        GameplayPresentationCause::BoxRemoved { to } => visible_window
            .world_to_local_cell(to)
            .map(|to| GameplayPresentationCause::BoxRemoved { to })
            .unwrap_or(GameplayPresentationCause::CurrentState),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_current_gameplay_screen_frame_request, build_level_select_frame_request};
    use crate::app::presentation::{FrameRequest, PresentMode};
    use crate::app::state::{AppOverlay, AppState};
    use presentation::screen_requests::{
        GameplayMenuScreenRequest, GameplayPresentationCause, GameplayScreenRequest,
    };
    use sokobanitron_gameplay::GameplayController;

    fn controller() -> GameplayController {
        let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
        GameplayController::new(vec![level], None)
    }

    #[test]
    fn current_gameplay_screen_frame_request_returns_level_select_when_open() {
        let controller = controller();
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start: 12 });

        assert_eq!(
            build_current_gameplay_screen_frame_request(&controller, &app_state),
            build_level_select_frame_request(12, controller.resume_level(), PresentMode::Full,),
        );
    }

    #[test]
    fn current_gameplay_screen_frame_request_uses_resume_level_for_level_select_indicator() {
        let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
        let controller = GameplayController::new_at_level(vec![level.clone(), level], 0, Some(1));
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start: 0 });

        let FrameRequest::LevelSelect { screen, .. } =
            build_current_gameplay_screen_frame_request(&controller, &app_state)
        else {
            panic!("expected level select request");
        };

        assert_eq!(screen.resume_level, 1);
    }

    #[test]
    fn current_gameplay_screen_frame_request_returns_gameplay_menu_when_open() {
        let controller = controller();
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::GameplayMenu);

        assert_eq!(
            build_current_gameplay_screen_frame_request(&controller, &app_state),
            FrameRequest::GameplayMenu {
                screen: GameplayMenuScreenRequest {
                    primary_action_icon: None,
                    show_change_level_set: false,
                },
            },
        );
    }

    #[test]
    fn current_gameplay_screen_frame_request_returns_gameplay_when_no_overlay() {
        let controller = controller();
        let app_state = AppState::default();

        let FrameRequest::Gameplay {
            update:
                presentation::screen_requests::GameplayPresentationUpdate {
                    scene: GameplayScreenRequest { level_number, .. },
                    cause,
                },
            present_mode,
        } = build_current_gameplay_screen_frame_request(&controller, &app_state)
        else {
            panic!("expected gameplay request");
        };

        assert_eq!(present_mode, PresentMode::Full);
        assert_eq!(level_number, 1);
        assert_eq!(cause, GameplayPresentationCause::CurrentState);
    }
}
