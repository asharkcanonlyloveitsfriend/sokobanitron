use crate::{AppMode, AppState};

pub fn is_menu_open(app_state: &AppState) -> bool {
    matches!(&app_state.ui.mode, AppMode::Menu { .. })
}

pub fn menu_page_start(app_state: &AppState) -> Option<usize> {
    match &app_state.ui.mode {
        AppMode::Menu { page_start } => Some(*page_start),
        AppMode::Gameplay => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_menu_open, menu_page_start};
    use crate::{AppMode, AppState};

    #[test]
    fn menu_helpers_for_gameplay_mode() {
        let app_state = AppState::default();
        assert!(!is_menu_open(&app_state));
        assert_eq!(menu_page_start(&app_state), None);
    }

    #[test]
    fn menu_helpers_for_menu_mode() {
        let mut app_state = AppState::default();
        app_state.ui.mode = AppMode::Menu { page_start: 7 };
        assert!(is_menu_open(&app_state));
        assert_eq!(menu_page_start(&app_state), Some(7));
    }
}
