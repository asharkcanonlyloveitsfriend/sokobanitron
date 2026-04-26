//! Gameplay screen request shaping.
//!
//! This module converts gameplay/app state into gameplay-facing `FrameRequest` values.
//! It should describe what the shared presentation layer needs to render gameplay, while staying
//! free of pixel drawing and platform timing concerns.
//!
//! Gameplay now follows the same basic rendering contract as the editor path: the app shapes a
//! request that includes the board scene and viewport, and clients hand that request to the shared
//! presentation layer.

use super::view::build_gameplay_board_viewport;
use crate::app::presentation::FrameRequest;
use crate::app::state::AppState;
use presentation::screen_requests::{
    GameplayMenuScreenRequest, GameplayPresentationCause, GameplayPresentationUpdate,
    GameplayScreenMode, GameplayScreenRequest, LevelSelectScreenRequest, LevelSetListEntry,
    LevelSetSelectScreenRequest,
};
use sokobanitron_gameplay::GameplayController;

pub fn build_gameplay_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
) -> FrameRequest {
    FrameRequest::Gameplay {
        update: build_gameplay_presentation_update(
            controller,
            app_state,
            GameplayScreenMode::Normal,
            GameplayPresentationCause::CurrentState,
        ),
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
    }
}

pub(crate) fn build_gameplay_frame_request_with_cause(
    controller: &GameplayController,
    app_state: &AppState,
    cause: GameplayPresentationCause,
) -> FrameRequest {
    FrameRequest::Gameplay {
        update: build_gameplay_presentation_update(
            controller,
            app_state,
            GameplayScreenMode::Normal,
            cause,
        ),
    }
}

pub fn build_level_select_frame_request(page_start: usize, resume_level: usize) -> FrameRequest {
    FrameRequest::LevelSelect {
        screen: LevelSelectScreenRequest {
            page_start,
            resume_level,
        },
    }
}

pub fn build_level_set_select_frame_request(
    page_start: usize,
    app_state: &AppState,
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
    }
}

pub fn build_current_gameplay_board_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
) -> FrameRequest {
    build_gameplay_frame_request(controller, app_state)
}

pub fn build_current_gameplay_screen_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
) -> FrameRequest {
    if let Some(page_start) = app_state.level_set_select_page_start() {
        build_level_set_select_frame_request(page_start, app_state)
    } else if let Some(page_start) = app_state.level_select_page_start() {
        build_level_select_frame_request(page_start, controller.resume_level())
    } else if app_state.is_gameplay_menu_open() {
        FrameRequest::GameplayMenu {
            screen: GameplayMenuScreenRequest {
                primary_action_label: Some("EDIT"),
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
    let viewport = build_gameplay_board_viewport(&app_state.gameplay, board);
    GameplayPresentationUpdate {
        scene: GameplayScreenRequest {
            board: board.clone(),
            viewport,
            level_number: controller.current_level() + 1,
            mode,
        },
        cause,
    }
}

#[cfg(test)]
mod tests {
    use super::{build_current_gameplay_screen_frame_request, build_level_select_frame_request};
    use crate::app::presentation::FrameRequest;
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
            build_level_select_frame_request(12, controller.resume_level()),
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
                    primary_action_label: Some("EDIT"),
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
        } = build_current_gameplay_screen_frame_request(&controller, &app_state)
        else {
            panic!("expected gameplay request");
        };

        assert_eq!(level_number, 1);
        assert_eq!(cause, GameplayPresentationCause::CurrentState);
    }
}
