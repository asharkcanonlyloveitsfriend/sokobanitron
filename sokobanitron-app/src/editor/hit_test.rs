//! Editor surface hit classification.
//!
//! Geometry-driven target selection lives here so editor input policy can stay focused on
//! translating targets into commands and mode changes.
//!
//! This stays app-owned on purpose: the editor's visible window, zoom state, and world-origin
//! mapping are app-local concerns rather than shared presentation geometry.

use crate::app::state::AppState;
use presentation::hit_test::{
    overlay_primary_action_button_contains, top_menu_toggle_button_contains,
};
use presentation::layout::top_left_level_button_rect;
use sokobanitron_level_editor::LevelEditor;

use super::view::{
    VisibleBoardWindow, build_visible_window, zoom_in_button_rect, zoom_out_button_rect,
};

#[derive(Debug)]
pub(super) struct EditorSurfaceModel {
    pub(super) surface_width: u32,
    pub(super) surface_height: u32,
    pub(super) visible_window: VisibleBoardWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditorControlSlot {
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EditorSurfaceTarget {
    ModeToggle,
    TopMenuToggle,
    OverlayPrimaryAction,
    ControlSlot(EditorControlSlot),
    BoardCell { world_x: i32, world_y: i32 },
}

pub(super) fn build_editor_surface_model(
    app_state: &AppState,
    editor: &LevelEditor,
) -> EditorSurfaceModel {
    EditorSurfaceModel {
        surface_width: app_state.editor.viewport.surface_width,
        surface_height: app_state.editor.viewport.surface_height,
        visible_window: build_visible_window(&app_state.editor, editor),
    }
}

pub(super) fn editor_surface_target_at(
    surface: &EditorSurfaceModel,
    screen_x: f64,
    screen_y: f64,
) -> Option<EditorSurfaceTarget> {
    if top_left_level_button_rect().contains(screen_x, screen_y) {
        return Some(EditorSurfaceTarget::ModeToggle);
    }
    if top_menu_toggle_button_contains(screen_x, screen_y, surface.surface_width) {
        return Some(EditorSurfaceTarget::TopMenuToggle);
    }
    if overlay_primary_action_button_contains(
        screen_x,
        screen_y,
        surface.surface_width,
        surface.surface_height,
    ) {
        return Some(EditorSurfaceTarget::OverlayPrimaryAction);
    }
    if zoom_out_button_rect(surface.surface_height).contains(screen_x, screen_y) {
        return Some(EditorSurfaceTarget::ControlSlot(
            EditorControlSlot::BottomLeft,
        ));
    }
    if zoom_in_button_rect(surface.surface_width, surface.surface_height)
        .contains(screen_x, screen_y)
    {
        return Some(EditorSurfaceTarget::ControlSlot(
            EditorControlSlot::BottomRight,
        ));
    }

    if let Some((world_x, world_y)) = surface
        .visible_window
        .screen_to_world_cell(screen_x, screen_y)
    {
        return Some(EditorSurfaceTarget::BoardCell { world_x, world_y });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{EditorSurfaceTarget, build_editor_surface_model, editor_surface_target_at};
    use crate::app::state::AppState;
    use sokobanitron_level_editor::LevelEditor;

    #[test]
    fn no_hit_returns_none() {
        let app_state = AppState::default();
        let editor = LevelEditor::new();
        let surface = build_editor_surface_model(&app_state, &editor);

        assert_eq!(editor_surface_target_at(&surface, -1.0, -1.0), None);
    }

    #[test]
    fn mode_toggle_hit_uses_top_left_control() {
        let app_state = AppState::default();
        let editor = LevelEditor::new();
        let surface = build_editor_surface_model(&app_state, &editor);

        assert_eq!(
            editor_surface_target_at(&surface, 20.0, 20.0),
            Some(EditorSurfaceTarget::ModeToggle)
        );
    }

    #[test]
    fn board_cell_hit_maps_back_through_visible_window() {
        let app_state = AppState::default();
        let editor = LevelEditor::new();
        let surface = build_editor_surface_model(&app_state, &editor);
        let local_x = surface.visible_window.board.width() / 2;
        let local_y = surface.visible_window.board.height() / 2;
        let (screen_x, screen_y, width, height) = surface
            .visible_window
            .viewport
            .cell_to_screen_rect(local_x, local_y);

        let target = editor_surface_target_at(
            &surface,
            (screen_x + (width / 2) as i32) as f64,
            (screen_y + (height / 2) as i32) as f64,
        );

        assert_eq!(
            target,
            Some(EditorSurfaceTarget::BoardCell {
                world_x: surface.visible_window.world_origin_x + local_x as i32,
                world_y: surface.visible_window.world_origin_y + local_y as i32,
            })
        );
    }
}
