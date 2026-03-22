#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    Gameplay,
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppOverlay {
    GameplayMenu,
    LevelSelect { page_start: usize },
    EditorMenu,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiState {
    pub screen: AppScreen,
    pub overlay: Option<AppOverlay>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            screen: AppScreen::Gameplay,
            overlay: None,
        }
    }
}
