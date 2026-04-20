use super::action::AppAction;
use super::state::AppState;
use sokobanitron_gameplay::BoardCell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppInput {
    Restart,
    Undo,
    ZoomGameplayIn {
        zoom_origin_x: u32,
        zoom_origin_y: u32,
    },
    ZoomGameplayOut,
    // Semantic navigation inputs.
    OverlayToggle,
    OpenLevelSelect,
    OpenLevelSetSelect,
    OverlayOpen,
    OverlayClose,
    EnterEditorMode,
    EnterGameplayMode,
    LevelSelectNavigate {
        page_start: usize,
    },
    LevelSelectSelect(usize),
    LevelSetSelectNavigate {
        page_start: usize,
    },
    LevelSetSelectSelect(usize),
    BoardTap(BoardCell),
    BoardDoubleTap(BoardCell),
    GameplaySwipePan {
        delta_x: i32,
        delta_y: i32,
    },
    NoOp,
}

pub fn interpret_input(_app_state: &AppState, input: AppInput) -> AppAction {
    match input {
        AppInput::Restart => AppAction::Restart,
        AppInput::Undo => AppAction::Undo,
        AppInput::ZoomGameplayIn {
            zoom_origin_x,
            zoom_origin_y,
        } => AppAction::ZoomGameplayIn {
            zoom_origin_x,
            zoom_origin_y,
        },
        AppInput::ZoomGameplayOut => AppAction::ZoomGameplayOut,
        AppInput::OverlayToggle => AppAction::ToggleOverlay,
        AppInput::OpenLevelSelect => AppAction::OpenLevelSelect,
        AppInput::OpenLevelSetSelect => AppAction::OpenLevelSetSelect,
        AppInput::OverlayOpen => AppAction::OpenOverlay,
        AppInput::OverlayClose => AppAction::CloseOverlay,
        AppInput::EnterEditorMode => AppAction::EnterEditorMode,
        AppInput::EnterGameplayMode => AppAction::EnterGameplayMode,
        AppInput::LevelSelectNavigate { page_start } => {
            AppAction::SetLevelSelectPageStart(page_start)
        }
        AppInput::LevelSelectSelect(level) => AppAction::SelectLevel(level),
        AppInput::LevelSetSelectNavigate { page_start } => {
            AppAction::SetLevelSetSelectPageStart(page_start)
        }
        AppInput::LevelSetSelectSelect(level_set) => AppAction::SelectLevelSet(level_set),
        AppInput::BoardTap(cell) => AppAction::TapBoardCell(cell),
        AppInput::BoardDoubleTap(cell) => AppAction::DoubleTapBoardCell(cell),
        AppInput::GameplaySwipePan { delta_x, delta_y } => {
            AppAction::PanZoomedGameplay { delta_x, delta_y }
        }
        AppInput::NoOp => AppAction::NoOp,
    }
}

#[cfg(test)]
mod tests {
    use super::{AppInput, interpret_input};
    use crate::app::action::AppAction;
    use crate::app::state::AppState;
    use sokobanitron_gameplay::BoardCell;

    #[test]
    fn interpret_restart_maps_to_restart_action() {
        let app_state = AppState::default();
        assert_eq!(
            interpret_input(&app_state, AppInput::Restart),
            AppAction::Restart
        );
    }

    #[test]
    fn interpret_level_select_navigate_maps_to_set_level_select_page_start_action() {
        let app_state = AppState::default();
        assert_eq!(
            interpret_input(&app_state, AppInput::LevelSelectNavigate { page_start: 12 }),
            AppAction::SetLevelSelectPageStart(12)
        );
    }

    #[test]
    fn interpret_gameplay_zoom_inputs_map_to_zoom_actions() {
        let app_state = AppState::default();
        assert_eq!(
            interpret_input(
                &app_state,
                AppInput::ZoomGameplayIn {
                    zoom_origin_x: 3,
                    zoom_origin_y: 5,
                }
            ),
            AppAction::ZoomGameplayIn {
                zoom_origin_x: 3,
                zoom_origin_y: 5,
            }
        );
        assert_eq!(
            interpret_input(&app_state, AppInput::ZoomGameplayOut),
            AppAction::ZoomGameplayOut
        );
    }

    #[test]
    fn interpret_overlay_toggle_maps_to_toggle_overlay() {
        let app_state = AppState::default();
        assert_eq!(
            interpret_input(&app_state, AppInput::OverlayToggle),
            AppAction::ToggleOverlay
        );
    }

    #[test]
    fn interpret_open_level_select_maps_to_open_level_select() {
        let app_state = AppState::default();
        assert_eq!(
            interpret_input(&app_state, AppInput::OpenLevelSelect),
            AppAction::OpenLevelSelect
        );
    }

    #[test]
    fn interpret_open_level_set_select_maps_to_open_level_set_select() {
        let app_state = AppState::default();
        assert_eq!(
            interpret_input(&app_state, AppInput::OpenLevelSetSelect),
            AppAction::OpenLevelSetSelect
        );
    }

    #[test]
    fn interpret_board_double_tap_maps_to_double_tap_board_cell_action() {
        let app_state = AppState::default();
        let cell = BoardCell::new(3, 4);
        assert_eq!(
            interpret_input(&app_state, AppInput::BoardDoubleTap(cell)),
            AppAction::DoubleTapBoardCell(cell)
        );
    }

    #[test]
    fn interpret_gameplay_swipe_pan_maps_to_action() {
        let app_state = AppState::default();
        assert_eq!(
            interpret_input(
                &app_state,
                AppInput::GameplaySwipePan {
                    delta_x: 64,
                    delta_y: -48,
                }
            ),
            AppAction::PanZoomedGameplay {
                delta_x: 64,
                delta_y: -48,
            }
        );
    }

    #[test]
    fn interpret_overlay_open_close_map_to_actions() {
        let app_state = AppState::default();
        assert_eq!(
            interpret_input(&app_state, AppInput::OverlayOpen),
            AppAction::OpenOverlay
        );
        assert_eq!(
            interpret_input(&app_state, AppInput::OverlayClose),
            AppAction::CloseOverlay
        );
    }
}
