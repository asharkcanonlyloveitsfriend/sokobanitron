use super::view::{GameplayUiState, build_gameplay_visible_window, gameplay_zoom_origin_for_focus};
use crate::app::input::AppInput;
use crate::app::state::{AppOverlay, AppState};
use crate::shared::{
    MOUSE_POINTER_ID, PinchDirection, PinchGesture, PointerEvent, PointerGesture, PointerPhase,
    ScreenPoint,
};
use presentation::hit_test::{
    GameplaySurfaceLayer, GameplaySurfaceModel, GameplaySurfaceTarget, LevelSelectSurfaceTarget,
    LevelSetSelectSurfaceTarget, gameplay_surface_target_at,
    level_select_menu_nav_action_for_swipe, level_select_menu_start_for_nav,
    level_set_select_start_for_nav,
};
use sokobanitron_gameplay::GameplayController;
use std::time::Instant;

const SWIPE_MOVE_THRESHOLD_PX: i32 = 56;

#[derive(Debug, Clone, Copy)]
pub struct GameplayPolicyContext {
    pub allow_enter_editor: bool,
    pub is_gameplay_screen: bool,
}

pub fn build_gameplay_surface_model(
    app_state: &AppState,
    controller: &GameplayController,
) -> GameplaySurfaceModel {
    let board = controller.board();
    let visible_window = build_gameplay_visible_window(&app_state.gameplay, board);
    GameplaySurfaceModel {
        layer: gameplay_surface_layer_from_app_state(app_state),
        surface_width: app_state.gameplay.surface_width,
        surface_height: app_state.gameplay.surface_height,
        level_count: controller.level_count(),
        resume_level: controller.resume_level(),
        level_set_count: app_state.gameplay.level_sets.len(),
        active_level_set: app_state.gameplay.active_level_set,
        can_change_level_set: app_state.gameplay.level_sets.len() > 1,
        board_origin_x: visible_window.board_origin_x,
        board_origin_y: visible_window.board_origin_y,
        board_viewport: visible_window.viewport,
        board: visible_window.board,
    }
}

fn gameplay_surface_layer_from_app_state(app_state: &AppState) -> GameplaySurfaceLayer {
    match app_state.ui.overlay {
        Some(AppOverlay::GameplayMenu) => GameplaySurfaceLayer::Menu,
        Some(AppOverlay::LevelSelect { page_start }) => {
            GameplaySurfaceLayer::LevelSelect { page_start }
        }
        Some(AppOverlay::LevelSetSelect { page_start }) => {
            GameplaySurfaceLayer::LevelSetSelect { page_start }
        }
        _ => GameplaySurfaceLayer::Board,
    }
}

pub fn build_gameplay_policy_context(app_state: &AppState) -> GameplayPolicyContext {
    GameplayPolicyContext {
        allow_enter_editor: app_state.editor_available,
        is_gameplay_screen: app_state.is_gameplay_screen(),
    }
}

pub fn interpret_gameplay_pointer_tap(
    app_state: &mut AppState,
    controller: &GameplayController,
    x: f64,
    y: f64,
) -> AppInput {
    let surface = build_gameplay_surface_model(app_state, controller);
    let policy = build_gameplay_policy_context(app_state);
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
    let policy = build_gameplay_policy_context(app_state);
    gameplay_pointer_event(&mut app_state.gameplay, &surface, policy, id, phase, x, y)
}

pub(crate) fn gameplay_pointer_tap(
    gameplay: &mut GameplayUiState,
    surface: &GameplaySurfaceModel,
    policy: GameplayPolicyContext,
    x: f64,
    y: f64,
) -> AppInput {
    let tap = gameplay
        .interaction
        .touch
        .synthetic_tap(MOUSE_POINTER_ID, x, y, Instant::now());
    interpret_gameplay_gesture(gameplay, surface, policy, PointerGesture::Tap(tap), None)
}

