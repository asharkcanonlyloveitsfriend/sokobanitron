use crate::ui_state::UiState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppState {
    pub ui: UiState,
}
