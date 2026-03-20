pub mod action;
pub mod app_state;
pub mod driver;
pub mod frame;
pub mod input;
pub mod menu;
pub mod present;
pub mod presentation_profile;
pub mod reducer;
pub mod ui_state;

pub use action::AppAction;
pub use app_state::AppState;
pub use driver::{AppDriverContext, AppliedUpdate, apply_action_and_present_in_context};
pub use frame::FrameRequest;
pub use input::{AppInput, interpret_input};
pub use menu::{is_menu_open, menu_page_start};
pub use present::{
    FrameSink, PresentationPlan, PresentationStep, build_presentation_plan,
    execute_presentation_plan,
};
pub use presentation_profile::{BoxPathStyle, BoxRemovedStyle, PresentMode, PresentationProfile};
pub use reducer::{AppUpdate, apply_action};
pub use ui_state::{AppMode, UiState};
