use super::view::{GameplayUiState, build_gameplay_viewport};
use crate::app::input::AppInput;
use crate::app::state::{AppOverlay, AppState};
use crate::shared::{MOUSE_POINTER_ID, PointerEvent, PointerGesture, PointerPhase, ScreenPoint};
use presentation::hit_test::{
    ControlsButtonAction, GameplaySurfaceLayer, GameplaySurfaceModel, GameplaySurfaceTarget,
    LevelSelectSurfaceTarget, gameplay_surface_target_at, level_select_menu_nav_action_for_swipe,
    level_select_menu_start_for_nav,
};
use sokobanitron_gameplay::GameplayController;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct GameplayPolicyContext {
    pub allow_enter_editor: bool,
    pub is_gameplay_screen: bool,
    pub is_solved: bool,
}

pub fn build_gameplay_surface_model<'a>(
    app_state: &AppState,
    controller: &'a GameplayController,
) -> GameplaySurfaceModel<'a> {
    let board = controller.board();
    GameplaySurfaceModel {
        layer: gameplay_surface_layer_from_app_state(app_state),
        surface_width: app_state.gameplay.surface_width,
        surface_height: app_state.gameplay.surface_height,
        level_count: controller.level_count(),
        current_level: controller.current_level(),
        can_undo: controller.can_undo(),
        can_restart: controller.can_restart(),
        board_viewport: build_gameplay_viewport(&app_state.gameplay, board),
        board,
    }
}

fn gameplay_surface_layer_from_app_state(app_state: &AppState) -> GameplaySurfaceLayer {
    match app_state.ui.overlay {
        Some(AppOverlay::GameplayMenu) => GameplaySurfaceLayer::Menu,
        Some(AppOverlay::LevelSelect { page_start }) => {
            GameplaySurfaceLayer::LevelSelect { page_start }
        }
        _ => GameplaySurfaceLayer::Board,
    }
}

pub fn build_gameplay_policy_context(
    app_state: &AppState,
    controller: &GameplayController,
) -> GameplayPolicyContext {
    GameplayPolicyContext {
        allow_enter_editor: app_state.editor_available,
        is_gameplay_screen: app_state.is_gameplay_screen(),
        is_solved: controller.board().is_solved(),
    }
}

pub fn interpret_gameplay_pointer_tap(
    app_state: &mut AppState,
    controller: &GameplayController,
    x: f64,
    y: f64,
) -> AppInput {
    let surface = build_gameplay_surface_model(app_state, controller);
    let policy = build_gameplay_policy_context(app_state, controller);
    gameplay_pointer_tap(&mut app_state.gameplay, &surface, policy, x, y)
}

pub fn interpret_gameplay_pointer_event(
    app_state: &mut AppState,
    controller: &GameplayController,
    id: u64,
    phase: PointerPhase,
    x: f64,
    y: f64,
) -> AppInput {
    let surface = build_gameplay_surface_model(app_state, controller);
    let policy = build_gameplay_policy_context(app_state, controller);
    gameplay_pointer_event(&mut app_state.gameplay, &surface, policy, id, phase, x, y)
}

pub(crate) fn gameplay_pointer_tap(
    gameplay: &mut GameplayUiState,
    surface: &GameplaySurfaceModel<'_>,
    policy: GameplayPolicyContext,
    x: f64,
    y: f64,
) -> AppInput {
    let tap = gameplay
        .interaction
        .pointer
        .synthetic_tap(MOUSE_POINTER_ID, x, y, Instant::now());
    interpret_gameplay_gesture(surface, policy, PointerGesture::Tap(tap), None)
}

