//! Drawing implementation for the presentation system.
//!
//! `Renderer` owns device-agnostic pixel composition for shared presentation requests. It draws
//! into caller-provided RGBA buffers, but it does not own frame scheduling, invalidation, or
//! platform refresh policy. Those remain client concerns.

mod background;
mod chrome;
mod editor;
mod entities;
mod gameplay;
mod level_select;
mod level_select_scrollbar;
mod overlay;
mod pixel_ui;
mod pixels;
mod tiles;

use image::RgbaImage;
use sokobanitron_gameplay::BoardView;
use std::collections::HashMap;

use crate::layout::BoardViewport;

pub use chrome::{
    draw_controls_ui, draw_overlay_primary_action_button, draw_top_left_level_button,
    draw_top_menu_toggle,
};
pub use pixel_ui::{
    PIXEL_FONT_HEIGHT, draw_centered_text_in_rect, draw_icon_bits_in_rect, measure_text_width,
};

pub type Rgba = [u8; 4];

const BG_SPACE_PNG: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/bg_space.png"));

#[derive(Debug, Clone, Copy)]
pub struct RendererTheme {
    pub floor_fill: Rgba,
    pub floor_stroke: Rgba,
    pub goal_fill: Rgba,
    pub box_primary: Rgba,
    pub box_highlight: Rgba,
    pub box_shadow: Rgba,
    pub selected_box_primary: Rgba,
    pub selected_box_highlight: Rgba,
    pub selected_box_shadow: Rgba,
    pub player_body: Rgba,
    pub player_highlight: Rgba,
    pub player_eye: Rgba,
    pub player_limb: Rgba,
    pub win_panel_fill: Rgba,
    pub win_panel_stroke: Rgba,
    pub win_text: Rgba,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RendererOverrides {
    pub floor_fill: Option<Rgba>,
    pub floor_stroke: Option<Rgba>,
    pub goal_fill: Option<Rgba>,
    pub box_primary: Option<Rgba>,
    pub box_highlight: Option<Rgba>,
    pub box_shadow: Option<Rgba>,
    pub selected_box_primary: Option<Rgba>,
    pub selected_box_highlight: Option<Rgba>,
    pub selected_box_shadow: Option<Rgba>,
    pub player_body: Option<Rgba>,
    pub player_highlight: Option<Rgba>,
    pub player_eye: Option<Rgba>,
    pub player_limb: Option<Rgba>,
    pub win_panel_fill: Option<Rgba>,
    pub win_panel_stroke: Option<Rgba>,
    pub win_text: Option<Rgba>,
}

impl Default for RendererTheme {
    fn default() -> Self {
        Self {
            floor_fill: [255, 255, 255, 255],
            floor_stroke: [240, 240, 240, 255],
            goal_fill: [224, 224, 224, 255],
            box_primary: [120, 129, 144, 255],
            box_highlight: [156, 163, 175, 255],
            box_shadow: [75, 85, 99, 255],
            selected_box_primary: [95, 103, 117, 255],
            selected_box_highlight: [123, 133, 150, 255],
            selected_box_shadow: [74, 81, 96, 255],
            player_body: [156, 163, 175, 255],
            player_highlight: [249, 250, 251, 255],
            player_eye: [2, 6, 23, 255],
            player_limb: [107, 114, 128, 255],
            win_panel_fill: [8, 12, 20, 220],
            win_panel_stroke: [255, 255, 255, 255],
            win_text: [255, 255, 255, 255],
        }
    }
}

impl RendererTheme {
    pub fn apply_overrides(mut self, overrides: RendererOverrides) -> Self {
        if let Some(v) = overrides.floor_fill {
            self.floor_fill = v;
        }
        if let Some(v) = overrides.floor_stroke {
            self.floor_stroke = v;
        }
        if let Some(v) = overrides.goal_fill {
            self.goal_fill = v;
        }
        if let Some(v) = overrides.box_primary {
            self.box_primary = v;
        }
        if let Some(v) = overrides.box_highlight {
            self.box_highlight = v;
        }
        if let Some(v) = overrides.box_shadow {
            self.box_shadow = v;
        }
        if let Some(v) = overrides.selected_box_primary {
            self.selected_box_primary = v;
        }
        if let Some(v) = overrides.selected_box_highlight {
            self.selected_box_highlight = v;
        }
        if let Some(v) = overrides.selected_box_shadow {
            self.selected_box_shadow = v;
        }
        if let Some(v) = overrides.player_body {
            self.player_body = v;
        }
        if let Some(v) = overrides.player_highlight {
            self.player_highlight = v;
        }
        if let Some(v) = overrides.player_eye {
            self.player_eye = v;
        }
        if let Some(v) = overrides.player_limb {
            self.player_limb = v;
        }
        if let Some(v) = overrides.win_panel_fill {
            self.win_panel_fill = v;
        }
        if let Some(v) = overrides.win_panel_stroke {
            self.win_panel_stroke = v;
        }
        if let Some(v) = overrides.win_text {
            self.win_text = v;
        }
        self
    }
}

pub struct Renderer {
    pub(crate) theme: RendererTheme,
    pub(crate) source_background: RgbaImage,
    pub(crate) cached_background: Vec<u8>,
    pub(crate) cached_width: u32,
    pub(crate) cached_height: u32,
    pub(crate) box_bitmap_cache: HashMap<u32, Vec<u8>>,
    pub(crate) selected_box_bitmap_cache: HashMap<u32, Vec<u8>>,
    pub(crate) player_bitmap_cache: HashMap<u32, Vec<u8>>,
}

impl Renderer {
    pub fn new() -> Self {
        Self::with_theme(RendererTheme::default())
    }

    pub fn with_overrides(overrides: RendererOverrides) -> Self {
        Self::with_theme(RendererTheme::default().apply_overrides(overrides))
    }

    pub fn with_theme(theme: RendererTheme) -> Self {
        let source_background = image::load_from_memory(BG_SPACE_PNG)
            .expect("failed to decode bg_space.png")
            .into_rgba8();

        Self {
            theme,
            source_background,
            cached_background: Vec::new(),
            cached_width: 0,
            cached_height: 0,
            box_bitmap_cache: HashMap::new(),
            selected_box_bitmap_cache: HashMap::new(),
            player_bitmap_cache: HashMap::new(),
        }
    }

    pub fn draw(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
    ) {
        self.draw_background_only(frame, width, height);
        self.draw_board_on_frame(frame, width, height, board, viewport, true, true);
    }

    pub fn draw_background_only(&mut self, frame: &mut [u8], width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.ensure_cached_background(width, height);
        frame.copy_from_slice(&self.cached_background);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_board_on_frame(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        draw_player: bool,
        draw_win_overlay: bool,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        self.draw_floor_tiles(frame, width, height, board, viewport);
        self.draw_boxes(frame, width, height, board, viewport);
        if draw_player {
            self.draw_player(frame, width, height, board, viewport);
        }
        if draw_win_overlay && board.is_solved() {
            self.draw_win_overlay(frame, width, height);
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
