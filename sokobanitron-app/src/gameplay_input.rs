use crate::AppInput;
use renderer::{
    BoardViewport, ControlsButtonAction, controls_button_action_at,
    level_select_menu_nav_action_at, level_select_menu_start_for_nav, level_select_menu_target_at,
    overlay_primary_action_button_contains, top_left_level_button_rect,
};
use sokobanitron_gameplay::BoardView;

#[derive(Debug, Clone, Copy)]
pub struct GameplayTapContext<'a> {
    pub allow_enter_editor: bool,
    pub is_gameplay_screen: bool,
    pub is_gameplay_menu_open: bool,
    pub is_level_select_open: bool,
    pub is_overlay_open: bool,
    pub surface_width: u32,
    pub surface_height: u32,
    pub tap_x: f64,
    pub tap_y: f64,
    pub level_count: usize,
    pub current_level: usize,
    pub current_level_select_page_start: usize,
    pub can_undo: bool,
    pub can_restart: bool,
    pub is_solved: bool,
    pub board_viewport: BoardViewport,
    pub board: &'a BoardView,
}

pub fn interpret_gameplay_tap(context: GameplayTapContext<'_>) -> AppInput {
    if context.allow_enter_editor
        && context.is_gameplay_menu_open
        && overlay_primary_action_button_contains(
            context.tap_x,
            context.tap_y,
            context.surface_width,
            context.surface_height,
        )
    {
        return AppInput::EnterEditorMode;
    }

    if !context.is_gameplay_screen {
        return AppInput::NoOp;
    }

    if !context.is_overlay_open
        && top_left_level_button_rect().contains(context.tap_x, context.tap_y)
    {
        return AppInput::OpenLevelSelect;
    }

    if let Some(action) = controls_button_action_at(
        context.tap_x,
        context.tap_y,
        context.surface_width,
        context.surface_height,
        context.can_undo,
        context.can_restart,
    ) {
        match action {
            ControlsButtonAction::Restart if !context.is_overlay_open => {
                return AppInput::ControlRestart;
            }
            ControlsButtonAction::Undo if !context.is_overlay_open => {
                return AppInput::ControlUndo;
            }
            ControlsButtonAction::ShowMenu => return AppInput::OverlayToggle,
            _ => {}
        }
    }

    if context.is_gameplay_menu_open {
        return AppInput::NoOp;
    }

    if context.is_level_select_open {
        if let Some(nav_action) = level_select_menu_nav_action_at(
            context.tap_x,
            context.tap_y,
            context.surface_width,
            context.surface_height,
            context.level_count,
            context.current_level,
            context.current_level_select_page_start,
        ) {
            let page_start = level_select_menu_start_for_nav(
                context.level_count,
                context.current_level,
                context.current_level_select_page_start,
                nav_action,
            );
            return AppInput::LevelSelectNavigate { page_start };
        }

        if let Some(level) = level_select_menu_target_at(
            context.tap_x,
            context.tap_y,
            context.surface_width,
            context.surface_height,
            context.level_count,
            context.current_level_select_page_start,
        ) {
            return AppInput::LevelSelectSelect(level);
        }

        return AppInput::NoOp;
    }

    if context.is_solved {
        return AppInput::SolvedAdvance;
    }

    if let Some((x, y)) =
        context
            .board_viewport
            .screen_to_cell(context.tap_x, context.tap_y, context.board)
    {
        return AppInput::BoardTap { x, y };
    }

    AppInput::NoOp
}
