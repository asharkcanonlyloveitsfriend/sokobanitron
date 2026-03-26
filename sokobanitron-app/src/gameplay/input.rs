use super::state::GameplayInteractionState;
use crate::app::input::AppInput;
use crate::shared::{MOUSE_POINTER_ID, PointerEvent, PointerGesture, PointerPhase};
use presentation::hit_test::{
    ControlsButtonAction, MenuNavAction, controls_button_action_at,
    level_select_menu_nav_action_at, level_select_menu_start_for_nav, level_select_menu_target_at,
    overlay_primary_action_button_contains,
};
use presentation::layout::{BoardViewport, top_left_level_button_rect};
use sokobanitron_gameplay::BoardView;
use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct GameplayInputContext<'a> {
    pub allow_enter_editor: bool,
    pub is_gameplay_screen: bool,
    pub is_gameplay_menu_open: bool,
    pub is_level_select_open: bool,
    pub is_overlay_open: bool,
    pub surface_width: u32,
    pub surface_height: u32,
    pub level_count: usize,
    pub current_level: usize,
    pub current_level_select_page_start: usize,
    pub can_undo: bool,
    pub can_restart: bool,
    pub is_solved: bool,
    pub board_viewport: BoardViewport,
    pub board: &'a BoardView,
}

pub fn gameplay_pointer_tap(
    interaction: &mut GameplayInteractionState,
    context: GameplayInputContext<'_>,
    x: f64,
    y: f64,
) -> AppInput {
    let tap = interaction
        .pointer
        .synthetic_tap(MOUSE_POINTER_ID, x, y, Instant::now());
    interpret_gameplay_gesture(context, PointerGesture::Tap(tap))
}

pub fn gameplay_pointer_event(
    interaction: &mut GameplayInteractionState,
    context: GameplayInputContext<'_>,
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
    interpret_gameplay_gesture(context, gesture)
}

fn interpret_gameplay_gesture(
    context: GameplayInputContext<'_>,
    gesture: PointerGesture,
) -> AppInput {
    let PointerGesture::Tap(tap) = gesture else {
        return AppInput::NoOp;
    };
    let (tap_x, tap_y) = tap.position.as_f64();
    let target = classify_gameplay_hit_target(context, tap_x, tap_y);
    interpret_gameplay_target(context, target)
}

fn interpret_gameplay_target(
    context: GameplayInputContext<'_>,
    target: GameplayHitTarget,
) -> AppInput {
    if context.allow_enter_editor
        && context.is_gameplay_menu_open
        && matches!(target, GameplayHitTarget::OverlayPrimaryAction)
    {
        return AppInput::EnterEditorMode;
    }

    if !context.is_gameplay_screen {
        return AppInput::NoOp;
    }

    match target {
        GameplayHitTarget::TopLeftLevelButton if !context.is_overlay_open => {
            return AppInput::OpenLevelSelect;
        }
        GameplayHitTarget::Controls(ControlsButtonAction::Restart) if !context.is_overlay_open => {
            return AppInput::ControlRestart;
        }
        GameplayHitTarget::Controls(ControlsButtonAction::Undo) if !context.is_overlay_open => {
            return AppInput::ControlUndo;
        }
        GameplayHitTarget::Controls(ControlsButtonAction::ShowMenu) => {
            return AppInput::OverlayToggle;
        }
        _ => {}
    }

    if context.is_gameplay_menu_open {
        return AppInput::NoOp;
    }

    if context.is_level_select_open {
        return match target {
            GameplayHitTarget::LevelSelectNav(nav) => {
                let page_start = level_select_menu_start_for_nav(
                    context.level_count,
                    context.current_level,
                    context.current_level_select_page_start,
                    nav,
                );
                AppInput::LevelSelectNavigate { page_start }
            }
            GameplayHitTarget::LevelSelectLevel(level) => AppInput::LevelSelectSelect(level),
            _ => AppInput::NoOp,
        };
    }

    if context.is_solved {
        return AppInput::SolvedAdvance;
    }

    match target {
        GameplayHitTarget::BoardCell { x, y } => AppInput::BoardTap { x, y },
        _ => AppInput::NoOp,
    }
}

