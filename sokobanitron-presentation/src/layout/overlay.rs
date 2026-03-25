use super::{ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE, UI_MENU_BUTTON_HEIGHT};

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
