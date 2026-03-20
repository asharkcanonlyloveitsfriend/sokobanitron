#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Gameplay,
    Menu { page_start: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiState {
    pub mode: AppMode,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            mode: AppMode::Gameplay,
        }
    }
}
