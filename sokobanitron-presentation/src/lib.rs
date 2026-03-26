//! Presentation and rendering support for Sokobanitron clients.
//!
//! The crate is organized around five public concepts:
//! `assets`, `layout`, `hit_test`, `screen_requests`, and `renderer`.
//!
//! Internally those responsibilities live in matching subdirectories so the crate can be read as
//! a swappable presentation system rather than a single flat renderer module.

pub mod assets;
pub mod hit_test;
pub mod layout;
pub mod renderer;
pub mod screen_requests;

pub use assets::{UiIcon, draw_ui_icon_in_rect};
pub use hit_test::{
    ControlsButtonAction, GameplaySurfaceLayer, GameplaySurfaceModel, GameplaySurfaceTarget,
    LevelSelectSurfaceTarget, MenuNavAction, controls_button_action_at,
    gameplay_surface_target_at, level_select_menu_nav_action_at,
    level_select_menu_start_for_nav, level_select_menu_target_at,
    overlay_primary_action_button_contains, top_menu_toggle_button_contains,
};
pub use layout::{
    BOARD_HORIZONTAL_MARGIN, BOARD_VERTICAL_MARGIN, BoardViewport, BoardViewportOptions,
    ControlsButtonRects, ControlsUiMode, ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE,
    UI_MENU_BUTTON_HEIGHT, board_viewport_margins, bottom_left_corner_button_rect,
    bottom_right_corner_button_rect, controls_button_rects, editor_bottom_left_button_rect,
    editor_bottom_right_button_rect, editor_viewport_size, fit_board_viewport_for_controls,
    level_select_menu_clamp_start, level_select_menu_indices, level_select_menu_slot_rects,
    level_select_menu_start_index, level_select_menu_step_start,
    overlay_primary_action_button_rect, top_left_level_button_rect,
    top_menu_toggle_button_hit_rect, top_menu_toggle_button_rect,
};
pub use renderer::{
    PIXEL_FONT_HEIGHT, Renderer, RendererOverrides, RendererTheme, Rgba,
    draw_centered_text_in_rect, draw_controls_ui, draw_icon_bits_in_rect,
    draw_overlay_primary_action_button, draw_top_left_level_button, draw_top_menu_toggle,
    measure_text_width,
};
pub use screen_requests::{
    EditorCountOverlay, EditorHintOverlay, EditorMenuScreenRequest, EditorScreenRequest,
    GameplayMenuScreenRequest, GameplayScreenRequest, LevelSelectScreenRequest,
};
