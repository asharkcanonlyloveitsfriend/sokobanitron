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
pub mod presentation;
pub mod reducer;
pub mod state;

pub use action::AppAction;
pub use driver::{AppDriverContext, AppliedUpdate, apply_action_in_context};
pub use input::{AppInput, interpret_input};
pub use presentation::{
    FrameRequest, FrameSink, PresentMode, PresentationPlan, build_presentation_plan,
    render_presentation_plan,
};
pub use reducer::{AppUpdate, apply_action};
pub use state::{AppOverlay, AppScreen, AppState, UiState};
