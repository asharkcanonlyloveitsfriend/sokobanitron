#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl ScreenRect {
    pub fn contains(self, px: f64, py: f64) -> bool {
        if px < 0.0 || py < 0.0 {
            return false;
        }
        let x = px as usize;
        let y = py as usize;
        x >= self.x as usize
            && x < self.x.saturating_add(self.w) as usize
            && y >= self.y as usize
            && y < self.y.saturating_add(self.h) as usize
    }
}

pub const UI_BUTTON_SIZE: u32 = 76;
pub const UI_BUTTON_MARGIN: u32 = 16;
pub const UI_MENU_BUTTON_HEIGHT: u32 = UI_BUTTON_SIZE / 2;
pub const BOARD_HORIZONTAL_MARGIN: u32 = UI_BUTTON_MARGIN;
pub const BOARD_VERTICAL_MARGIN: u32 = UI_BUTTON_MARGIN + UI_MENU_BUTTON_HEIGHT;

pub fn board_viewport_margins() -> (u32, u32) {
    (BOARD_HORIZONTAL_MARGIN, BOARD_VERTICAL_MARGIN)
}

pub fn top_menu_toggle_button_visible_rect(width: u32) -> ScreenRect {
    ScreenRect {
        x: width.saturating_sub(UI_BUTTON_SIZE) / 2,
        y: UI_BUTTON_MARGIN,
        w: UI_BUTTON_SIZE,
        h: UI_MENU_BUTTON_HEIGHT,
    }
}

pub fn top_menu_toggle_button_expanded_hit_rect(width: u32) -> ScreenRect {
    let base = top_menu_toggle_button_visible_rect(width);
    let extra_w = base.w / 2;
    let extra_h = base.h;
    ScreenRect {
        x: base.x.saturating_sub(extra_w / 2),
        y: base.y.saturating_sub(extra_h / 2),
        w: base.w.saturating_add(extra_w),
        h: base.h.saturating_add(extra_h),
    }
}

pub fn top_left_level_button_rect() -> ScreenRect {
    ScreenRect {
        x: UI_BUTTON_MARGIN,
        y: UI_BUTTON_MARGIN,
        w: UI_BUTTON_SIZE,
        h: UI_BUTTON_SIZE,
    }
}

pub fn bottom_left_corner_button_rect(height: u32) -> ScreenRect {
    ScreenRect {
        x: UI_BUTTON_MARGIN,
        y: height.saturating_sub(UI_BUTTON_MARGIN + UI_BUTTON_SIZE),
        w: UI_BUTTON_SIZE,
        h: UI_BUTTON_SIZE,
    }
}

pub fn bottom_right_corner_button_rect(width: u32, height: u32) -> ScreenRect {
    ScreenRect {
        x: width.saturating_sub(UI_BUTTON_MARGIN + UI_BUTTON_SIZE),
        y: height.saturating_sub(UI_BUTTON_MARGIN + UI_BUTTON_SIZE),
        w: UI_BUTTON_SIZE,
        h: UI_BUTTON_SIZE,
    }
}
