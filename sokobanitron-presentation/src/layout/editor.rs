use super::{
    BoardViewport, ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE, bottom_left_corner_button_rect,
    bottom_right_corner_button_rect,
};

pub fn editor_bottom_left_button_rect(height: u32) -> ScreenRect {
    bottom_left_corner_button_rect(height)
}

pub fn editor_bottom_right_button_rect(width: u32, height: u32) -> ScreenRect {
    bottom_right_corner_button_rect(width, height)
}

pub fn editor_viewport_size(width: u32, height: u32, viewport: &BoardViewport) -> (u32, u32) {
    (
        width.max(viewport.board_pixel_width + UI_BUTTON_MARGIN * 2),
        height.max(viewport.board_pixel_height + UI_BUTTON_SIZE),
    )
}
