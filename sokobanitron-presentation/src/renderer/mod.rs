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
mod level_set_select;
mod pixel_ui;
mod pixels;
mod tiles;

use image::RgbaImage;
use sokobanitron_gameplay::BoardView;
use std::collections::HashMap;

use crate::layout::{BoardViewport, ScreenRect};

pub use chrome::{
    draw_controls_ui, draw_gameplay_menu_level_set_button, draw_overlay_primary_action_button,
    draw_top_left_level_button, draw_top_menu_toggle,
};
pub use pixel_ui::{
    PIXEL_FONT_HEIGHT, draw_centered_text_in_rect, draw_icon_bits_in_rect, draw_text,
    measure_text_width,
};
pub(crate) use pixels::blit_rgba;

pub type Rgba = [u8; 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerSceneComposition {
    pub visible: bool,
    pub sleeping: bool,
}

impl Default for PlayerSceneComposition {
    fn default() -> Self {
        Self {
            visible: true,
            sleeping: false,
        }
    }
}

// Keep under/over layers distinct even before they have behavior so future animation work can
// grow them independently without smuggling over-entity concerns into the under-entity seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct UnderEntitySceneComposition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OverEntitySceneComposition {
    pub entity_visual_style: EntityVisualStyle,
}

impl Default for OverEntitySceneComposition {
    fn default() -> Self {
        Self {
            entity_visual_style: EntityVisualStyle::Standard,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct BoardSceneComposition {
    pub player: PlayerSceneComposition,
    pub under_entities: UnderEntitySceneComposition,
    pub over_entities: OverEntitySceneComposition,
}

impl BoardSceneComposition {
    pub(crate) fn static_scene() -> Self {
        Self {
            player: PlayerSceneComposition::default(),
            under_entities: UnderEntitySceneComposition,
            over_entities: OverEntitySceneComposition::default(),
        }
    }

    pub(crate) fn gameplay_snapshot(
        entity_visual_style: EntityVisualStyle,
        sleeping_player: bool,
    ) -> Self {
        Self {
            player: PlayerSceneComposition {
                visible: true,
                sleeping: sleeping_player,
            },
            under_entities: UnderEntitySceneComposition,
            over_entities: OverEntitySceneComposition {
                entity_visual_style,
            },
        }
    }
}

const BG_SPACE_PNG: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/bg_space.png"));
pub(crate) const WHITE: Rgba = [255, 255, 255, 255];
pub(crate) const BLACK: Rgba = [0, 0, 0, 255];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EntityVisualStyle {
    #[default]
    Standard,
    Solved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BoardSceneCacheKey {
    pub surface_width: u32,
    pub surface_height: u32,
    pub viewport: BoardViewport,
    pub board_width: u32,
    pub board_height: u32,
    pub tile_signature: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct RendererTheme {
    pub light_1: Rgba,
    pub light_2: Rgba,
    pub mid_1: Rgba,
    pub mid_2: Rgba,
    pub mid_3: Rgba,
    pub mid_4: Rgba,
    pub mid_5: Rgba,
    pub dark_1: Rgba,
    pub dark_2: Rgba,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RendererOverrides {
    pub light_1: Option<Rgba>,
    pub light_2: Option<Rgba>,
    pub mid_1: Option<Rgba>,
    pub mid_2: Option<Rgba>,
    pub mid_3: Option<Rgba>,
    pub mid_4: Option<Rgba>,
    pub mid_5: Option<Rgba>,
    pub dark_1: Option<Rgba>,
    pub dark_2: Option<Rgba>,
}

impl Default for RendererTheme {
    fn default() -> Self {
        Self {
            light_1: [240, 240, 240, 255],
            light_2: [224, 224, 224, 255],
            mid_1: [156, 163, 175, 255],
            mid_2: [123, 133, 150, 255],
            mid_3: [120, 129, 144, 255],
            mid_4: [107, 114, 128, 255],
            mid_5: [95, 103, 117, 255],
            dark_1: [75, 85, 99, 255],
            dark_2: [74, 81, 96, 255],
        }
    }
}

impl RendererTheme {
    pub fn apply_overrides(mut self, overrides: RendererOverrides) -> Self {
        if let Some(v) = overrides.light_1 {
            self.light_1 = v;
        }
        if let Some(v) = overrides.light_2 {
            self.light_2 = v;
        }
        if let Some(v) = overrides.mid_1 {
            self.mid_1 = v;
        }
        if let Some(v) = overrides.mid_2 {
            self.mid_2 = v;
        }
        if let Some(v) = overrides.mid_3 {
            self.mid_3 = v;
        }
        if let Some(v) = overrides.mid_4 {
            self.mid_4 = v;
        }
        if let Some(v) = overrides.mid_5 {
            self.mid_5 = v;
        }
        if let Some(v) = overrides.dark_1 {
            self.dark_1 = v;
        }
        if let Some(v) = overrides.dark_2 {
            self.dark_2 = v;
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
    pub(crate) cached_board_scene: Vec<u8>,
    pub(crate) cached_board_scene_key: Option<BoardSceneCacheKey>,
    pub(crate) box_bitmap_cache: HashMap<u32, Vec<u8>>,
    pub(crate) solved_box_bitmap_cache: HashMap<u32, Vec<u8>>,
    pub(crate) selected_box_bitmap_cache: HashMap<u32, Vec<u8>>,
    pub(crate) player_bitmap_cache: HashMap<u32, Vec<u8>>,
    pub(crate) blink_player_bitmap_cache: HashMap<u32, Vec<u8>>,
    pub(crate) squint_player_bitmap_cache: HashMap<u32, Vec<u8>>,
    pub(crate) sleeping_player_bitmap_cache: HashMap<u32, Vec<u8>>,
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
            cached_board_scene: Vec::new(),
            cached_board_scene_key: None,
            box_bitmap_cache: HashMap::new(),
            solved_box_bitmap_cache: HashMap::new(),
            selected_box_bitmap_cache: HashMap::new(),
            player_bitmap_cache: HashMap::new(),
            blink_player_bitmap_cache: HashMap::new(),
            squint_player_bitmap_cache: HashMap::new(),
            sleeping_player_bitmap_cache: HashMap::new(),
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
        self.draw_board_scene_on_frame(
            frame,
            width,
            height,
            board,
            viewport,
            BoardSceneComposition::static_scene(),
        );
    }

    pub fn draw_background_only(&mut self, frame: &mut [u8], width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.ensure_cached_background(width, height);
        frame.copy_from_slice(&self.cached_background);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_board_scene_on_frame(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        composition: BoardSceneComposition,
    ) {
        self.draw_board_layers_on_frame(
            frame,
            width,
            height,
            board,
            viewport,
            composition,
            BoardBaseLayer::CachedScene,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_board_on_frame_with_composition(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        composition: BoardSceneComposition,
    ) {
        self.draw_board_layers_on_frame(
            frame,
            width,
            height,
            board,
            viewport,
            composition,
            BoardBaseLayer::Tiles,
        );
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
        entity_visual_style: EntityVisualStyle,
        sleeping_player: bool,
    ) {
        let mut composition =
            BoardSceneComposition::gameplay_snapshot(entity_visual_style, sleeping_player);
        composition.player.visible = draw_player;
        self.draw_board_on_frame_with_composition(
            frame,
            width,
            height,
            board,
            viewport,
            composition,
        );
    }

    fn ensure_cached_board_scene(
        &mut self,
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
    ) {
        let key = BoardSceneCacheKey {
            surface_width: width,
            surface_height: height,
            viewport: *viewport,
            board_width: board.width(),
            board_height: board.height(),
            tile_signature: tile_signature(board),
        };
        if self.cached_board_scene_key == Some(key) {
            return;
        }

        self.ensure_cached_background(width, height);
        let mut cached_board_scene = self.cached_background.clone();
        self.draw_floor_tiles(&mut cached_board_scene, width, height, board, viewport);
        self.cached_board_scene = cached_board_scene;
        self.cached_board_scene_key = Some(key);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_board_layers_on_frame(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        composition: BoardSceneComposition,
        base_layer: BoardBaseLayer,
    ) {
        if width == 0 || height == 0 {
            return;
        }
        self.draw_board_base_layer_on_frame(frame, width, height, board, viewport, base_layer);
        self.draw_board_under_entity_layer_on_frame(
            frame,
            width,
            height,
            board,
            viewport,
            composition.under_entities,
        );
        self.draw_board_entity_layer_on_frame(
            frame,
            width,
            height,
            board,
            viewport,
            composition.player,
            composition.over_entities,
        );
        self.draw_board_over_entity_layer_on_frame(
            frame,
            width,
            height,
            board,
            viewport,
            composition.over_entities,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_board_base_layer_on_frame(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        base_layer: BoardBaseLayer,
    ) {
        match base_layer {
            BoardBaseLayer::CachedScene => {
                self.ensure_cached_board_scene(width, height, board, viewport);
                frame.copy_from_slice(&self.cached_board_scene);
            }
            BoardBaseLayer::Tiles => self.draw_floor_tiles(frame, width, height, board, viewport),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_board_under_entity_layer_on_frame(
        &mut self,
        _frame: &mut [u8],
        _width: u32,
        _height: u32,
        _board: &BoardView,
        _viewport: &BoardViewport,
        _composition: UnderEntitySceneComposition,
    ) {
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_board_entity_layer_on_frame(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        player: PlayerSceneComposition,
        over_entities: OverEntitySceneComposition,
    ) {
        self.draw_boxes(
            frame,
            width,
            height,
            board,
            viewport,
            over_entities.entity_visual_style,
        );
        if player.visible {
            self.draw_player(
                frame,
                width,
                height,
                board,
                viewport,
                over_entities.entity_visual_style,
                player.sleeping,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_board_over_entity_layer_on_frame(
        &mut self,
        _frame: &mut [u8],
        _width: u32,
        _height: u32,
        _board: &BoardView,
        _viewport: &BoardViewport,
        _composition: OverEntitySceneComposition,
    ) {
    }

    // Internal composition helper used by chrome overlays that need to reveal the cached
    // background art without redrawing the full scene underneath.
    pub(crate) fn restore_background_rect(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        rect: ScreenRect,
    ) {
        if width == 0 || height == 0 || rect.w == 0 || rect.h == 0 {
            return;
        }
        self.ensure_cached_background(width, height);
        copy_rect_rgba(
            frame,
            width,
            height,
            &self.cached_background,
            self.cached_width,
            rect,
        );
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BoardBaseLayer {
    CachedScene,
    Tiles,
}

fn copy_rect_rgba(
    dst: &mut [u8],
    dst_width: u32,
    dst_height: u32,
    src: &[u8],
    src_width: u32,
    rect: ScreenRect,
) {
    let start_x = rect.x.min(dst_width) as usize;
    let end_x = rect.x.saturating_add(rect.w).min(dst_width) as usize;
    let start_y = rect.y.min(dst_height) as usize;
    let end_y = rect.y.saturating_add(rect.h).min(dst_height) as usize;
    if start_x >= end_x || start_y >= end_y {
        return;
    }

    let row_bytes = (end_x - start_x) * 4;
    for y in start_y..end_y {
        let dst_row_start = ((y * dst_width as usize) + start_x) * 4;
        let src_row_start = ((y * src_width as usize) + start_x) * 4;
        let dst_row_end = dst_row_start + row_bytes;
        let src_row_end = src_row_start + row_bytes;
        dst[dst_row_start..dst_row_end].copy_from_slice(&src[src_row_start..src_row_end]);
    }
}

fn tile_signature(board: &BoardView) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    hash ^= u64::from(board.width());
    hash = hash.wrapping_mul(0x100000001b3);
    hash ^= u64::from(board.height());
    hash = hash.wrapping_mul(0x100000001b3);
    for cell in board.cells() {
        let tile = match board.tile(cell) {
            sokobanitron_gameplay::TileKind::Void => 0_u64,
            sokobanitron_gameplay::TileKind::Floor => 1_u64,
            sokobanitron_gameplay::TileKind::Goal => 2_u64,
        };
        hash ^= tile;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