fn classify_gameplay_hit_target(
    context: GameplayInputContext<'_>,
    tap_x: f64,
    tap_y: f64,
) -> GameplayHitTarget {
    if overlay_primary_action_button_contains(
        tap_x,
        tap_y,
        context.surface_width,
        context.surface_height,
    ) {
        return GameplayHitTarget::OverlayPrimaryAction;
    }
    if top_left_level_button_rect().contains(tap_x, tap_y) {
        return GameplayHitTarget::TopLeftLevelButton;
    }
    if let Some(action) = controls_button_action_at(
        tap_x,
        tap_y,
        context.surface_width,
        context.surface_height,
        context.can_undo,
        context.can_restart,
    ) {
        return GameplayHitTarget::Controls(action);
    }
    if context.is_level_select_open {
        if let Some(nav_action) = level_select_menu_nav_action_at(
            tap_x,
            tap_y,
            context.surface_width,
            context.surface_height,
            context.level_count,
            context.current_level,
            context.current_level_select_page_start,
        ) {
            return GameplayHitTarget::LevelSelectNav(nav_action);
        }
        if let Some(level) = level_select_menu_target_at(
            tap_x,
            tap_y,
            context.surface_width,
            context.surface_height,
            context.level_count,
            context.current_level_select_page_start,
        ) {
            return GameplayHitTarget::LevelSelectLevel(level);
        }
    }
    if let Some((x, y)) = context
        .board_viewport
        .screen_to_cell(tap_x, tap_y, context.board)
    {
        return GameplayHitTarget::BoardCell { x, y };
    }
    GameplayHitTarget::Background
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameplayHitTarget {
    OverlayPrimaryAction,
    TopLeftLevelButton,
    Controls(ControlsButtonAction),
    LevelSelectNav(MenuNavAction),
    LevelSelectLevel(usize),
    BoardCell { x: u32, y: u32 },
    Background,
}

#[cfg(test)]
mod tests {
    use super::{GameplayInputContext, gameplay_pointer_tap};
    use crate::app::input::AppInput;
    use presentation::layout::fit_board_viewport_for_controls;
    use sokobanitron_gameplay::GameplayController;

    fn test_controller() -> GameplayController {
        let level = "#######\n#@ $. #\n#######".to_string();
        GameplayController::new(vec![level], Some(0))
    }

    fn test_context<'a>(controller: &'a GameplayController) -> GameplayInputContext<'a> {
        let board = controller.board();
        GameplayInputContext {
            allow_enter_editor: true,
            is_gameplay_screen: true,
            is_gameplay_menu_open: false,
            is_level_select_open: false,
            is_overlay_open: false,
            surface_width: 670,
            surface_height: 891,
            level_count: controller.level_count(),
            current_level: controller.current_level(),
            current_level_select_page_start: 0,
            can_undo: controller.can_undo(),
            can_restart: controller.can_restart(),
            is_solved: board.is_solved(),
            board_viewport: fit_board_viewport_for_controls(670, 891, board),
            board,
        }
    }

    #[test]
    fn board_tap_is_not_misclassified_as_level_select_when_overlay_is_closed() {
        let controller = test_controller();
        let mut interaction = super::GameplayInteractionState::default();
        let context = test_context(&controller);
        let (x, y, w, h) = context.board_viewport.cell_to_screen_rect(1, 1);
        let input = gameplay_pointer_tap(
            &mut interaction,
            context,
            (x + (w / 2) as i32) as f64,
            (y + (h / 2) as i32) as f64,
        );

        assert!(matches!(input, AppInput::BoardTap { .. }));
    }

    #[test]
    fn level_select_targets_are_used_when_level_select_is_open() {
        let controller = test_controller();
        let mut interaction = super::GameplayInteractionState::default();
        let mut context = test_context(&controller);
        context.is_level_select_open = true;
        context.is_overlay_open = true;
        let input = gameplay_pointer_tap(&mut interaction, context, 12.0, 120.0);

        assert!(matches!(input, AppInput::LevelSelectSelect(_)));
    }
}
