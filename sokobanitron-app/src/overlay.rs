use crate::{AppOverlay, AppScreen, AppState};

pub fn is_overlay_open(app_state: &AppState) -> bool {
    app_state.ui.overlay.is_some()
}

pub fn is_gameplay_screen(app_state: &AppState) -> bool {
    matches!(app_state.ui.screen, AppScreen::Gameplay)
}

pub fn is_editor_screen(app_state: &AppState) -> bool {
    matches!(app_state.ui.screen, AppScreen::Editor)
}

pub fn is_gameplay_menu_open(app_state: &AppState) -> bool {
    matches!(app_state.ui.overlay, Some(AppOverlay::GameplayMenu))
}

pub fn is_editor_menu_open(app_state: &AppState) -> bool {
    matches!(app_state.ui.overlay, Some(AppOverlay::EditorMenu))
}

pub fn is_level_select_open(app_state: &AppState) -> bool {
    matches!(app_state.ui.overlay, Some(AppOverlay::LevelSelect { .. }))
}

pub fn level_select_page_start(app_state: &AppState) -> Option<usize> {
    match &app_state.ui.overlay {
        Some(AppOverlay::LevelSelect { page_start }) => Some(*page_start),
        _ => None,
    }
}

pub fn active_screen(app_state: &AppState) -> AppScreen {
    app_state.ui.screen
}

#[cfg(test)]
mod tests {
    use super::{
        active_screen, is_editor_menu_open, is_editor_screen, is_gameplay_menu_open,
        is_gameplay_screen, is_level_select_open, is_overlay_open, level_select_page_start,
    };
    use crate::{AppOverlay, AppScreen, AppState};

    #[test]
    fn overlay_helpers_for_default_state() {
        let app_state = AppState::default();
        assert!(!is_overlay_open(&app_state));
        assert!(is_gameplay_screen(&app_state));
        assert!(!is_editor_screen(&app_state));
        assert_eq!(level_select_page_start(&app_state), None);
        assert_eq!(active_screen(&app_state), AppScreen::Gameplay);
    }

    #[test]
    fn overlay_helpers_for_level_select_overlay() {
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::LevelSelect { page_start: 7 });
        assert!(is_overlay_open(&app_state));
        assert!(!is_gameplay_menu_open(&app_state));
        assert!(!is_editor_menu_open(&app_state));
        assert!(is_level_select_open(&app_state));
        assert_eq!(level_select_page_start(&app_state), Some(7));
    }

    #[test]
    fn overlay_helpers_for_gameplay_menu() {
        let mut app_state = AppState::default();
        app_state.ui.overlay = Some(AppOverlay::GameplayMenu);
        assert!(is_overlay_open(&app_state));
        assert!(is_gameplay_menu_open(&app_state));
        assert!(!is_editor_menu_open(&app_state));
        assert!(!is_level_select_open(&app_state));
        assert_eq!(level_select_page_start(&app_state), None);
    }

    #[test]
    fn overlay_helpers_for_editor_menu() {
        let mut app_state = AppState::default();
        app_state.ui.screen = AppScreen::Editor;
        app_state.ui.overlay = Some(AppOverlay::EditorMenu);
        assert!(is_overlay_open(&app_state));
        assert!(!is_gameplay_menu_open(&app_state));
        assert!(is_editor_menu_open(&app_state));
        assert!(!is_level_select_open(&app_state));
        assert!(!is_gameplay_screen(&app_state));
        assert!(is_editor_screen(&app_state));
        assert_eq!(level_select_page_start(&app_state), None);
        assert_eq!(active_screen(&app_state), AppScreen::Editor);
    }
}