pub(crate) fn gameplay_pointer_event(
    gameplay: &mut GameplayUiState,
    surface: &GameplaySurfaceModel<'_>,
    policy: GameplayPolicyContext,
    id: u64,
    phase: PointerPhase,
    x: f64,
    y: f64,
) -> AppInput {
    let drag_start = match phase {
        PointerPhase::Ended | PointerPhase::Cancelled => {
            gameplay.interaction.pointer.active_start_position()
        }
        PointerPhase::Started | PointerPhase::Moved => None,
    };
    let Some(gesture) = gameplay.interaction.pointer.handle_event(PointerEvent::new(
        id,
        phase,
        x,
        y,
        Instant::now(),
    )) else {
        return AppInput::NoOp;
    };
    interpret_gameplay_gesture(surface, policy, gesture, drag_start)
}

fn interpret_gameplay_gesture(
    surface: &GameplaySurfaceModel<'_>,
    policy: GameplayPolicyContext,
    gesture: PointerGesture,
    drag_start: Option<ScreenPoint>,
) -> AppInput {
    match gesture {
        PointerGesture::Tap(tap) => {
            let (tap_x, tap_y) = tap.position.as_f64();
            let target = gameplay_surface_target_at(surface, tap_x, tap_y);
            interpret_gameplay_surface_target(surface, policy, target)
        }
        PointerGesture::Ended(contact) => {
            interpret_level_select_swipe(surface, contact.position, drag_start)
        }
        PointerGesture::Started(_)
        | PointerGesture::DragStarted(_)
        | PointerGesture::DragMoved(_)
        | PointerGesture::Cancelled(_) => AppInput::NoOp,
    }
}

fn interpret_level_select_swipe(
    surface: &GameplaySurfaceModel<'_>,
    end: ScreenPoint,
    drag_start: Option<ScreenPoint>,
) -> AppInput {
    let Some(page_start) = surface.layer.level_select_page_start() else {
        return AppInput::NoOp;
    };
    let Some(start) = drag_start else {
        return AppInput::NoOp;
    };
    let Some(nav) = level_select_menu_nav_action_for_swipe(end.x - start.x, end.y - start.y) else {
        return AppInput::NoOp;
    };
    let page_start = level_select_menu_start_for_nav(
        surface.level_count,
        surface.current_level,
        page_start,
        nav,
    );
    AppInput::LevelSelectNavigate { page_start }
}

