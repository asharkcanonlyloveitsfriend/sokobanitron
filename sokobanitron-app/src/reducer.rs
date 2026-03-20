use crate::action::AppAction;
use crate::app_state::AppState;
use crate::present::{PresentationPlan, build_presentation_plan};
use crate::presentation_profile::PresentationProfile;
use crate::ui_state::AppMode;
use sokobanitron_gameplay::{GameplayController, GameplayControllerChanges};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppUpdate {
    pub changes: GameplayControllerChanges,
    pub presentation_plan: Option<PresentationPlan>,
}

pub fn apply_action(
    controller: &mut GameplayController,
    app_state: &mut AppState,
    action: AppAction,
    profile: &PresentationProfile,
) -> AppUpdate {
    let mut update = AppUpdate::default();

    match action {
        AppAction::Restart => {
            update.changes = controller.restart_with_changes();
        }
        AppAction::Undo => {
            update.changes = controller.undo_with_changes();
        }
        AppAction::ToggleMenu => match app_state.ui.mode {
            AppMode::Gameplay => {
                app_state.ui.mode = AppMode::Menu {
                    page_start: controller.current_level(),
                };
            }
            AppMode::Menu { .. } => {
                app_state.ui.mode = AppMode::Gameplay;
            }
        },
        AppAction::OpenMenu => {
            app_state.ui.mode = AppMode::Menu {
                page_start: controller.current_level(),
            };
        }
        AppAction::CloseMenu => {
            app_state.ui.mode = AppMode::Gameplay;
        }
        AppAction::SetMenuPageStart(page_start) => {
            if let AppMode::Menu {
                page_start: current_page_start,
            } = &mut app_state.ui.mode
            {
                *current_page_start = page_start;
            }
        }
        AppAction::SelectLevel(level) => {
            update.changes = controller.jump_to_level(level);
            app_state.ui.mode = AppMode::Gameplay;
        }
        AppAction::AdvanceAfterSolved => {
            if let Some(next_level) = controller.peek_level(1) {
                update.changes = controller.advance_after_win(next_level);
            }
        }
        AppAction::TapBoardCell { x, y } => {
            if matches!(app_state.ui.mode, AppMode::Gameplay) {
                let outcome = controller.click_cell_with_outcome(x, y);
                update.changes = outcome.changes;
                update.presentation_plan = Some(build_presentation_plan(&outcome, profile));
            }
        }
        AppAction::NoOp => {}
    }

    update
}

#[cfg(test)]
mod tests {
    use super::apply_action;
    use crate::{AppAction, AppMode, AppState, PresentationProfile};
    use sokobanitron_gameplay::GameplayController;

    fn test_controller() -> GameplayController {
        let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
        GameplayController::new(vec![level], None)
    }

    #[test]
    fn set_menu_page_start_updates_menu_mode() {
        let mut controller = test_controller();
        let mut app_state = AppState::default();
        app_state.ui.mode = AppMode::Menu { page_start: 0 };
        let profile = PresentationProfile::default();

        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::SetMenuPageStart(12),
            &profile,
        );

        assert_eq!(update.changes, Default::default());
        assert_eq!(app_state.ui.mode, AppMode::Menu { page_start: 12 });
    }

    #[test]
    fn set_menu_page_start_noop_in_gameplay_mode() {
        let mut controller = test_controller();
        let mut app_state = AppState::default();
        let profile = PresentationProfile::default();

        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::SetMenuPageStart(12),
            &profile,
        );

        assert_eq!(update.changes, Default::default());
        assert_eq!(app_state.ui.mode, AppMode::Gameplay);
    }
}
