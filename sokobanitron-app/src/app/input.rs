use super::action::AppAction;
use super::state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppInput {
    // Control-style gameplay inputs.
    ControlRestart,
    ControlUndo,
    // Semantic navigation inputs.
    OverlayToggle,
    OpenLevelSelect,
    OverlayOpen,
    OverlayClose,
    EnterEditorMode,
    EnterGameplayMode,
    LevelSelectNavigate { page_start: usize },
    LevelSelectSelect(usize),
    SolvedAdvance,
    BoardTap { x: u32, y: u32 },
    KeyRestart,
    KeyUndo,
    NoOp,
}

pub fn interpret_input(_app_state: &AppState, input: AppInput) -> AppAction {
    match input {
        AppInput::ControlRestart => AppAction::Restart,
        AppInput::ControlUndo => AppAction::Undo,
        AppInput::OverlayToggle => AppAction::ToggleOverlay,
        AppInput::OpenLevelSelect => AppAction::OpenLevelSelect,
        AppInput::OverlayOpen => AppAction::OpenOverlay,
        AppInput::OverlayClose => AppAction::CloseOverlay,
        AppInput::EnterEditorMode => AppAction::EnterEditorMode,
        AppInput::EnterGameplayMode => AppAction::EnterGameplayMode,
        AppInput::LevelSelectNavigate { page_start } => {
            AppAction::SetLevelSelectPageStart(page_start)
        }
        AppInput::LevelSelectSelect(level) => AppAction::SelectLevel(level),
        AppInput::SolvedAdvance => AppAction::AdvanceAfterSolved,
        AppInput::BoardTap { x, y } => AppAction::TapBoardCell { x, y },
        AppInput::KeyRestart => AppAction::Restart,
        AppInput::KeyUndo => AppAction::Undo,
        AppInput::NoOp => AppAction::NoOp,
    }
}

#[cfg(test)]
mod tests {
    use super::{AppInput, interpret_input};
    use crate::app::action::AppAction;
    use crate::app::state::AppState;

    #[test]
    fn interpret_control_restart_maps_to_restart_action() {
        let app_state = AppState::default();
        assert_eq!(
            interpret_input(&app_state, AppInput::ControlRestart),
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
