mod frame;
mod input;
mod paint_mode;
mod view;

pub use frame::build_current_editor_frame_request;
pub use input::{
    EditorPointerPhase, editor_cursor_moved, editor_mouse_pressed, editor_mouse_released,
    editor_touch,
};
pub use view::EditorUiState;

use crate::AppState;

pub fn resize_editor_surface(app_state: &mut AppState, width: u32, height: u32) {
    view::resize_editor_surface(&mut app_state.editor, width, height);
}

pub fn reset_editor_interaction_state(app_state: &mut AppState) {
    view::reset_editor_interaction_state(&mut app_state.editor);
}
