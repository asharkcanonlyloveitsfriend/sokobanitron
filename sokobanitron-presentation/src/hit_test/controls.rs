use crate::layout::top_menu_toggle_button_expanded_hit_rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlsButtonAction {
    Restart,
    Undo,
}

pub fn top_menu_toggle_button_expanded_hit_contains(px: f64, py: f64, width: u32) -> bool {
    top_menu_toggle_button_expanded_hit_rect(width).contains(px, py)
}
