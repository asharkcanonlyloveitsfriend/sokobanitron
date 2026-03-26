pub mod action;
pub mod driver;
pub mod input;
pub mod presentation;
pub mod reducer;
pub mod state;

pub use action::AppAction;
pub use driver::{AppDriverContext, AppliedUpdate, apply_action_and_present_in_context};
pub use input::{AppInput, interpret_input};
pub use presentation::{
    FrameRequest, FrameSink, PresentMode, PresentationPlan, PresentationStep,
    build_presentation_plan, execute_presentation_plan,
};
pub use reducer::{AppUpdate, apply_action};
pub use state::{AppOverlay, AppScreen, AppState, UiState};
