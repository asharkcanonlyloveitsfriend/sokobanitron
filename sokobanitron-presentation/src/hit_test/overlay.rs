use crate::layout::{overlay_primary_action_button_rect, overlay_secondary_action_button_rect};

pub fn overlay_primary_action_button_contains(px: f64, py: f64, width: u32, height: u32) -> bool {
    overlay_primary_action_button_rect(width, height).contains(px, py)
}

pub fn overlay_secondary_action_button_contains(px: f64, py: f64, width: u32, height: u32) -> bool {
    overlay_secondary_action_button_rect(width, height).contains(px, py)
}
