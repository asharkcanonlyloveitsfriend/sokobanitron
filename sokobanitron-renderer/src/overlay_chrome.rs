use crate::controls::{ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE, UI_MENU_BUTTON_HEIGHT};
use crate::icons::{UiIcon, draw_ui_icon_in_rect};

pub fn overlay_primary_action_button_rect(width: u32, height: u32) -> ScreenRect {
    let desired_w = UI_BUTTON_SIZE.saturating_mul(2);
    let w = desired_w.min(width.saturating_sub(UI_BUTTON_MARGIN.saturating_mul(2)));
    let h = UI_BUTTON_SIZE.saturating_add(UI_MENU_BUTTON_HEIGHT / 2);
    let x = width.saturating_sub(w) / 2;
    let y = (UI_BUTTON_MARGIN.saturating_mul(2)).saturating_add(UI_MENU_BUTTON_HEIGHT);
    let max_y = height.saturating_sub(h).saturating_sub(UI_BUTTON_MARGIN);
    ScreenRect {
        x,
        y: y.min(max_y),
        w,
        h,
    }
}

pub fn overlay_primary_action_button_contains(px: f64, py: f64, width: u32, height: u32) -> bool {
    overlay_primary_action_button_rect(width, height).contains(px, py)
}

pub fn draw_overlay_primary_action_button(
    frame: &mut [u8],
    width: u32,
    height: u32,
    icon: UiIcon,
    color: [u8; 4],
) {
    let rect = overlay_primary_action_button_rect(width, height);
    draw_ui_icon_in_rect(frame, width, height, rect, icon, color);
}
