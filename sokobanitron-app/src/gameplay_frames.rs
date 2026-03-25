use crate::app_state::AppState;
use crate::frame::FrameRequest;
use crate::overlay::{is_gameplay_menu_open, level_select_page_start};
use crate::presentation_profile::PresentMode;
use presentation::assets::UiIcon;
use presentation::screen_requests::{
    GameplayMenuScreenRequest, GameplayScreenRequest, LevelSelectScreenRequest,
};
use sokobanitron_gameplay::GameplayController;

pub fn build_gameplay_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
    present_mode: PresentMode,
) -> FrameRequest {
    FrameRequest::Gameplay {
        screen: build_gameplay_screen_request(controller, app_state),
        present_mode,
    }
}

pub fn build_level_select_frame_request(
    page_start: usize,
    present_mode: PresentMode,
) -> FrameRequest {
    FrameRequest::LevelSelect {
        screen: LevelSelectScreenRequest { page_start },
        present_mode,
    }
}

pub fn build_current_gameplay_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
) -> FrameRequest {
    build_gameplay_frame_request(controller, app_state, PresentMode::Full)
}

pub fn build_current_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
) -> FrameRequest {
    if let Some(page_start) = level_select_page_start(app_state) {
        build_level_select_frame_request(page_start, PresentMode::Full)
    } else if is_gameplay_menu_open(app_state) {
        FrameRequest::GameplayMenu {
            screen: GameplayMenuScreenRequest {
                primary_action_icon: app_state.editor_available.then_some(UiIcon::Draw),
            },
        }
    } else {
        build_current_gameplay_frame_request(controller, app_state)
    }
}

pub(crate) fn build_gameplay_screen_request(
    controller: &GameplayController,
    _app_state: &AppState,
) -> GameplayScreenRequest {
    GameplayScreenRequest {
        can_undo: controller.can_undo(),
        can_restart: controller.can_restart(),
        level_number: controller.current_level() + 1,
        show_solved_overlay: controller.board().is_solved(),
    }
}

#[cfg(test)]
mod tests {
    use super::{build_current_frame_request, build_level_select_frame_request};
    use crate::{AppOverlay, AppState, FrameRequest, PresentMode};
    use presentation::screen_requests::GameplayMenuScreenRequest;
    use sokobanitron_gameplay::GameplayController;

    fn controller() -> GameplayController {
        let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
        GameplayController::new(vec![level], None)
    }

    #[test]
    fn current_frame_request_returns_level_select_when_open() {
        let controller = controller();
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start: 12 });

        assert_eq!(
            build_current_frame_request(&controller, &app_state),
            build_level_select_frame_request(12, PresentMode::Full),
        );
    }

    #[test]
    fn current_frame_request_returns_gameplay_menu_when_open() {
        let controller = controller();
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::GameplayMenu);

        assert_eq!(
            build_current_frame_request(&controller, &app_state),
            FrameRequest::GameplayMenu {
                screen: GameplayMenuScreenRequest {
                    primary_action_icon: None,
                },
            },
        );
    }

    #[test]
    fn current_frame_request_returns_gameplay_when_no_overlay() {
        let controller = controller();
        let app_state = AppState::default();

        assert!(matches!(
            build_current_frame_request(&controller, &app_state),
            FrameRequest::Gameplay {
                present_mode: PresentMode::Full,
                ..
            }
        ));
    }
}