pub(crate) fn gameplay_pointer_event(
    gameplay: &mut GameplayUiState,
    surface: &GameplaySurfaceModel,
    policy: GameplayPolicyContext,
    id: u64,
    phase: PointerPhase,
    x: f64,
    y: f64,
) -> AppInput {
    let event = PointerEvent::new(id, phase, x, y, Instant::now());
    let drag_start = match phase {
        PointerPhase::Ended | PointerPhase::Cancelled => {
            gameplay.interaction.touch.active_start_position()
        }
        PointerPhase::Started | PointerPhase::Moved => None,
    };
    let touch_update = gameplay.interaction.touch.handle_touch_event(event);
    if touch_update.reset_screen_state {
        gameplay.interaction.double_tap.clear();
    }
    if let Some(pinch) = touch_update.pinch {
        gameplay.interaction.double_tap.clear();
        return interpret_gameplay_pinch(gameplay, surface, pinch);
    }
    if touch_update.suppress_screen_gestures {
        gameplay.interaction.double_tap.clear();
        return AppInput::NoOp;
    }

    let Some(gesture) = touch_update.gesture else {
        return AppInput::NoOp;
    };
    interpret_gameplay_gesture(gameplay, surface, policy, gesture, drag_start)
}

fn interpret_gameplay_gesture(
    gameplay: &mut GameplayUiState,
    surface: &GameplaySurfaceModel,
    policy: GameplayPolicyContext,
    gesture: PointerGesture,
    drag_start: Option<ScreenPoint>,
) -> AppInput {
    match gesture {
        PointerGesture::Tap(tap) => {
            let (tap_x, tap_y) = tap.position.as_f64();
            let target = gameplay_surface_target_at(surface, tap_x, tap_y);
            interpret_gameplay_tap(gameplay, surface, policy, target, tap.at)
        }
        PointerGesture::Ended(contact) => {
            gameplay.interaction.double_tap.clear();
            interpret_swipe(
                surface,
                gameplay.viewport.zoomed_in,
                contact.position,
                drag_start,
            )
        }
        PointerGesture::Started(_) => AppInput::NoOp,
        PointerGesture::DragStarted(_)
        | PointerGesture::DragMoved(_)
        | PointerGesture::Cancelled(_) => {
            gameplay.interaction.double_tap.clear();
            AppInput::NoOp
        }
    }
}

fn interpret_gameplay_tap(
    gameplay: &mut GameplayUiState,
    surface: &GameplaySurfaceModel,
    policy: GameplayPolicyContext,
    target: Option<GameplaySurfaceTarget>,
    at: Instant,
) -> AppInput {
    let input = interpret_gameplay_surface_target(surface, policy, target);
    let Some(GameplaySurfaceTarget::BoardCell(cell)) = target else {
        gameplay.interaction.double_tap.clear();
        return input;
    };

    let is_double_tap = gameplay.interaction.double_tap.register_tap(
        cell,
        at,
        gameplay.interaction.double_tap_window,
    );
    if is_double_tap {
        return AppInput::BoardDoubleTap(cell);
    }

    input
}

fn interpret_swipe(
    surface: &GameplaySurfaceModel,
    zoomed_in: bool,
    end: ScreenPoint,
    drag_start: Option<ScreenPoint>,
) -> AppInput {
    let Some(start) = drag_start else {
        return AppInput::NoOp;
    };
    let delta_x = end.x - start.x;
    let delta_y = end.y - start.y;

    if let Some(page_start) = surface.layer.level_select_page_start() {
        let Some(nav) = level_select_menu_nav_action_for_swipe(delta_x, delta_y) else {
            return AppInput::NoOp;
        };
        let page_start = level_select_menu_start_for_nav(
            surface.level_count,
            surface.resume_level,
            page_start,
            nav,
        );
        return AppInput::LevelSelectNavigate { page_start };
    }

    if let Some(page_start) = surface.layer.level_set_select_page_start() {
        let Some(nav) = level_select_menu_nav_action_for_swipe(delta_x, delta_y) else {
            return AppInput::NoOp;
        };
        let page_start = level_set_select_start_for_nav(
            surface.level_set_count,
            surface.active_level_set,
            page_start,
            nav,
        );
        return AppInput::LevelSetSelectNavigate { page_start };
    }

    if matches!(surface.layer, GameplaySurfaceLayer::Board)
        && zoomed_in
        && (delta_x.abs() >= SWIPE_MOVE_THRESHOLD_PX || delta_y.abs() >= SWIPE_MOVE_THRESHOLD_PX)
    {
        return AppInput::GameplaySwipePan { delta_x, delta_y };
    }

    AppInput::NoOp
}

