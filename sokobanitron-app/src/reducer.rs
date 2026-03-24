use crate::action::AppAction;
use crate::app_state::AppState;
use crate::present::{PresentationPlan, build_presentation_plan};
use crate::ui_state::{AppOverlay, AppScreen};
use sokobanitron_gameplay::{GameplayController, GameplayControllerChanges};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppUpdate {
    pub changes: GameplayControllerChanges,
    pub presentation_plan: Option<PresentationPlan>,
}

const LEVEL_SELECT_PAGE_SIZE: usize = 4;

pub fn apply_action(
    controller: &mut GameplayController,
    app_state: &mut AppState,
    action: AppAction,
) -> AppUpdate {
    let mut update = AppUpdate::default();

    match action {
        AppAction::Restart => {
            if matches!(app_state.ui.screen, AppScreen::Gameplay) {
                update.changes = controller.restart_with_changes();
            }
        }
        AppAction::Undo => {
            if matches!(app_state.ui.screen, AppScreen::Gameplay) {
                update.changes = controller.undo_with_changes();
            }
        }
        AppAction::ToggleOverlay => {
            if app_state.ui.overlay.is_some() {
                app_state.ui.overlay = None;
            } else {
                app_state.ui.overlay = Some(match app_state.ui.screen {
                    AppScreen::Gameplay => AppOverlay::GameplayMenu,
                    AppScreen::Editor => AppOverlay::EditorMenu,
                });
            }
        }
        AppAction::OpenOverlay => {
            app_state.ui.overlay = Some(match app_state.ui.screen {
                AppScreen::Gameplay => AppOverlay::GameplayMenu,
                AppScreen::Editor => AppOverlay::EditorMenu,
            });
        }
        AppAction::CloseOverlay => {
            app_state.ui.overlay = None;
        }
        AppAction::OpenLevelSelect => {
            if matches!(app_state.ui.screen, AppScreen::Gameplay) {
                let page_start =
                    level_select_start_index(controller.level_count(), controller.current_level());
                app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start });
            }
        }
        AppAction::EnterEditorMode => {
            app_state.ui.screen = AppScreen::Editor;
            app_state.ui.overlay = None;
        }
        AppAction::EnterGameplayMode => {
            app_state.ui.screen = AppScreen::Gameplay;
            app_state.ui.overlay = None;
        }
        AppAction::SetLevelSelectPageStart(page_start) => {
            if let Some(AppOverlay::LevelSelect {
                page_start: current_page_start,
            }) = &mut app_state.ui.overlay
            {
                *current_page_start = page_start;
            }
        }
        AppAction::SelectLevel(level) => {
            if matches!(app_state.ui.screen, AppScreen::Gameplay) {
                update.changes = controller.jump_to_level(level);
                app_state.ui.overlay = None;
            }
        }
        AppAction::AdvanceAfterSolved => {
            if matches!(app_state.ui.screen, AppScreen::Gameplay)
                && let Some(next_level) = controller.peek_level(1)
            {
                update.changes = controller.advance_after_win(next_level);
            }
        }
        AppAction::TapBoardCell { x, y } => {
            if matches!(app_state.ui.screen, AppScreen::Gameplay) && app_state.ui.overlay.is_none()
            {
                let outcome = controller.click_cell_with_outcome(x, y);
                update.changes = outcome.changes;
                update.presentation_plan =
                    Some(build_presentation_plan(&outcome, controller, app_state));
            }
        }
        AppAction::NoOp => {}
    }

    update
}

fn level_select_start_index(level_count: usize, current_level: usize) -> usize {
    if level_count <= LEVEL_SELECT_PAGE_SIZE || current_level == 0 {
        0
    } else if current_level >= level_count.saturating_sub(1) {
        level_count.saturating_sub(LEVEL_SELECT_PAGE_SIZE)
    } else {
        current_level.saturating_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::apply_action;
    use crate::{AppAction, AppOverlay, AppScreen, AppState};
    use sokobanitron_gameplay::GameplayController;

    fn test_controller() -> GameplayController {
        let level = "    ###   \n $$     #@\n $ #...   \n   #######".to_string();
        GameplayController::new(vec![level], None)
    }

    #[test]
    fn set_level_select_page_start_updates_level_select_overlay() {
        let mut controller = test_controller();
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start: 0 });
        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::SetLevelSelectPageStart(12),
        );

        assert_eq!(update.changes, Default::default());
        assert_eq!(
            app_state.ui.overlay,
            Some(AppOverlay::LevelSelect { page_start: 12 })
        );
    }

    #[test]
    fn set_level_select_page_start_noop_when_not_level_select_overlay() {
        let mut controller = test_controller();
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        app_state.ui.overlay = Some(AppOverlay::EditorMenu);
        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::SetLevelSelectPageStart(12),
        );

        assert_eq!(update.changes, Default::default());
        assert_eq!(app_state.ui.overlay, Some(AppOverlay::EditorMenu));
    }

    #[test]
    fn toggle_overlay_opens_gameplay_menu_overlay() {
        let mut controller = test_controller();
        let mut app_state = AppState::default();
        apply_action(&mut controller, &mut app_state, AppAction::ToggleOverlay);

        assert_eq!(app_state.ui.overlay, Some(AppOverlay::GameplayMenu));
    }

    #[test]
    fn open_level_select_sets_level_select_overlay() {
        let mut controller = test_controller();
        let mut app_state = AppState::default();
        apply_action(&mut controller, &mut app_state, AppAction::OpenLevelSelect);

        assert_eq!(
            app_state.ui.overlay,
            Some(AppOverlay::LevelSelect { page_start: 0 })
        );
    }

    #[test]
    fn open_level_select_replaces_existing_gameplay_menu_overlay() {
        let mut controller = test_controller();
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::GameplayMenu);
        apply_action(&mut controller, &mut app_state, AppAction::OpenLevelSelect);

        assert_eq!(
            app_state.ui.overlay,
            Some(AppOverlay::LevelSelect { page_start: 0 })
        );
    }

    #[test]
    fn open_level_select_positions_current_level_in_top_right_when_possible() {
        let levels = vec!["    ###   \n $$     #@\n $ #...   \n   #######".to_string(); 30];
        let mut controller = GameplayController::new(levels, Some(18));
        let mut app_state = AppState::default();
        apply_action(&mut controller, &mut app_state, AppAction::OpenLevelSelect);

        assert_eq!(
            app_state.ui.overlay,
            Some(AppOverlay::LevelSelect { page_start: 17 })
        );
    }

    #[test]
    fn open_level_select_clamps_to_last_page_at_end() {
        let levels = vec!["    ###   \n $$     #@\n $ #...   \n   #######".to_string(); 30];
        let mut controller = GameplayController::new(levels, Some(29));
        let mut app_state = AppState::default();
        apply_action(&mut controller, &mut app_state, AppAction::OpenLevelSelect);

        assert_eq!(
            app_state.ui.overlay,
            Some(AppOverlay::LevelSelect { page_start: 26 })
        );
    }
}
