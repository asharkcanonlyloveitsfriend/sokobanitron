use crate::{
    BoardViewport, ControlsButtonAction, controls_button_action_at,
    level_select_menu_nav_action_at, level_select_menu_start_for_nav, level_select_menu_target_at,
    overlay_primary_action_button_contains, top_left_level_button_rect,
};
use sokobanitron_app::AppInput;
use sokobanitron_gameplay::BoardView;

#[derive(Debug, Clone, Copy)]
pub struct GameplayTapContext<'a> {
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

// TODO: This gameplay tap interpreter lives in `renderer` for now because it depends on
// renderer-owned hit testing and viewport types; that layering is provisional.
pub fn interpret_gameplay_tap(context: GameplayTapContext<'_>) -> AppInput {
    if context.is_gameplay_menu_open
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

#[cfg(test)]
mod tests {
    use super::{GameplayTapContext, interpret_gameplay_tap};
    use crate::{
        BoardViewport, ControlsButtonRects, controls_button_rects, fit_board_viewport_for_controls,
        level_select_menu_nav_action_at, level_select_menu_slot_rects,
        level_select_menu_start_for_nav, level_select_scrollbar,
        overlay_primary_action_button_rect,
    };
    use sokobanitron_app::AppInput;
    use sokobanitron_gameplay::{BoardView, GameplayController};

    const WIDTH: u32 = 670;
    const HEIGHT: u32 = 891;

    fn test_controller(level_count: usize, current_level: usize) -> GameplayController {
        let level = "#######\n#@ $. #\n#######".to_string();
        GameplayController::new(vec![level; level_count], Some(current_level))
    }

    fn gameplay_context<'a>(board: &'a BoardView) -> GameplayTapContext<'a> {
        GameplayTapContext {
            is_gameplay_screen: true,
            is_gameplay_menu_open: false,
            is_level_select_open: false,
            is_overlay_open: false,
            surface_width: WIDTH,
            surface_height: HEIGHT,
            tap_x: 0.0,
            tap_y: 0.0,
            level_count: 8,
            current_level: 3,
            current_level_select_page_start: 0,
            can_undo: false,
            can_restart: false,
            is_solved: false,
            board_viewport: fit_board_viewport_for_controls(WIDTH, HEIGHT, board),
            board,
        }
    }

    fn find_nav_tap(
        width: u32,
        height: u32,
        level_count: usize,
        current_level: usize,
        page_start: usize,
    ) -> (f64, f64, usize) {
        let rail_start_x = width.saturating_sub(level_select_scrollbar::right_rail_width(width));
        for y in 0..height {
            for x in rail_start_x..width {
                if let Some(action) = level_select_menu_nav_action_at(
                    x as f64,
                    y as f64,
                    width,
                    height,
                    level_count,
                    current_level,
                    page_start,
                ) {
                    let next_start = level_select_menu_start_for_nav(
                        level_count,
                        current_level,
                        page_start,
                        action,
                    );
                    if next_start != page_start {
                        return (x as f64, y as f64, next_start);
                    }
                }
            }
        }
        panic!("expected to find a navigation tap target");
    }

    #[test]
    fn gameplay_menu_primary_action_enters_editor_mode() {
        let controller = test_controller(8, 3);
        let rect = overlay_primary_action_button_rect(WIDTH, HEIGHT);
        let input = interpret_gameplay_tap(GameplayTapContext {
            is_gameplay_menu_open: true,
            is_overlay_open: true,
            tap_x: (rect.x + rect.w / 2) as f64,
            tap_y: (rect.y + rect.h / 2) as f64,
            ..gameplay_context(controller.board())
        });

        assert_eq!(input, AppInput::EnterEditorMode);
    }

    #[test]
    fn top_left_level_button_opens_level_select() {
        let controller = test_controller(8, 3);
        let input = interpret_gameplay_tap(GameplayTapContext {
            tap_x: 24.0,
            tap_y: 24.0,
            ..gameplay_context(controller.board())
        });

        assert_eq!(input, AppInput::OpenLevelSelect);
    }

    #[test]
    fn controls_buttons_map_to_gameplay_inputs() {
        let controller = test_controller(8, 3);
        let ControlsButtonRects {
            menu,
            undo,
            restart,
        } = controls_button_rects(WIDTH, HEIGHT);

        let menu_input = interpret_gameplay_tap(GameplayTapContext {
            tap_x: (menu.x + menu.w / 2) as f64,
            tap_y: (menu.y + menu.h / 2) as f64,
            ..gameplay_context(controller.board())
        });
        let undo_input = interpret_gameplay_tap(GameplayTapContext {
            can_undo: true,
            tap_x: (undo.x + undo.w / 2) as f64,
            tap_y: (undo.y + undo.h / 2) as f64,
            ..gameplay_context(controller.board())
        });
        let restart_input = interpret_gameplay_tap(GameplayTapContext {
            can_restart: true,
            tap_x: (restart.x + restart.w / 2) as f64,
            tap_y: (restart.y + restart.h / 2) as f64,
            ..gameplay_context(controller.board())
        });

        assert_eq!(menu_input, AppInput::OverlayToggle);
        assert_eq!(undo_input, AppInput::ControlUndo);
        assert_eq!(restart_input, AppInput::ControlRestart);
    }

    #[test]
    fn level_select_taps_cover_navigation_and_selection() {
        let controller = test_controller(8, 3);
        let page_start = 0;
        let (nav_x, nav_y, expected_start) =
            find_nav_tap(WIDTH, HEIGHT, 8, controller.current_level(), page_start);
        let slot_rect = level_select_menu_slot_rects(WIDTH, HEIGHT)[0];
        let select_x = slot_rect.0 + slot_rect.2 as i32 / 2;
        let select_y = slot_rect.1 + slot_rect.3 as i32 / 2;

        let nav_input = interpret_gameplay_tap(GameplayTapContext {
            is_level_select_open: true,
            is_overlay_open: true,
            current_level_select_page_start: page_start,
            tap_x: nav_x,
            tap_y: nav_y,
            ..gameplay_context(controller.board())
        });
        let select_input = interpret_gameplay_tap(GameplayTapContext {
            is_level_select_open: true,
            is_overlay_open: true,
            current_level_select_page_start: page_start,
            tap_x: select_x as f64,
            tap_y: select_y as f64,
            ..gameplay_context(controller.board())
        });

        assert_eq!(
            nav_input,
            AppInput::LevelSelectNavigate {
                page_start: expected_start,
            }
        );
        assert_eq!(select_input, AppInput::LevelSelectSelect(0));
    }

    #[test]
    fn solved_and_board_taps_map_to_expected_inputs() {
        let controller = test_controller(8, 3);
        let viewport: BoardViewport =
            fit_board_viewport_for_controls(WIDTH, HEIGHT, controller.board());
        let (x, y, w, h) = viewport.cell_to_screen_rect(1, 1);

        let solved_input = interpret_gameplay_tap(GameplayTapContext {
            is_solved: true,
            tap_x: 400.0,
            tap_y: 400.0,
            board_viewport: viewport,
            ..gameplay_context(controller.board())
        });
        let board_input = interpret_gameplay_tap(GameplayTapContext {
            tap_x: (x + w as i32 / 2) as f64,
            tap_y: (y + h as i32 / 2) as f64,
            board_viewport: viewport,
            ..gameplay_context(controller.board())
        });

        assert_eq!(solved_input, AppInput::SolvedAdvance);
        assert_eq!(board_input, AppInput::BoardTap { x: 1, y: 1 });
    }

    #[test]
    fn gameplay_menu_non_primary_taps_are_noop() {
        let controller = test_controller(8, 3);
        let input = interpret_gameplay_tap(GameplayTapContext {
            is_gameplay_menu_open: true,
            is_overlay_open: true,
            tap_x: 10.0,
            tap_y: 200.0,
            ..gameplay_context(controller.board())
        });

        assert_eq!(input, AppInput::NoOp);
    }
}