fn interpret_gameplay_surface_target(
    surface: &GameplaySurfaceModel,
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
        Some(GameplaySurfaceTarget::LevelSetButton)
            if matches!(layer, GameplaySurfaceLayer::Menu) =>
        {
            return AppInput::OpenLevelSetSelect;
        }
        Some(GameplaySurfaceTarget::MenuToggle) => {
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
                    surface.resume_level,
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

    if let Some(page_start) = layer.level_set_select_page_start() {
        return match target {
            Some(GameplaySurfaceTarget::LevelSetSelect(LevelSetSelectSurfaceTarget::Navigate(
                nav,
            ))) => {
                let page_start = level_set_select_start_for_nav(
                    surface.level_set_count,
                    surface.active_level_set,
                    page_start,
                    nav,
                );
                AppInput::LevelSetSelectNavigate { page_start }
            }
            Some(GameplaySurfaceTarget::LevelSetSelect(LevelSetSelectSurfaceTarget::LevelSet(
                level_set,
            ))) => AppInput::LevelSetSelectSelect(level_set),
            _ => AppInput::NoOp,
        };
    }

    match target {
        Some(GameplaySurfaceTarget::BoardCell(cell)) => AppInput::BoardTap(cell),
        _ => AppInput::NoOp,
    }
}

fn interpret_gameplay_pinch(
    gameplay: &GameplayUiState,
    surface: &GameplaySurfaceModel,
    pinch: PinchGesture,
) -> AppInput {
    if !matches!(surface.layer, GameplaySurfaceLayer::Board) {
        return AppInput::NoOp;
    }

    match pinch.direction {
        PinchDirection::Out if !gameplay.viewport.zoomed_in => {
            let Some(GameplaySurfaceTarget::BoardCell(cell)) =
                gameplay_surface_target_at(surface, pinch.center.x as f64, pinch.center.y as f64)
            else {
                return AppInput::NoOp;
            };
            let (zoom_origin_x, zoom_origin_y) =
                gameplay_zoom_origin_for_focus(gameplay, &surface.board, cell);
            AppInput::ZoomGameplayIn {
                zoom_origin_x,
                zoom_origin_y,
            }
        }
        PinchDirection::In if gameplay.viewport.zoomed_in => AppInput::ZoomGameplayOut,
        PinchDirection::In | PinchDirection::Out => AppInput::NoOp,
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
    use crate::gameplay::{
        gameplay_zoom_origin_for_focus, set_gameplay_double_tap_window, set_gameplay_touch_slop,
        set_gameplay_zoomed_in,
    };
    use crate::shared::PointerPhase;
    use presentation::hit_test::{
        GameplaySurfaceLayer, GameplaySurfaceModel, gameplay_surface_target_at,
    };
    use sokobanitron_gameplay::{BoardCell, GameplayController};
    use std::{thread, time::Duration};

    fn test_controller() -> GameplayController {
        let level = "#######\n#@ $. #\n#######".to_string();
        GameplayController::new(vec![level], Some(0))
    }

    fn test_controller_with_levels(count: usize) -> GameplayController {
        let level = "#######\n#@ $. #\n#######".to_string();
        GameplayController::new(vec![level; count], Some(0))
    }

    fn large_test_controller() -> GameplayController {
        let level = [
            "############",
            "#@         #",
            "#          #",
            "#    $     #",
            "#          #",
            "#          #",
            "#       .  #",
            "#          #",
            "#          #",
            "############",
        ]
        .join("\n");
        GameplayController::new(vec![level], Some(0))
    }

    fn test_app_state() -> AppState {
        AppState {
            editor_available: true,
            ..AppState::default()
        }
    }

    fn test_surface(controller: &GameplayController, app_state: &AppState) -> GameplaySurfaceModel {
        build_gameplay_surface_model(app_state, controller)
    }

    fn test_policy(app_state: &AppState) -> GameplayPolicyContext {
        build_gameplay_policy_context(app_state)
    }

    fn cell_center(surface: &GameplaySurfaceModel, cell: BoardCell) -> (f64, f64) {
        let (x, y, w, h) = surface.board_viewport.cell_to_screen_rect(cell);
        ((x + (w / 2) as i32) as f64, (y + (h / 2) as i32) as f64)
    }

    #[test]
    fn board_tap_is_not_misclassified_as_level_select_when_overlay_is_closed() {
        let controller = test_controller();
        let app_state = test_app_state();
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);
        let (x, y, w, h) = surface
            .board_viewport
            .cell_to_screen_rect(BoardCell::new(1, 1));
        let input = gameplay_pointer_tap(
            &mut gameplay,
            &surface,
            policy,
            (x + (w / 2) as i32) as f64,
            (y + (h / 2) as i32) as f64,
        );

        assert!(matches!(input, AppInput::BoardTap(_)));
    }

    #[test]
    fn level_select_targets_are_used_when_level_select_is_open() {
        let controller = test_controller();
        let mut app_state = test_app_state();
        app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start: 0 });
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);
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
        let policy = test_policy(&app_state);

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
    fn downward_swipe_on_board_is_noop_when_not_zoomed() {
        let controller = test_controller();
        let app_state = test_app_state();
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);

        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                1,
                PointerPhase::Started,
                140.0,
                160.0,
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
                144.0,
                248.0,
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
                144.0,
                248.0,
            ),
            AppInput::NoOp
        );
    }

    #[test]
    fn zoomed_rightward_swipe_pans_view_left() {
        let controller = large_test_controller();
        let mut app_state = test_app_state();
        app_state.gameplay.surface_width = 320;
        app_state.gameplay.surface_height = 480;
        let board = controller.board();
        let origin =
            gameplay_zoom_origin_for_focus(&app_state.gameplay, board, BoardCell::new(7, 6));
        set_gameplay_zoomed_in(&mut app_state.gameplay, board, origin.0, origin.1);
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);

        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                1,
                PointerPhase::Started,
                120.0,
                220.0,
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
                220.0,
                224.0,
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
                220.0,
                224.0,
            ),
            AppInput::GameplaySwipePan {
                delta_x: 100,
                delta_y: 4,
            }
        );
    }

    #[test]
    fn zoomed_leftward_swipe_pans_view_right() {
        let controller = large_test_controller();
        let mut app_state = test_app_state();
        app_state.gameplay.surface_width = 320;
        app_state.gameplay.surface_height = 480;
        let board = controller.board();
        let origin =
            gameplay_zoom_origin_for_focus(&app_state.gameplay, board, BoardCell::new(7, 6));
        set_gameplay_zoomed_in(&mut app_state.gameplay, board, origin.0, origin.1);
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);

        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                1,
                PointerPhase::Started,
                220.0,
                220.0,
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
                120.0,
                224.0,
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
                120.0,
                224.0,
            ),
            AppInput::GameplaySwipePan {
                delta_x: -100,
                delta_y: 4,
            }
        );
    }

    #[test]
    fn zoomed_diagonal_swipe_pans_view_diagonally() {
        let controller = large_test_controller();
        let mut app_state = test_app_state();
        app_state.gameplay.surface_width = 320;
        app_state.gameplay.surface_height = 480;
        let board = controller.board();
        let origin =
            gameplay_zoom_origin_for_focus(&app_state.gameplay, board, BoardCell::new(7, 6));
        set_gameplay_zoomed_in(&mut app_state.gameplay, board, origin.0, origin.1);
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);

        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                1,
                PointerPhase::Started,
                120.0,
                160.0,
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
                220.0,
                260.0,
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
                220.0,
                260.0,
            ),
            AppInput::GameplaySwipePan {
                delta_x: 100,
                delta_y: 100,
            }
        );
    }

    #[test]
    fn kindle_tap_slop_keeps_noisy_board_touch_as_tap() {
        let controller = test_controller();
        let app_state = test_app_state();
        let mut gameplay = app_state.gameplay.clone();
        set_gameplay_touch_slop(&mut gameplay, 24);
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);
        let (x, y, w, h) = surface
            .board_viewport
            .cell_to_screen_rect(BoardCell::new(1, 1));
        let tap_x = (x + (w / 2) as i32) as f64;
        let tap_y = (y + (h / 2) as i32) as f64;

        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                1,
                PointerPhase::Started,
                tap_x,
                tap_y,
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
                tap_x + 18.0,
                tap_y + 2.0,
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
                tap_x + 18.0,
                tap_y + 2.0,
            ),
            AppInput::BoardTap(BoardCell::new(1, 1))
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

    #[test]
    fn same_cell_double_tap_emits_board_double_tap() {
        let mut controller = test_controller();
        let _ = controller.click_cell_with_outcome(BoardCell::new(2, 1));
        let app_state = test_app_state();
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);
        let (x, y, w, h) = surface
            .board_viewport
            .cell_to_screen_rect(BoardCell::new(2, 1));
        let tap_x = (x + (w / 2) as i32) as f64;
        let tap_y = (y + (h / 2) as i32) as f64;

        assert_eq!(
            gameplay_pointer_tap(&mut gameplay, &surface, policy, tap_x, tap_y),
            AppInput::BoardTap(BoardCell::new(2, 1))
        );
        assert_eq!(
            gameplay_pointer_tap(&mut gameplay, &surface, policy, tap_x, tap_y),
            AppInput::BoardDoubleTap(BoardCell::new(2, 1))
        );
    }

    #[test]
    fn input_layer_emits_board_double_tap_without_gameplay_meaning() {
        let level = "#######\n#@ $ .#\n#######".to_string();
        let mut controller = GameplayController::new(vec![level], Some(0));
        let _ = controller.click_cell_with_outcome(BoardCell::new(3, 1));
        let _ = controller.click_cell_with_outcome(BoardCell::new(4, 1));
        let app_state = test_app_state();
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);
        let (x, y, w, h) = surface
            .board_viewport
            .cell_to_screen_rect(BoardCell::new(4, 1));
        let tap_x = (x + (w / 2) as i32) as f64;
        let tap_y = (y + (h / 2) as i32) as f64;

        assert_eq!(
            gameplay_pointer_tap(&mut gameplay, &surface, policy, tap_x, tap_y),
            AppInput::BoardTap(BoardCell::new(4, 1))
        );
        assert_eq!(
            gameplay_pointer_tap(&mut gameplay, &surface, policy, tap_x, tap_y),
            AppInput::BoardDoubleTap(BoardCell::new(4, 1))
        );
    }

    #[test]
    fn different_cell_second_tap_does_not_emit_board_double_tap() {
        let level = "########\n#@ $   #\n#  $ . #\n########".to_string();
        let mut controller = GameplayController::new(vec![level], Some(0));
        let _ = controller.click_cell_with_outcome(BoardCell::new(3, 1));
        let _ = controller.click_cell_with_outcome(BoardCell::new(4, 1));
        let app_state = test_app_state();
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);
        let (x1, y1, w1, h1) = surface
            .board_viewport
            .cell_to_screen_rect(BoardCell::new(3, 2));
        let tap_x1 = (x1 + (w1 / 2) as i32) as f64;
        let tap_y1 = (y1 + (h1 / 2) as i32) as f64;
        let (x2, y2, w2, h2) = surface
            .board_viewport
            .cell_to_screen_rect(BoardCell::new(4, 1));
        let tap_x2 = (x2 + (w2 / 2) as i32) as f64;
        let tap_y2 = (y2 + (h2 / 2) as i32) as f64;

        assert_eq!(
            gameplay_pointer_tap(&mut gameplay, &surface, policy, tap_x1, tap_y1),
            AppInput::BoardTap(BoardCell::new(3, 2))
        );
        assert_eq!(
            gameplay_pointer_tap(&mut gameplay, &surface, policy, tap_x2, tap_y2),
            AppInput::BoardTap(BoardCell::new(4, 1))
        );
    }

    #[test]
    fn same_cell_taps_outside_window_do_not_emit_board_double_tap() {
        let controller = test_controller();
        let app_state = test_app_state();
        let mut gameplay = app_state.gameplay.clone();
        set_gameplay_double_tap_window(&mut gameplay, Duration::ZERO);
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);
        let (x, y, w, h) = surface
            .board_viewport
            .cell_to_screen_rect(BoardCell::new(1, 1));
        let tap_x = (x + (w / 2) as i32) as f64;
        let tap_y = (y + (h / 2) as i32) as f64;

        assert_eq!(
            gameplay_pointer_tap(&mut gameplay, &surface, policy, tap_x, tap_y),
            AppInput::BoardTap(BoardCell::new(1, 1))
        );
        thread::sleep(Duration::from_millis(1));
        assert_eq!(
            gameplay_pointer_tap(&mut gameplay, &surface, policy, tap_x, tap_y),
            AppInput::BoardTap(BoardCell::new(1, 1))
        );
    }

    #[test]
    fn non_board_tap_resets_double_tap_tracking() {
        let controller = test_controller();
        let app_state = test_app_state();
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);
        let (x, y, w, h) = surface
            .board_viewport
            .cell_to_screen_rect(BoardCell::new(1, 1));
        let tap_x = (x + (w / 2) as i32) as f64;
        let tap_y = (y + (h / 2) as i32) as f64;

        assert_eq!(
            gameplay_pointer_tap(&mut gameplay, &surface, policy, tap_x, tap_y),
            AppInput::BoardTap(BoardCell::new(1, 1))
        );
        assert_eq!(
            gameplay_pointer_tap(&mut gameplay, &surface, policy, 20.0, 20.0),
            AppInput::OpenLevelSelect
        );
        assert_eq!(
            gameplay_pointer_tap(&mut gameplay, &surface, policy, tap_x, tap_y),
            AppInput::BoardTap(BoardCell::new(1, 1))
        );
    }

    #[test]
    fn pinch_out_on_board_emits_zoom_in_action() {
        let controller = large_test_controller();
        let mut app_state = test_app_state();
        app_state.gameplay.surface_width = 320;
        app_state.gameplay.surface_height = 480;
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);
        let focus = BoardCell::new(7, 6);
        let (center_x, center_y) = cell_center(&surface, focus);
        let expected_origin = gameplay_zoom_origin_for_focus(&gameplay, &surface.board, focus);

        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                1,
                PointerPhase::Started,
                center_x - 24.0,
                center_y,
            ),
            AppInput::NoOp
        );
        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                2,
                PointerPhase::Started,
                center_x + 24.0,
                center_y,
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
                center_x - 72.0,
                center_y,
            ),
            AppInput::NoOp
        );
        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                2,
                PointerPhase::Moved,
                center_x + 72.0,
                center_y,
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
                center_x - 72.0,
                center_y,
            ),
            AppInput::ZoomGameplayIn {
                zoom_origin_x: expected_origin.0,
                zoom_origin_y: expected_origin.1,
            }
        );
    }

    #[test]
    fn pinch_in_when_zoomed_emits_zoom_out_action() {
        let controller = large_test_controller();
        let mut app_state = test_app_state();
        app_state.gameplay.surface_width = 320;
        app_state.gameplay.surface_height = 480;
        let board = controller.board();
        let origin =
            gameplay_zoom_origin_for_focus(&app_state.gameplay, board, BoardCell::new(7, 6));
        set_gameplay_zoomed_in(&mut app_state.gameplay, board, origin.0, origin.1);
        let mut gameplay = app_state.gameplay.clone();
        let surface = test_surface(&controller, &app_state);
        let policy = test_policy(&app_state);
        let (center_x, center_y) = cell_center(&surface, BoardCell::new(3, 3));

        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                1,
                PointerPhase::Started,
                center_x - 72.0,
                center_y,
            ),
            AppInput::NoOp
        );
        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                2,
                PointerPhase::Started,
                center_x + 72.0,
                center_y,
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
                center_x - 24.0,
                center_y,
            ),
            AppInput::NoOp
        );
        assert_eq!(
            gameplay_pointer_event(
                &mut gameplay,
                &surface,
                policy,
                2,
                PointerPhase::Moved,
                center_x + 24.0,
                center_y,
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
                center_x - 24.0,
                center_y,
            ),
            AppInput::ZoomGameplayOut
        );
    }
}
