mod frame;
mod hit_test;
mod input;
mod view;

pub use frame::{build_current_editor_frame_request, build_sleep_editor_frame_request};
pub use input::{
    EditorUiAction, editor_cursor_moved, editor_mouse_pressed, editor_mouse_released, editor_touch,
};
pub use view::EditorUiState;

use crate::app::state::AppState;
use std::time::Duration;

pub fn resize_editor_surface(app_state: &mut AppState, width: u32, height: u32) {
    view::resize_editor_surface(&mut app_state.editor, width, height);
}

pub fn set_editor_touch_slop(app_state: &mut AppState, tap_slop_px: i32) {
    view::set_editor_touch_slop(&mut app_state.editor, tap_slop_px);
}

pub fn set_editor_double_tap_window(app_state: &mut AppState, window: Duration) {
    view::set_editor_double_tap_window(&mut app_state.editor, window);
}

pub fn reset_editor_interaction_state(app_state: &mut AppState) {
    view::reset_editor_interaction_state(&mut app_state.editor);
}

pub fn reset_editor_view_state(app_state: &mut AppState) {
    view::reset_editor_view_state(&mut app_state.editor);
}
