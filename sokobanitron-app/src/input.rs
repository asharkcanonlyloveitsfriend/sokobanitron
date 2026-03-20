use crate::action::AppAction;
use crate::app_state::AppState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppInput {
    ControlRestart,
    ControlUndo,
    ControlToggleMenu,
    MenuNavigate { page_start: usize },
    MenuSelectLevel(usize),
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
        AppInput::ControlToggleMenu => AppAction::ToggleMenu,
        AppInput::MenuNavigate { page_start } => AppAction::SetMenuPageStart(page_start),
        AppInput::MenuSelectLevel(level) => AppAction::SelectLevel(level),
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
    use crate::{AppAction, AppState};

    #[test]
    fn interpret_control_restart_maps_to_restart_action() {
        let app_state = AppState::default();
        assert_eq!(
            interpret_input(&app_state, AppInput::ControlRestart),
            AppAction::Restart
        );
    }

    #[test]
    fn interpret_menu_navigate_maps_to_set_menu_page_start_action() {
        let app_state = AppState::default();
        assert_eq!(
            interpret_input(&app_state, AppInput::MenuNavigate { page_start: 12 }),
            AppAction::SetMenuPageStart(12)
        );
    }
}
