use crate::editor_ui::EditorUiState;
use crate::gameplay_input::GameplayInteractionState;
use crate::ui_state::UiState;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppState {
    pub ui: UiState,
    pub editor: EditorUiState,
    pub gameplay: GameplayInteractionState,
    pub editor_available: bool,
}