fn interpret_gameplay_surface_target(
    surface: &GameplaySurfaceModel<'_>,
    policy: GameplayPolicyContext,
    target: Option<GameplaySurfaceTarget>,
) -> AppInput {
    let layer = surface.layer;

    if policy.allow_enter_editor
        && matches!(layer, GameplaySurfaceLayer::Menu)
        && matches!(target, Some(GameplaySurfaceTarget::OverlayPrimaryAction))
    {
        return AppInput::EnterEditorMode;
    }

    if !policy.is_gameplay_screen {
        return AppInput::NoOp;
    }

    match target {
        Some(GameplaySurfaceTarget::LevelButton) if !layer.is_overlay_open() => {
            return AppInput::OpenLevelSelect;
        }
        Some(GameplaySurfaceTarget::Control(ControlsButtonAction::Restart))
            if !layer.is_overlay_open() =>
        {
            return AppInput::ControlRestart;
        }
        Some(GameplaySurfaceTarget::Control(ControlsButtonAction::Undo))
            if !layer.is_overlay_open() =>
        {
            return AppInput::ControlUndo;
        }
        Some(GameplaySurfaceTarget::Control(ControlsButtonAction::ShowMenu)) => {
            return AppInput::OverlayToggle;
        }
        _ => {}
    }

    if matches!(layer, GameplaySurfaceLayer::Menu) {
        return AppInput::NoOp;
    }

    if let Some(page_start) = layer.level_select_page_start() {
        return match target {
            Some(GameplaySurfaceTarget::LevelSelect(LevelSelectSurfaceTarget::Navigate(nav))) => {
                let page_start = level_select_menu_start_for_nav(
                    surface.level_count,
                    surface.current_level,
                    page_start,
                    nav,
                );
                AppInput::LevelSelectNavigate { page_start }
            }
            Some(GameplaySurfaceTarget::LevelSelect(LevelSelectSurfaceTarget::Level(level))) => {
                AppInput::LevelSelectSelect(level)
            }
            _ => AppInput::NoOp,
        };
    }

    if policy.is_solved {
        return AppInput::SolvedAdvance;
    }

    match target {
        Some(GameplaySurfaceTarget::BoardCell { x, y }) => AppInput::BoardTap { x, y },
        _ => AppInput::NoOp,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GameplayPolicyContext, build_gameplay_policy_context, build_gameplay_surface_model,
        gameplay_pointer_event, gameplay_pointer_tap,
    };
    use crate::app::input::AppInput;
    use crate::app::state::{AppOverlay, AppState};
    use crate::shared::PointerPhase;
    use presentation::hit_test::{
        GameplaySurfaceLayer, GameplaySurfaceModel, gameplay_surface_target_at,
    };
    use sokobanitron_gameplay::GameplayController;

    fn test_controller() -> GameplayController {
        let level = "#######\n#@ $. #\n#######".to_string();
        GameplayController::new(vec![level], Some(0))
    }

    fn test_controller_with_levels(count: usize) -> GameplayController {
        let level = "#######\n#@ $. #\n#######".to_string();
        GameplayController::new(vec![level; count], Some(0))
    }

    fn test_app_state() -> AppState {
        AppState {
            editor_available: true,
            ..AppState::default()
        }
    }

    fn test_surface<'a>(
        controller: &'a GameplayController,
        app_state: &AppState,
    ) -> GameplaySurfaceModel<'a> {
        build_gameplay_surface_model(app_state, controller)
    }

    fn test_policy(controller: &GameplayController, app_state: &AppState) -> GameplayPolicyContext {
        build_gameplay_policy_context(app_state, controller)
    }

    #[test]
    fn board_tap_is_not_misclassified_as_level_select_when_overlay_is_closed() {
        let controller = test_controller();
        let app_state = test_app_state();
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&controller, &app_state);
        let (x, y, w, h) = surface.board_viewport.cell_to_screen_rect(1, 1);
        let input = gameplay_pointer_tap(
            &mut gameplay,
            &surface,
            policy,
            (x + (w / 2) as i32) as f64,
            (y + (h / 2) as i32) as f64,
        );

        assert!(matches!(input, AppInput::BoardTap { .. }));
    }

    #[test]
    fn level_select_targets_are_used_when_level_select_is_open() {
        let controller = test_controller();
        let mut app_state = test_app_state();
        app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start: 0 });
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&controller, &app_state);
        let input = gameplay_pointer_tap(&mut gameplay, &surface, policy, 12.0, 120.0);

        assert!(matches!(input, AppInput::LevelSelectSelect(_)));
    }

    #[test]
    fn vertical_swipe_pages_level_select() {
        let controller = test_controller_with_levels(12);
        let mut app_state = test_app_state();
        app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start: 4 });
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&controller, &app_state);

        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                1,
                PointerPhase::Started,
                120.0,
                480.0,
            ),
            AppInput::NoOp
        );
        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                1,
                PointerPhase::Moved,
                124.0,
                392.0,
            ),
            AppInput::NoOp
        );

        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                1,
                PointerPhase::Ended,
                124.0,
                392.0,
            ),
            AppInput::LevelSelectNavigate { page_start: 8 }
        );
    }

    #[test]
    fn surface_builder_reflects_level_select_overlay_state() {
        let controller = test_controller();
        let mut app_state = test_app_state();
        app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start: 3 });

        let surface = test_surface(&controller, &app_state);

        assert_eq!(
            surface.layer,
            GameplaySurfaceLayer::LevelSelect { page_start: 3 }
        );
    }

    #[test]
    fn no_hit_returns_none_for_surface_hit_testing() {
        let controller = test_controller();
        let app_state = test_app_state();
        let surface = test_surface(&controller, &app_state);

        assert_eq!(gameplay_surface_target_at(&surface, -1.0, -1.0), None);
    }
}
