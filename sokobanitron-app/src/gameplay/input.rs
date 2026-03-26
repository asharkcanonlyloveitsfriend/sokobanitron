use super::state::GameplayInteractionState;
use crate::app::input::AppInput;
use crate::app::state::{AppOverlay, AppState};
use crate::shared::{MOUSE_POINTER_ID, PointerEvent, PointerGesture, PointerPhase};
use presentation::hit_test::{
    ControlsButtonAction, GameplaySurfaceLayer, GameplaySurfaceModel, GameplaySurfaceTarget,
    LevelSelectSurfaceTarget, gameplay_surface_target_at, level_select_menu_start_for_nav,
};
use presentation::layout::BoardViewport;
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
    surface_width: u32,
    surface_height: u32,
    board_viewport: BoardViewport,
) -> GameplaySurfaceModel<'a> {
    GameplaySurfaceModel {
        layer: gameplay_surface_layer_from_app_state(app_state),
        surface_width,
        surface_height,
        level_count: controller.level_count(),
        current_level: controller.current_level(),
        can_undo: controller.can_undo(),
        can_restart: controller.can_restart(),
        board_viewport,
        board: controller.board(),
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

pub fn gameplay_pointer_tap(
    interaction: &mut GameplayInteractionState,
    surface: &GameplaySurfaceModel<'_>,
    policy: GameplayPolicyContext,
    x: f64,
    y: f64,
) -> AppInput {
    let tap = interaction
        .pointer
        .synthetic_tap(MOUSE_POINTER_ID, x, y, Instant::now());
    interpret_gameplay_gesture(surface, policy, PointerGesture::Tap(tap))
}

pub fn gameplay_pointer_event(
    interaction: &mut GameplayInteractionState,
    surface: &GameplaySurfaceModel<'_>,
    policy: GameplayPolicyContext,
    id: u64,
    phase: PointerPhase,
    x: f64,
    y: f64,
) -> AppInput {
    let Some(gesture) =
        interaction
            .pointer
            .handle_event(PointerEvent::new(id, phase, x, y, Instant::now()))
    else {
        return AppInput::NoOp;
    };
    interpret_gameplay_gesture(surface, policy, gesture)
}

fn interpret_gameplay_gesture(
    surface: &GameplaySurfaceModel<'_>,
    policy: GameplayPolicyContext,
    gesture: PointerGesture,
) -> AppInput {
    let PointerGesture::Tap(tap) = gesture else {
        return AppInput::NoOp;
    };
    let (tap_x, tap_y) = tap.position.as_f64();
    let target = gameplay_surface_target_at(surface, tap_x, tap_y);
    interpret_gameplay_surface_target(surface, policy, target)
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
        gameplay_pointer_tap,
    };
    use crate::app::input::AppInput;
    use crate::app::state::{AppOverlay, AppState};
    use presentation::hit_test::{
        GameplaySurfaceLayer, GameplaySurfaceModel, gameplay_surface_target_at,
    };
    use presentation::layout::fit_board_viewport_for_controls;
    use sokobanitron_gameplay::GameplayController;

    fn test_controller() -> GameplayController {
        let level = "#######\n#@ $. #\n#######".to_string();
        GameplayController::new(vec![level], Some(0))
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
        build_gameplay_surface_model(
            app_state,
            controller,
            670,
            891,
            fit_board_viewport_for_controls(670, 891, controller.board()),
        )
    }

    fn test_policy(controller: &GameplayController, app_state: &AppState) -> GameplayPolicyContext {
        build_gameplay_policy_context(app_state, controller)
    }

    #[test]
    fn board_tap_is_not_misclassified_as_level_select_when_overlay_is_closed() {
        let controller = test_controller();
        let mut interaction = super::GameplayInteractionState::default();
        let app_state = test_app_state();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&controller, &app_state);
        let (x, y, w, h) = surface.board_viewport.cell_to_screen_rect(1, 1);
        let input = gameplay_pointer_tap(
            &mut interaction,
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
        let mut interaction = super::GameplayInteractionState::default();
        let mut app_state = test_app_state();
        app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start: 0 });
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&controller, &app_state);
        let input = gameplay_pointer_tap(&mut interaction, &surface, policy, 12.0, 120.0);

        assert!(matches!(input, AppInput::LevelSelectSelect(_)));
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
