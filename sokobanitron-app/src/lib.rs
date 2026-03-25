pub mod action;
pub mod app_state;
pub mod driver;
pub mod editor_ui;
pub mod frame;
pub mod gameplay_frames;
pub mod gameplay_input;
pub mod input;
pub mod level_bootstrap;
pub mod overlay;
pub mod present;
pub mod presentation_profile;
pub mod reducer;
pub mod ui_state;

pub use action::AppAction;
pub use app_state::AppState;
pub use driver::{AppDriverContext, AppliedUpdate, apply_action_and_present_in_context};
pub use editor_ui::{
    EditorPointerPhase, EditorUiState, build_current_editor_frame_request, editor_cursor_moved,
    editor_mouse_pressed, editor_mouse_released, editor_touch, reset_editor_interaction_state,
    resize_editor_surface,
};
pub use frame::FrameRequest;
pub use gameplay_frames::{
    build_current_frame_request, build_current_gameplay_frame_request, build_gameplay_frame_request,
};
pub use gameplay_input::{GameplayTapContext, interpret_gameplay_tap};
pub use input::{AppInput, interpret_input};
pub use level_bootstrap::{InitialLevels, load_initial_levels_for_app};
pub use overlay::{
    active_screen, is_editor_menu_open, is_editor_screen, is_gameplay_menu_open,
    is_gameplay_screen, is_level_select_open, is_overlay_open, level_select_page_start,
};
pub use present::{
    FrameSink, PresentationPlan, PresentationStep, build_presentation_plan,
    execute_presentation_plan,
};
pub use presentation_profile::PresentMode;
pub use reducer::{AppUpdate, apply_action};
pub use renderer::{GameplayScreenRequest, LevelSelectScreenRequest};
pub use ui_state::{AppOverlay, AppScreen, UiState};
