//! Hit-testing helpers for presentation-owned surfaces.

mod controls;
mod gameplay;
mod level_select;
mod level_set_select;
mod overlay;

pub use controls::{
    ControlsButtonAction, controls_button_action_at, top_menu_toggle_button_contains,
};
pub use gameplay::{
    GameplaySurfaceLayer, GameplaySurfaceModel, GameplaySurfaceTarget, LevelSelectSurfaceTarget,
    LevelSetSelectSurfaceTarget, gameplay_surface_target_at,
};
pub use level_select::{
    MenuNavAction, level_select_menu_nav_action_at, level_select_menu_nav_action_for_swipe,
    level_select_menu_start_for_nav, level_select_menu_target_at,
};
pub use level_set_select::{
    level_set_select_nav_action_at, level_set_select_start_for_nav, level_set_select_target_at,
};
pub use overlay::{
    overlay_primary_action_button_contains, overlay_secondary_action_button_contains,
};
