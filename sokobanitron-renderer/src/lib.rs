mod background;
mod controls;
mod entities;
mod overlay;
mod pixels;
mod sprites;
mod tiles;
mod trail;
mod viewport;

use image::RgbaImage;
use sokobanitron_gameplay::BoardView;
use std::collections::HashMap;

pub use controls::{
    BOARD_HORIZONTAL_MARGIN, BOARD_VERTICAL_MARGIN, ControlsButtonAction, UI_BUTTON_MARGIN,
    UI_BUTTON_SIZE, UI_MENU_BUTTON_HEIGHT, board_viewport_margins, controls_button_action_at,
    draw_controls_ui,
};
pub use viewport::{BoardViewport, BoardViewportOptions};
pub type Rgba = [u8; 4];

const BG_SPACE_PNG: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/bg_space.png"));

pub fn fit_board_viewport_for_controls(
    width: u32,
    height: u32,
    board: &BoardView,
) -> BoardViewport {
    let (h_margin, v_margin) = board_viewport_margins();
    let safe_width = width.saturating_sub(h_margin * 2).max(1);
    let safe_height = height.saturating_sub(v_margin * 2).max(1);

    let mut viewport = BoardViewport::fit_to_window_with_options(
        safe_width,
        safe_height,
        board,
        BoardViewportOptions::fill_available_space(),
    );
    viewport.origin_x += h_margin as i32;
    viewport.origin_y += v_margin as i32;
    viewport
}

#[derive(Debug, Clone, Copy)]
pub struct RendererTheme {
    pub floor_fill: Rgba,
    pub floor_stroke: Rgba,
    pub target_fill: Rgba,
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
    pub target_fill: Option<Rgba>,
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
            target_fill: [224, 224, 224, 255],
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
        if let Some(v) = overrides.target_fill {
            self.target_fill = v;
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
    theme: RendererTheme,
    source_background: RgbaImage,
    cached_background: Vec<u8>,
    cached_width: u32,
    cached_height: u32,
    box_bitmap_cache: HashMap<u32, Vec<u8>>,
    selected_box_bitmap_cache: HashMap<u32, Vec<u8>>,
    player_bitmap_cache: HashMap<u32, Vec<u8>>,
    player_blink_bitmap_cache: HashMap<u32, Vec<u8>>,
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
            player_blink_bitmap_cache: HashMap::new(),
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
        self.draw_with_box_trail(frame, width, height, board, viewport, None);
    }

    pub fn draw_with_box_trail(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        box_trail: Option<&[(u32, u32)]>,
    ) {
        self.draw_with_box_trail_options(
            frame, width, height, board, viewport, box_trail, true, true,
        );
    }

    pub fn draw_with_box_trail_options(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        box_trail: Option<&[(u32, u32)]>,
        draw_player: bool,
        draw_win_overlay: bool,
    ) {
        self.draw_with_box_trail_progress_options(
            frame,
            width,
            height,
            board,
            viewport,
            box_trail,
            None,
            draw_player,
            draw_win_overlay,
        );
    }

    pub fn draw_with_box_trail_progress_options(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        box_trail: Option<&[(u32, u32)]>,
        box_trail_consumed_segments: Option<f32>,
        draw_player: bool,
        draw_win_overlay: bool,
    ) {
        self.draw_with_box_trail_progress_effects(
            frame,
            width,
            height,
            board,
            viewport,
            box_trail,
            box_trail_consumed_segments,
            draw_player,
            false,
            draw_win_overlay,
        );
    }

    pub fn draw_with_box_trail_progress_effects(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        box_trail: Option<&[(u32, u32)]>,
        box_trail_consumed_segments: Option<f32>,
        draw_player: bool,
        draw_player_blink: bool,
        draw_win_overlay: bool,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        self.ensure_cached_background(width, height);
        frame.copy_from_slice(&self.cached_background);
        self.draw_floor_tiles(frame, width, height, board, viewport);
        if let Some(path) = box_trail {
            if let Some(consumed) = box_trail_consumed_segments {
                self.draw_box_trail_with_progress(frame, width, height, viewport, path, consumed);
            } else {
                self.draw_box_trail(frame, width, height, viewport, path);
            }
        }
        self.draw_boxes(frame, width, height, board, viewport);
        if draw_player {
            self.draw_player(frame, width, height, board, viewport);
            if draw_player_blink {
                self.draw_player_blink(frame, width, height, board, viewport);
            }
        }
        if draw_win_overlay && board.is_won() {
            self.draw_win_overlay(frame, width, height);
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
