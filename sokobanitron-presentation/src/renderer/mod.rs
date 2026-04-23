//! Drawing implementation for the presentation system.
//!
//! `Renderer` owns device-agnostic pixel composition for shared presentation requests. It draws
//! into caller-provided grayscale buffers, but it does not own frame scheduling, invalidation, or
//! platform refresh policy. Those remain client concerns.

mod background;
mod chrome;
mod editor;
mod entities;
mod frame;
mod gameplay;
mod level_select;
mod level_select_scrollbar;
mod level_set_select;
mod pixel_ui;
mod pixels;
mod tiles;

use image::GrayImage;
use sokobanitron_gameplay::BoardView;
use std::collections::HashMap;

use crate::layout::{BoardViewport, ScreenRect};

pub use chrome::{draw_controls_ui, draw_top_left_level_button, draw_top_menu_toggle};
pub use frame::{FrameDamage, FrameRenderResult};
pub use pixel_ui::{
    PIXEL_FONT_HEIGHT, draw_centered_text_in_rect, draw_icon_bits_in_rect, draw_text,
    measure_text_width,
};
pub(crate) use pixels::{
    blit_premultiplied_gray_alpha, fill_rect, premultiply_straight_gray, rgba_to_gray,
};

pub type Gray = u8;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EntityVisualStyle {
    #[default]
    Standard,
    SolvedClean,
    SolvedDirty,
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
    pub white: Gray,
    pub gray_1: Gray,
    pub gray_2: Gray,
    pub gray_3: Gray,
    pub gray_4: Gray,
    pub gray_5: Gray,
    pub gray_6: Gray,
    pub gray_7: Gray,
    pub gray_8: Gray,
    pub gray_9: Gray,
    pub gray_10: Gray,
    pub gray_11: Gray,
    pub gray_12: Gray,
    pub gray_13: Gray,
    pub gray_14: Gray,
    pub black: Gray,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RendererOverrides {
    pub gray_1: Option<Gray>,
    pub gray_2: Option<Gray>,
    pub gray_3: Option<Gray>,
    pub gray_4: Option<Gray>,
    pub gray_5: Option<Gray>,
    pub gray_6: Option<Gray>,
    pub gray_7: Option<Gray>,
    pub gray_8: Option<Gray>,
    pub gray_9: Option<Gray>,
    pub gray_10: Option<Gray>,
    pub gray_11: Option<Gray>,
    pub gray_12: Option<Gray>,
    pub gray_13: Option<Gray>,
    pub gray_14: Option<Gray>,
}

impl Default for RendererTheme {
    fn default() -> Self {
        Self {
            white: 255,
            gray_1: 238,
            gray_2: 221,
            gray_3: 204,
            gray_4: 187,
            gray_5: 170,
            gray_6: 153,
            gray_7: 136,
            gray_8: 119,
            gray_9: 102,
            gray_10: 85,
            gray_11: 68,
            gray_12: 51,
            gray_13: 34,
            gray_14: 17,
            black: 0,
        }
    }
}

impl RendererTheme {
    pub fn apply_overrides(mut self, overrides: RendererOverrides) -> Self {
        if let Some(v) = overrides.gray_1 {
            self.gray_1 = v;
        }
        if let Some(v) = overrides.gray_2 {
            self.gray_2 = v;
        }
        if let Some(v) = overrides.gray_3 {
            self.gray_3 = v;
        }
        if let Some(v) = overrides.gray_4 {
            self.gray_4 = v;
        }
        if let Some(v) = overrides.gray_5 {
            self.gray_5 = v;
        }
        if let Some(v) = overrides.gray_6 {
            self.gray_6 = v;
        }
        if let Some(v) = overrides.gray_7 {
            self.gray_7 = v;
        }
        if let Some(v) = overrides.gray_8 {
            self.gray_8 = v;
        }
        if let Some(v) = overrides.gray_9 {
            self.gray_9 = v;
        }
        if let Some(v) = overrides.gray_10 {
            self.gray_10 = v;
        }
        if let Some(v) = overrides.gray_11 {
            self.gray_11 = v;
        }
        if let Some(v) = overrides.gray_12 {
            self.gray_12 = v;
        }
        if let Some(v) = overrides.gray_13 {
            self.gray_13 = v;
        }
        if let Some(v) = overrides.gray_14 {
            self.gray_14 = v;
        }
        self
    }
}

pub struct Renderer {
    pub(crate) theme: RendererTheme,
    pub(crate) source_background: GrayImage,
    pub(crate) cached_background: Vec<u8>,
    pub(crate) cached_width: u32,
    pub(crate) cached_height: u32,
    pub(crate) cached_board_scene: Vec<u8>,
    pub(crate) cached_board_scene_key: Option<BoardSceneCacheKey>,
    pub(crate) box_bitmap_cache: HashMap<u32, Vec<u8>>,
    pub(crate) editor_disabled_box_bitmap_cache: HashMap<u32, Vec<u8>>,
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
            .into_luma8();

        Self {
            theme,
            source_background,
            cached_background: Vec::new(),
            cached_width: 0,
            cached_height: 0,
            cached_board_scene: Vec::new(),
            cached_board_scene_key: None,
            box_bitmap_cache: HashMap::new(),
            editor_disabled_box_bitmap_cache: HashMap::new(),
            solved_box_bitmap_cache: HashMap::new(),
            selected_box_bitmap_cache: HashMap::new(),
            player_bitmap_cache: HashMap::new(),
            blink_player_bitmap_cache: HashMap::new(),
            squint_player_bitmap_cache: HashMap::new(),
            sleeping_player_bitmap_cache: HashMap::new(),
        }
    }

    pub fn theme(&self) -> RendererTheme {
        self.theme
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
        copy_rect_gray(
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

fn copy_rect_gray(
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

    let row_bytes = end_x - start_x;
    for y in start_y..end_y {
        let dst_row_start = (y * dst_width as usize) + start_x;
        let src_row_start = (y * src_width as usize) + start_x;
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
