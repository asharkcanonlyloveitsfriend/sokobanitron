use super::{
    BoardViewport, ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE, bottom_left_corner_button_rect,
    bottom_right_corner_button_rect,
};

const EDITOR_MODE_BUTTON_WIDTH: u32 = 136;
const EDITOR_MODE_MENU_WIDTH: u32 = 192;
const EDITOR_MODE_MENU_ROW_HEIGHT: u32 = 108;
const EDITOR_MODE_MENU_OPTIONS: usize = 3;
const EDITOR_MODE_MENU_HEIGHT: u32 = EDITOR_MODE_MENU_ROW_HEIGHT * EDITOR_MODE_MENU_OPTIONS as u32;

pub fn editor_bottom_left_button_rect(height: u32) -> ScreenRect {
    bottom_left_corner_button_rect(height)
}

pub fn editor_bottom_right_button_rect(width: u32, height: u32) -> ScreenRect {
    bottom_right_corner_button_rect(width, height)
}

pub fn editor_mode_button_rect(width: u32, height: u32) -> ScreenRect {
    let x = UI_BUTTON_MARGIN.min(width.saturating_sub(1));
    let y = UI_BUTTON_MARGIN.min(height.saturating_sub(1));
    let max_w = width.saturating_sub(x);
    let max_h = height.saturating_sub(y);

    ScreenRect {
        x,
        y,
        w: EDITOR_MODE_BUTTON_WIDTH.min(max_w).max(1),
        h: UI_BUTTON_SIZE.min(max_h).max(1),
    }
}

pub fn editor_mode_menu_rect(width: u32, height: u32) -> ScreenRect {
    let x = UI_BUTTON_MARGIN.min(width.saturating_sub(1));
    let y = UI_BUTTON_MARGIN.min(height.saturating_sub(1));
    let max_w = width.saturating_sub(x);
    let max_h = height.saturating_sub(y);

    ScreenRect {
        x,
        y,
        w: EDITOR_MODE_MENU_WIDTH.min(max_w).max(1),
        h: EDITOR_MODE_MENU_HEIGHT.min(max_h).max(1),
    }
}

pub fn editor_mode_menu_damage_rect(width: u32, height: u32) -> ScreenRect {
    let menu = editor_mode_menu_rect(width, height);
    ScreenRect {
        x: 0,
        y: 0,
        w: menu.x.saturating_add(menu.w).min(width),
        h: menu.y.saturating_add(menu.h).min(height),
    }
}

pub fn editor_mode_menu_option_rects(width: u32, height: u32) -> [ScreenRect; 3] {
    let menu = editor_mode_menu_rect(width, height);
    let row_h = menu.h / EDITOR_MODE_MENU_OPTIONS as u32;
    let remainder = menu.h % EDITOR_MODE_MENU_OPTIONS as u32;
    let mut y = menu.y;
    std::array::from_fn(|index| {
        let h = row_h + u32::from(index == EDITOR_MODE_MENU_OPTIONS - 1) * remainder;
        let rect = ScreenRect {
            x: menu.x,
            y,
            w: menu.w,
            h,
        };
        y = y.saturating_add(h);
        rect
    })
}

pub fn editor_viewport_size(width: u32, height: u32, viewport: &BoardViewport) -> (u32, u32) {
    (
        width.max(viewport.board_pixel_width + UI_BUTTON_MARGIN * 2),
        height.max(viewport.board_pixel_height + UI_BUTTON_SIZE),
    )
}
