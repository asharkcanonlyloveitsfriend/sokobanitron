use super::action::AppAction;
use sokobanitron_gameplay::BoardCell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppInput {
    Restart,
    Undo,
    // Semantic navigation inputs.
    OverlayToggle,
    OpenLevelSelect,
    OpenLevelSetSelect,
    OverlayOpen,
    OverlayClose,
    EnterEditorMode,
    EnterGameplayMode,
    LevelSelectNavigate { page_start: usize },
    LevelSelectSelect(usize),
    LevelSetSelectNavigate { page_start: usize },
    LevelSetSelectSelect(usize),
    BoardTap(BoardCell),
    BoardDoubleTap(BoardCell),
    NoOp,
}

pub fn interpret_input(input: AppInput) -> AppAction {
    match input {
        AppInput::Restart => AppAction::Restart,
        AppInput::Undo => AppAction::Undo,
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
        AppInput::NoOp => AppAction::NoOp,
    }
}

#[cfg(test)]
mod tests {
    use super::{AppInput, interpret_input};
    use crate::app::action::AppAction;
    use sokobanitron_gameplay::BoardCell;

    #[test]
    fn interpret_restart_maps_to_restart_action() {
        assert_eq!(interpret_input(AppInput::Restart), AppAction::Restart);
    }

    #[test]
    fn interpret_level_select_navigate_maps_to_set_level_select_page_start_action() {
        assert_eq!(
            interpret_input(AppInput::LevelSelectNavigate { page_start: 12 }),
            AppAction::SetLevelSelectPageStart(12)
        );
    }

    #[test]
    fn interpret_overlay_toggle_maps_to_toggle_overlay() {
        assert_eq!(
            interpret_input(AppInput::OverlayToggle),
            AppAction::ToggleOverlay
        );
    }

    #[test]
    fn interpret_open_level_select_maps_to_open_level_select() {
        assert_eq!(
            interpret_input(AppInput::OpenLevelSelect),
            AppAction::OpenLevelSelect
        );
    }

    #[test]
    fn interpret_open_level_set_select_maps_to_open_level_set_select() {
        assert_eq!(
            interpret_input(AppInput::OpenLevelSetSelect),
            AppAction::OpenLevelSetSelect
        );
    }

    #[test]
    fn interpret_board_double_tap_maps_to_double_tap_board_cell_action() {
        let cell = BoardCell::new(3, 4);
        assert_eq!(
            interpret_input(AppInput::BoardDoubleTap(cell)),
            AppAction::DoubleTapBoardCell(cell)
        );
    }

    #[test]
    fn interpret_overlay_open_close_map_to_actions() {
        assert_eq!(
            interpret_input(AppInput::OverlayOpen),
            AppAction::OpenOverlay
        );
        assert_eq!(
            interpret_input(AppInput::OverlayClose),
            AppAction::CloseOverlay
        );
    }
}
