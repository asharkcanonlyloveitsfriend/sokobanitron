use crate::editor::EditorUiState;
use crate::gameplay::GameplayUiState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppScreen {
    Gameplay,
    Editor,
}

impl AppScreen {
    pub fn default_overlay(self) -> AppOverlay {
        match self {
            Self::Gameplay => AppOverlay::GameplayMenu,
            Self::Editor => AppOverlay::EditorMenu,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppOverlay {
    GameplayMenu,
    LevelSelect { page_start: usize },
    LevelSetSelect { page_start: usize },
    EditorMenu,
}

impl AppOverlay {
    pub fn owning_screen(self) -> AppScreen {
        match self {
            Self::GameplayMenu | Self::LevelSelect { .. } | Self::LevelSetSelect { .. } => {
                AppScreen::Gameplay
            }
            Self::EditorMenu => AppScreen::Editor,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppInteractionMode {
    Gameplay,
    Editor,
    Overlay(AppOverlay),
}

impl AppInteractionMode {
    pub fn screen(self) -> AppScreen {
        match self {
            Self::Gameplay => AppScreen::Gameplay,
            Self::Editor => AppScreen::Editor,
            Self::Overlay(overlay) => overlay.owning_screen(),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppState {
    pub ui: UiState,
    pub editor: EditorUiState,
    pub gameplay: GameplayUiState,
    pub supports_multi_touch: bool,
}

impl AppState {
    pub fn active_screen(&self) -> AppScreen {
        self.ui.screen
    }

    pub fn interaction_mode(&self) -> AppInteractionMode {
        match self.ui.overlay {
            Some(overlay) => AppInteractionMode::Overlay(overlay),
            None => match self.ui.screen {
                AppScreen::Gameplay => AppInteractionMode::Gameplay,
                AppScreen::Editor => AppInteractionMode::Editor,
            },
        }
    }

    pub fn is_overlay_open(&self) -> bool {
        self.ui.overlay.is_some()
    }

    pub fn is_gameplay_screen(&self) -> bool {
        matches!(self.ui.screen, AppScreen::Gameplay)
    }

    pub fn is_editor_screen(&self) -> bool {
        matches!(self.ui.screen, AppScreen::Editor)
    }

    pub fn is_gameplay_menu_open(&self) -> bool {
        matches!(self.ui.overlay, Some(AppOverlay::GameplayMenu))
    }

    pub fn is_editor_menu_open(&self) -> bool {
        matches!(self.ui.overlay, Some(AppOverlay::EditorMenu))
    }

    pub fn is_level_select_open(&self) -> bool {
        matches!(self.ui.overlay, Some(AppOverlay::LevelSelect { .. }))
    }

    pub fn is_level_set_select_open(&self) -> bool {
        matches!(self.ui.overlay, Some(AppOverlay::LevelSetSelect { .. }))
    }

    pub fn level_select_page_start(&self) -> Option<usize> {
        match self.ui.overlay {
            Some(AppOverlay::LevelSelect { page_start }) => Some(page_start),
            _ => None,
        }
    }

    pub fn level_set_select_page_start(&self) -> Option<usize> {
        match self.ui.overlay {
            Some(AppOverlay::LevelSetSelect { page_start }) => Some(page_start),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppInteractionMode, AppOverlay, AppScreen, AppState};

    #[test]
    fn overlay_helpers_for_default_state() {
        let app_state = AppState::default();
        assert!(!app_state.is_overlay_open());
        assert!(app_state.is_gameplay_screen());
        assert!(!app_state.is_editor_screen());
        assert!(!app_state.supports_multi_touch);
        assert_eq!(app_state.level_select_page_start(), None);
        assert_eq!(app_state.active_screen(), AppScreen::Gameplay);
        assert_eq!(app_state.interaction_mode(), AppInteractionMode::Gameplay);
    }

    #[test]
    fn overlay_helpers_for_level_select_overlay() {
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start: 7 });
        assert!(app_state.is_overlay_open());
        assert!(!app_state.is_gameplay_menu_open());
        assert!(!app_state.is_editor_menu_open());
        assert!(app_state.is_level_select_open());
        assert!(!app_state.is_level_set_select_open());
        assert_eq!(app_state.level_select_page_start(), Some(7));
        assert_eq!(
            app_state.interaction_mode(),
            AppInteractionMode::Overlay(AppOverlay::LevelSelect { page_start: 7 })
        );
    }

    #[test]
    fn overlay_helpers_for_gameplay_menu() {
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::GameplayMenu);
        assert!(app_state.is_overlay_open());
        assert!(app_state.is_gameplay_menu_open());
        assert!(!app_state.is_editor_menu_open());
        assert!(!app_state.is_level_select_open());
        assert!(!app_state.is_level_set_select_open());
        assert_eq!(app_state.level_select_page_start(), None);
        assert_eq!(
            app_state.interaction_mode(),
            AppInteractionMode::Overlay(AppOverlay::GameplayMenu)
        );
    }

    #[test]
    fn overlay_helpers_for_editor_menu() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        app_state.ui.overlay = Some(AppOverlay::EditorMenu);
        assert!(app_state.is_overlay_open());
        assert!(!app_state.is_gameplay_menu_open());
        assert!(app_state.is_editor_menu_open());
        assert!(!app_state.is_level_select_open());
        assert!(!app_state.is_level_set_select_open());
        assert!(!app_state.is_gameplay_screen());
        assert!(app_state.is_editor_screen());
        assert_eq!(app_state.level_select_page_start(), None);
        assert_eq!(app_state.active_screen(), AppScreen::Editor);
        assert_eq!(
            app_state.interaction_mode(),
            AppInteractionMode::Overlay(AppOverlay::EditorMenu)
        );
    }

    #[test]
    fn overlay_owns_expected_screen() {
        assert_eq!(
            AppOverlay::GameplayMenu.owning_screen(),
            AppScreen::Gameplay
        );
        assert_eq!(
            AppOverlay::LevelSelect { page_start: 3 }.owning_screen(),
            AppScreen::Gameplay
        );
        assert_eq!(
            AppOverlay::LevelSetSelect { page_start: 2 }.owning_screen(),
            AppScreen::Gameplay
        );
        assert_eq!(AppOverlay::EditorMenu.owning_screen(), AppScreen::Editor);
    }

    #[test]
    fn screen_knows_default_overlay() {
        assert_eq!(
            AppScreen::Gameplay.default_overlay(),
            AppOverlay::GameplayMenu
        );
        assert_eq!(AppScreen::Editor.default_overlay(), AppOverlay::EditorMenu);
    }
}
