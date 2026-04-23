//! App-level orchestration for Sokobanitron.
//!
//! The `app` module sits between gameplay/editor domain state and the shared presentation system.
//! It owns app UI state, interprets input into actions, and translates domain outcomes into
//! presentation plans and screen requests.
//!
//! It coordinates shared frame rendering through the presentation layer, while platform clients
//! still own wakeups, window/device surfaces, and final present-to-screen behavior.

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
    RenderWorkResult, SharedAppRuntime, apply_action_and_render_in_context,
    apply_action_in_context, apply_editor_ui_action, apply_input_and_render_in_context,
    build_current_app_screen_frame_request, continue_pending_render_work_and_render_in_context,
    handle_pointer_input_and_render_in_context, has_pending_render_work_in_context,
};
pub use input::{AppInput, interpret_input};
pub use presentation::{
    AppFrameRenderer, FrameDamage, FrameRequest, FrameSink, GameplayAnimationPolicy,
    PresentationPlan, RendererOverrides, ScreenRect, build_presentation_plan,
    render_presentation_plan,
};
pub use reducer::{AppUpdate, PersistenceUpdate, apply_action};
pub use state::{AppInteractionMode, AppOverlay, AppScreen, AppState, UiState};
