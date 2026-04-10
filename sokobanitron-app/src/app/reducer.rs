use super::action::AppAction;
use super::presentation::{PresentationPlan, build_presentation_plan};
use super::state::{AppOverlay, AppScreen, AppState};
use presentation::layout::{level_select_menu_start_index, level_set_select_start_index};
use sokobanitron_gameplay::{GameplayController, GameplayControllerChanges, GameplayTapEffect};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PersistenceUpdate {
    pub resume_level_changed: Option<usize>,
    pub solved_level: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppUpdate {
    pub changes: GameplayControllerChanges,
    pub persistence: PersistenceUpdate,
    pub level_set_selected: Option<usize>,
    pub gameplay_effect: Option<GameplayTapEffect>,
    pub presentation_plan: Option<PresentationPlan>,
}

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
                app_state.ui.overlay = Some(app_state.ui.screen.default_overlay());
            }
        }
        AppAction::OpenOverlay => {
            app_state.ui.overlay = Some(app_state.ui.screen.default_overlay());
        }
        AppAction::CloseOverlay => {
            app_state.ui.overlay = None;
        }
        AppAction::OpenLevelSelect => {
            if matches!(app_state.ui.screen, AppScreen::Gameplay) {
                let page_start = level_select_menu_start_index(
                    controller.level_count(),
                    controller.current_level(),
                );
                app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start });
            }
        }
        AppAction::OpenLevelSetSelect => {
            if matches!(app_state.ui.screen, AppScreen::Gameplay)
                && app_state.gameplay.level_sets.len() > 1
            {
                let page_start = level_set_select_start_index(
                    app_state.gameplay.level_sets.len(),
                    app_state.gameplay.active_level_set,
                );
                app_state.ui.overlay = Some(AppOverlay::LevelSetSelect { page_start });
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
        AppAction::SetLevelSetSelectPageStart(page_start) => {
            if let Some(AppOverlay::LevelSetSelect {
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
        AppAction::SelectLevelSet(level_set) => {
            if matches!(app_state.ui.screen, AppScreen::Gameplay)
                && level_set < app_state.gameplay.level_sets.len()
            {
                update.level_set_selected = Some(level_set);
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
            apply_board_tap(controller, app_state, &mut update, x, y);
        }
        AppAction::DoubleTapBoardCell { x, y } => {
            apply_board_double_tap(controller, app_state, &mut update, x, y);
        }
        AppAction::NoOp => {}
    }

    if update.persistence.resume_level_changed.is_none() {
        update.persistence.resume_level_changed = update.changes.resume_level_changed;
    }

    update
}

fn apply_board_tap(
    controller: &mut GameplayController,
    app_state: &mut AppState,
    update: &mut AppUpdate,
    x: u32,
    y: u32,
) {
    if !matches!(app_state.ui.screen, AppScreen::Gameplay) || app_state.ui.overlay.is_some() {
        return;
    }

    if controller.board().is_solved() {
        if let Some(next_level) = controller.peek_level(1) {
            update.changes = controller.advance_after_win(next_level);
        }
        return;
    }

    let outcome = controller.click_cell_with_outcome(x, y);
    update.changes = outcome.changes;
    update.gameplay_effect = Some(outcome.effect.clone());
    update.persistence.resume_level_changed = if outcome.started_now {
        Some(controller.current_level())
    } else {
        update.changes.resume_level_changed
    };
    if outcome.became_solved {
        update.persistence.solved_level = Some(controller.current_level());
    }
    update.presentation_plan = Some(build_presentation_plan(&outcome, controller, app_state));
}

fn apply_board_double_tap(
    controller: &mut GameplayController,
    app_state: &mut AppState,
    update: &mut AppUpdate,
    x: u32,
    y: u32,
) {
    if !matches!(app_state.ui.screen, AppScreen::Gameplay) || app_state.ui.overlay.is_some() {
        return;
    }

    if controller.can_restart() && controller.board().player() == Some((x, y)) {
        update.changes = controller.restart_with_changes();
        return;
    }

    if controller.board().is_solved() {
        return;
    }

    if controller.can_undo() && controller.last_box_move_destination() == Some((x, y)) {
        update.changes = controller.undo_with_changes();
        return;
    }

    apply_board_tap(controller, app_state, update, x, y);
}

#[cfg(test)]
mod tests {
    use super::apply_action;
    use crate::app::action::AppAction;
    use crate::app::state::{AppOverlay, AppScreen, AppState};
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
    fn open_level_select_uses_current_browsed_level_for_page_start() {
        let levels = vec!["    ###   \n $$     #@\n $ #...   \n   #######".to_string(); 30];
        let mut controller = GameplayController::new_at_level(levels, 4, Some(18));
        let mut app_state = AppState::default();
        apply_action(&mut controller, &mut app_state, AppAction::OpenLevelSelect);

        assert_eq!(
            app_state.ui.overlay,
            Some(AppOverlay::LevelSelect { page_start: 3 })
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

    #[test]
    fn open_level_set_select_positions_active_set_near_top() {
        let mut controller = test_controller();
        let mut app_state = AppState::default();
        app_state.gameplay.level_sets = (0..30)
            .map(|index| crate::persistence::LevelSetCatalogEntry {
                kind: crate::persistence::LevelSetKind::Imported,
                title: format!("Set {}", index + 1),
                completed_puzzle_count: 0,
                total_puzzle_count: 10,
            })
            .collect();
        app_state.gameplay.active_level_set = Some(18);

        apply_action(
            &mut controller,
            &mut app_state,
            AppAction::OpenLevelSetSelect,
        );

        assert_eq!(
            app_state.ui.overlay,
            Some(AppOverlay::LevelSetSelect { page_start: 17 })
        );
    }

    #[test]
    fn open_level_set_select_requires_more_than_one_set() {
        let mut controller = test_controller();
        let mut app_state = AppState::default();
        app_state.gameplay.level_sets = vec![crate::persistence::LevelSetCatalogEntry {
            kind: crate::persistence::LevelSetKind::Imported,
            title: "Only Set".to_string(),
            completed_puzzle_count: 0,
            total_puzzle_count: 10,
        }];

        apply_action(
            &mut controller,
            &mut app_state,
            AppAction::OpenLevelSetSelect,
        );

        assert_eq!(app_state.ui.overlay, None);
    }

    #[test]
    fn open_level_set_select_clamps_to_last_page_at_end() {
        let mut controller = test_controller();
        let mut app_state = AppState::default();
        app_state.gameplay.level_sets = (0..30)
            .map(|index| crate::persistence::LevelSetCatalogEntry {
                kind: crate::persistence::LevelSetKind::Imported,
                title: format!("Set {}", index + 1),
                completed_puzzle_count: 0,
                total_puzzle_count: 10,
            })
            .collect();
        app_state.gameplay.active_level_set = Some(29);

        apply_action(
            &mut controller,
            &mut app_state,
            AppAction::OpenLevelSetSelect,
        );

        assert_eq!(
            app_state.ui.overlay,
            Some(AppOverlay::LevelSetSelect { page_start: 10 })
        );
    }

    #[test]
    fn select_level_set_records_requested_index_and_closes_overlay() {
        let mut controller = test_controller();
        let mut app_state = AppState::default();
        app_state.gameplay.level_sets = (0..2)
            .map(|index| crate::persistence::LevelSetCatalogEntry {
                kind: crate::persistence::LevelSetKind::Imported,
                title: format!("Set {}", index + 1),
                completed_puzzle_count: 0,
                total_puzzle_count: 10,
            })
            .collect();
        app_state.ui.overlay = Some(AppOverlay::LevelSetSelect { page_start: 0 });

        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::SelectLevelSet(1),
        );

        assert_eq!(update.level_set_selected, Some(1));
        assert_eq!(app_state.ui.overlay, None);
    }

    #[test]
    fn starting_a_level_persists_active_set_even_when_resume_level_was_already_current() {
        let level = "#######\n#@ $. #\n#######".to_string();
        let mut preview_controller =
            GameplayController::new_at_level(vec![level.clone()], 0, Some(0));
        let preview_outcome = preview_controller.click_cell_with_outcome(2, 1);
        assert!(preview_outcome.started_now);

        let mut controller = GameplayController::new_at_level(vec![level], 0, Some(0));
        let mut app_state = AppState::default();

        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::TapBoardCell { x: 2, y: 1 },
        );

        assert_eq!(update.persistence.resume_level_changed, Some(0));
    }

    #[test]
    fn board_tap_action_advances_when_board_is_solved() {
        let solved_level = "###\n#@#\n###".to_string();
        let mut controller =
            GameplayController::new(vec![solved_level.clone(), solved_level], None);
        let mut app_state = AppState::default();

        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::TapBoardCell { x: 1, y: 1 },
        );

        assert_eq!(controller.current_level(), 1);
        assert_eq!(update.persistence.resume_level_changed, Some(1));
    }

    #[test]
    fn board_double_tap_on_started_player_maps_to_restart() {
        let level = "#######\n#@ $. #\n#######".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        let _ = controller.click_cell_with_outcome(2, 1);

        apply_action(
            &mut controller,
            &mut app_state,
            AppAction::DoubleTapBoardCell { x: 2, y: 1 },
        );

        assert_eq!(controller.board().player(), Some((1, 1)));
        assert!(!controller.can_restart());
    }

    #[test]
    fn board_double_tap_on_last_move_destination_maps_to_undo() {
        let level = "#######\n#@ $ .#\n#######".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        let _ = controller.click_cell_with_outcome(3, 1);
        let _ = controller.click_cell_with_outcome(4, 1);

        apply_action(
            &mut controller,
            &mut app_state,
            AppAction::DoubleTapBoardCell { x: 4, y: 1 },
        );

        assert!(controller.board().has_box(3, 1));
        assert_eq!(controller.last_box_move_destination(), None);
    }

    #[test]
    fn board_double_tap_on_void_destination_maps_to_undo() {
        let level = "#######\n# @ . #\n# $ $ #\n#######".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        let _ = controller.click_cell_with_outcome(2, 2);
        let _ = controller.click_cell_with_outcome(2, 3);

        assert_eq!(controller.last_box_move_destination(), Some((2, 3)));
        assert!(!controller.board().has_box(2, 3));
        assert!(!controller.board().is_solved());

        apply_action(
            &mut controller,
            &mut app_state,
            AppAction::DoubleTapBoardCell { x: 2, y: 3 },
        );

        assert!(controller.board().has_box(2, 2));
        assert_eq!(controller.board().player(), Some((2, 1)));
        assert_eq!(controller.last_box_move_destination(), None);
    }

    #[test]
    fn board_double_tap_undo_uses_latest_remaining_destination_after_undo() {
        let level = "########\n#@ $   #\n#  $ . #\n########".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        let _ = controller.click_cell_with_outcome(3, 1);
        let _ = controller.click_cell_with_outcome(4, 1);
        let _ = controller.click_cell_with_outcome(3, 2);
        let _ = controller.click_cell_with_outcome(4, 2);

        assert_eq!(controller.last_box_move_destination(), Some((4, 2)));

        apply_action(
            &mut controller,
            &mut app_state,
            AppAction::DoubleTapBoardCell { x: 4, y: 2 },
        );

        assert_eq!(controller.last_box_move_destination(), Some((4, 1)));

        apply_action(
            &mut controller,
            &mut app_state,
            AppAction::DoubleTapBoardCell { x: 4, y: 1 },
        );

        assert!(controller.board().has_box(3, 1));
        assert!(controller.board().has_box(3, 2));
        assert_eq!(controller.last_box_move_destination(), None);
    }

    #[test]
    fn board_double_tap_on_solved_player_still_maps_to_restart() {
        let level = "#####\n# @ #\n# $.#\n#####".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        let _ = controller.click_cell_with_outcome(2, 2);
        let _ = controller.click_cell_with_outcome(3, 2);

        assert!(controller.board().is_solved());

        apply_action(
            &mut controller,
            &mut app_state,
            AppAction::DoubleTapBoardCell { x: 2, y: 2 },
        );

        assert!(
            !controller.board().is_solved(),
            "restart should leave solved state"
        );
        assert_eq!(controller.board().player(), Some((2, 1)));
        assert!(controller.board().has_box(2, 2));
        assert_eq!(controller.last_box_move_destination(), None);
    }

    #[test]
    fn board_double_tap_on_solved_last_destination_is_noop() {
        let level = "#####\n# @ #\n# $.#\n#####".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        let _ = controller.click_cell_with_outcome(2, 2);
        let _ = controller.click_cell_with_outcome(3, 2);

        assert!(controller.board().is_solved());
        assert_eq!(controller.last_box_move_destination(), Some((3, 2)));

        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::DoubleTapBoardCell { x: 3, y: 2 },
        );

        assert!(controller.board().is_solved());
        assert!(controller.board().has_box(3, 2));
        assert_eq!(controller.last_box_move_destination(), Some((3, 2)));
        assert_eq!(update.changes, Default::default());
        assert!(update.presentation_plan.is_none());
    }

    #[test]
    fn board_double_tap_on_non_last_box_falls_back_to_board_tap() {
        let level = "########\n#@ $   #\n#  $ . #\n########".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        let _ = controller.click_cell_with_outcome(3, 1);
        let _ = controller.click_cell_with_outcome(4, 1);
        let _ = controller.click_cell_with_outcome(3, 2);

        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::DoubleTapBoardCell { x: 3, y: 2 },
        );

        assert_eq!(controller.board().selected_box(), None);
        assert!(update.presentation_plan.is_some());
    }

    #[test]
    fn board_double_tap_is_ignored_when_overlay_is_open() {
        let level = "#######\n#@ $. #\n#######".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::GameplayMenu);
        let _ = controller.click_cell_with_outcome(2, 1);

        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::DoubleTapBoardCell { x: 2, y: 1 },
        );

        assert_eq!(controller.board().player(), Some((2, 1)));
        assert_eq!(update.changes, Default::default());
        assert!(update.presentation_plan.is_none());
    }

    #[test]
    fn board_double_tap_is_ignored_outside_gameplay_screen() {
        let level = "#######\n#@ $. #\n#######".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        let _ = controller.click_cell_with_outcome(2, 1);

        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::DoubleTapBoardCell { x: 2, y: 1 },
        );

        assert_eq!(controller.board().player(), Some((2, 1)));
        assert_eq!(update.changes, Default::default());
        assert!(update.presentation_plan.is_none());
    }

    #[test]
    fn board_double_tap_is_noop_when_board_is_solved() {
        let solved_level = "###\n#@#\n###".to_string();
        let mut controller = GameplayController::new(vec![solved_level], None);
        let mut app_state = AppState::default();

        let update = apply_action(
            &mut controller,
            &mut app_state,
            AppAction::DoubleTapBoardCell { x: 1, y: 1 },
        );

        assert_eq!(controller.current_level(), 0);
        assert_eq!(update.changes, Default::default());
        assert!(update.presentation_plan.is_none());
    }

    #[test]
    fn undo_action_works_when_history_exists_after_level_is_solved() {
        let level = "#####\n# @ #\n# $.#\n#####".to_string();
        let mut controller = GameplayController::new(vec![level], None);
        let mut app_state = AppState::default();
        let _ = controller.click_cell_with_outcome(2, 2);
        let _ = controller.click_cell_with_outcome(3, 2);

        assert!(controller.board().is_solved());

        apply_action(&mut controller, &mut app_state, AppAction::Undo);

        assert!(!controller.board().is_solved());
        assert!(controller.board().has_box(2, 2));
        assert_eq!(controller.last_box_move_destination(), None);
    }
}
