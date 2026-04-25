//! Presentation and rendering support for Sokobanitron clients.
//!
//! The crate is organized around five public concepts:
//! `assets`, `layout`, `hit_test`, `screen_requests`, and `renderer`.
//!
//! Internally those responsibilities live in matching subdirectories so the crate can be read as
//! a swappable presentation system rather than a single flat renderer module.
//!
//! This crate is the device-agnostic presentation layer. It owns shared geometry, screen request
//! types, gameplay presentation state, and pixel drawing, while platform clients continue to own
//! clocks, redraw scheduling, and final presentation to screen.

pub mod assets;
mod gameplay_animation;
pub mod gameplay_presentation;
pub mod hit_test;
pub mod layout;
pub mod renderer;
pub mod screen_requests;

pub use assets::{UiIcon, draw_ui_icon_in_rect};
pub use gameplay_animation::GameplayAnimationPolicy;
pub use gameplay_presentation::{
    GameplayDamage, GameplayPresentationResult, GameplayPresentationState,
    gameplay_damage_union_rect,
};
pub use hit_test::{
    ControlsButtonAction, GameplaySurfaceLayer, GameplaySurfaceModel, GameplaySurfaceTarget,
    LevelSelectSurfaceTarget, LevelSetSelectSurfaceTarget, MenuNavAction,
    gameplay_surface_target_at, level_select_menu_nav_action_at,
    level_select_menu_nav_action_for_swipe, level_select_menu_start_for_nav,
    level_select_menu_target_at, level_set_select_nav_action_at, level_set_select_start_for_nav,
    level_set_select_target_at, overlay_primary_action_button_contains,
    overlay_secondary_action_button_contains, top_menu_toggle_button_expanded_hit_contains,
};
pub use layout::{
    BOARD_HORIZONTAL_MARGIN, BOARD_VERTICAL_MARGIN, BoardViewport, BoardViewportOptions,
    ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE, UI_MENU_BUTTON_HEIGHT, board_viewport_margins,
    bottom_left_corner_button_rect, bottom_right_corner_button_rect,
    editor_bottom_left_button_rect, editor_bottom_right_button_rect, editor_viewport_size,
    fit_board_viewport_for_controls, fit_board_viewport_for_controls_capped,
    gameplay_menu_level_set_button_rect, level_select_menu_clamp_start, level_select_menu_indices,
    level_select_menu_slot_rects, level_select_menu_start_index, level_select_menu_step_start,
    level_set_select_clamp_start, level_set_select_indices, level_set_select_row_rects,
    level_set_select_start_index, level_set_select_step_start, overlay_primary_action_button_rect,
    overlay_secondary_action_button_rect, top_left_level_button_rect,
    top_menu_toggle_button_expanded_hit_rect, top_menu_toggle_button_visible_rect,
};
pub use renderer::{
    EntityVisualStyle, FrameDamage, Gray, PIXEL_FONT_HEIGHT, Renderer, RendererOverrides,
    RendererTheme, draw_centered_text_in_rect, draw_controls_ui, draw_icon_bits_in_rect, draw_text,
    draw_top_left_level_button, draw_top_menu_toggle, measure_text_width,
};
pub use screen_requests::{
    EditorCountOverlay, EditorHintChange, EditorHintOverlay, EditorHintState,
    EditorMenuScreenRequest, EditorScreenRequest, FrameRequest, GameplayMenuScreenRequest,
    GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenMode,
    GameplayScreenRequest, LevelSelectScreenRequest, LevelSetListEntry,
    LevelSetSelectScreenRequest,
};
