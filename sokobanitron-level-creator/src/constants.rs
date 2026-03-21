use std::time::Duration;

pub const INITIAL_WIDTH: u32 = 670;
pub const INITIAL_HEIGHT: u32 = 891;
pub const GRID_MARGIN_TILES: u32 = 1;
pub const BASE_VISIBLE_COLS: u32 = 9;
pub const MIN_VISIBLE_COLS: u32 = 5;
pub const MAX_VISIBLE_COLS: u32 = 25;
pub const INITIAL_PATCH_SIZE: i32 = 3;
pub const UI_TEXT_SCALE: usize = 4;
pub const MODE_ICON_SCALE: usize = 3;
pub const BUTTON_TEXT_COLOR: [u8; 4] = [220, 220, 220, 255];
pub const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(325);
pub const MODE_ICON_SIZE: usize = 9;
