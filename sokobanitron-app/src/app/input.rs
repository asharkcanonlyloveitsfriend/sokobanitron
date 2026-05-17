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
    fn interpret_input_maps_each_semantic_input_to_action() {
        let cell = BoardCell::new(3, 4);
        let cases = [
            (AppInput::Restart, AppAction::Restart),
            (AppInput::Undo, AppAction::Undo),
            (AppInput::OverlayToggle, AppAction::ToggleOverlay),
            (AppInput::OpenLevelSelect, AppAction::OpenLevelSelect),
            (AppInput::OpenLevelSetSelect, AppAction::OpenLevelSetSelect),
            (AppInput::OverlayOpen, AppAction::OpenOverlay),
            (AppInput::OverlayClose, AppAction::CloseOverlay),
            (AppInput::EnterEditorMode, AppAction::EnterEditorMode),
            (AppInput::EnterGameplayMode, AppAction::EnterGameplayMode),
            (
                AppInput::LevelSelectNavigate { page_start: 12 },
                AppAction::SetLevelSelectPageStart(12),
            ),
            (AppInput::LevelSelectSelect(5), AppAction::SelectLevel(5)),
            (
                AppInput::LevelSetSelectNavigate { page_start: 3 },
                AppAction::SetLevelSetSelectPageStart(3),
            ),
            (
                AppInput::LevelSetSelectSelect(2),
                AppAction::SelectLevelSet(2),
            ),
            (AppInput::BoardTap(cell), AppAction::TapBoardCell(cell)),
            (
                AppInput::BoardDoubleTap(cell),
                AppAction::DoubleTapBoardCell(cell),
            ),
            (AppInput::NoOp, AppAction::NoOp),
        ];

        for (input, expected) in cases {
            assert_eq!(interpret_input(input), expected);
        }
    }
}
