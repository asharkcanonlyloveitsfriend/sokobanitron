//! App-level orchestration for Sokobanitron.
//!
//! The `app` module sits between gameplay/editor domain state and the shared presentation system.
//! It owns app UI state, interprets input into actions, and translates domain outcomes into
//! presentation plans and screen requests.
//!
//! It does not own pixel drawing or device-specific presentation policy. Those belong in
//! `sokobanitron-presentation` and the platform clients respectively.

pub mod action;
pub mod driver;
pub mod input;
pub mod persistence;
pub mod presentation;
pub mod reducer;
pub mod state;

pub use action::AppAction;
pub use driver::{
    AppDriverContext, AppPointerInput, AppRuntimeMut, AppliedUpdate, EditorAppRuntimeMut,
    RenderWorkResult, apply_action_and_render_in_context, apply_action_in_context,
    apply_editor_ui_action, apply_input_and_render_in_context,
    build_current_app_screen_frame_request, continue_pending_render_work_and_render_in_context,
    handle_pointer_input_and_render_in_context,
};
pub use input::{AppInput, interpret_input};
pub use presentation::{
    FrameRequest, FrameSink, PresentMode, PresentationPlan, build_presentation_plan,
    render_presentation_plan,
};
pub use reducer::{AppUpdate, PersistenceUpdate, apply_action};
pub use state::{AppInteractionMode, AppOverlay, AppScreen, AppState, UiState};
