pub const WIDTH: usize = 1072;
pub const HEIGHT: usize = 1448;
pub const STRIDE: usize = 1088;

pub const TOUCH_DEVICE: &str = "/dev/input/event1";
pub const FRAMEBUFFER_DEVICE: &str = "/dev/fb0";
pub const REFRESH_DEVICE: &str = "/sys/devices/platform/imx_epdc_fb/mxc_epdc_update";

pub const TOUCH_MIN_X: i32 = 0;
pub const TOUCH_MAX_X: i32 = WIDTH as i32 - 1;
pub const TOUCH_MIN_Y: i32 = 0;
pub const TOUCH_MAX_Y: i32 = HEIGHT as i32 - 1;

pub const UI_BUTTON_SIZE: usize = 76;
pub const UI_BUTTON_MARGIN: usize = 16;
pub const UI_TEXT_SCALE: usize = 8;

pub const KINDLE_SELECTED_BOX_PRIMARY: [u8; 4] = [28, 28, 28, 255];
pub const KINDLE_SELECTED_BOX_HIGHLIGHT: [u8; 4] = [64, 64, 64, 255];
pub const KINDLE_SELECTED_BOX_SHADOW: [u8; 4] = [0, 0, 0, 255];
