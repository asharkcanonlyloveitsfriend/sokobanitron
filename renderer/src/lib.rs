mod background;
mod entities;
mod overlay;
mod pixels;
mod sprites;
mod tiles;
mod viewport;

use image::RgbaImage;
use sokobanitron_gameplay::BoardView;
use std::collections::HashMap;

pub use viewport::BoardViewport;

const BG_SPACE_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../android-client/app/src/main/res/drawable-nodpi/bg_space.png"
));

pub struct Renderer {
    source_background: RgbaImage,
    cached_background: Vec<u8>,
    cached_width: u32,
    cached_height: u32,
    box_bitmap_cache: HashMap<u32, Vec<u8>>,
    selected_box_bitmap_cache: HashMap<u32, Vec<u8>>,
    player_bitmap_cache: HashMap<u32, Vec<u8>>,
}

impl Renderer {
    pub fn new() -> Self {
        let source_background = image::load_from_memory(BG_SPACE_PNG)
            .expect("failed to decode bg_space.png")
            .into_rgba8();

        Self {
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
        if width == 0 || height == 0 {
            return;
        }
        self.ensure_cached_background(width, height);
        frame.copy_from_slice(&self.cached_background);
        self.draw_floor_tiles(frame, width, height, board, viewport);
        self.draw_boxes(frame, width, height, board, viewport);
        self.draw_player(frame, width, height, board, viewport);
        if board.is_won() {
            overlay::draw_win_overlay(frame, width, height);
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
