use crate::layout::{controls_button_rects, top_menu_toggle_button_hit_rect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlsButtonAction {
    ShowMenu,
    Restart,
    Undo,
}

pub fn top_menu_toggle_button_contains(px: f64, py: f64, width: u32) -> bool {
    top_menu_toggle_button_hit_rect(width).contains(px, py)
}

pub fn controls_button_action_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
    can_undo: bool,
    can_restart: bool,
) -> Option<ControlsButtonAction> {
    if top_menu_toggle_button_contains(px, py, width) {
        return Some(ControlsButtonAction::ShowMenu);
    }

    let controls = controls_button_rects(width, height);
    if can_undo && controls.undo.contains(px, py) {
        return Some(ControlsButtonAction::Undo);
    }
    if can_restart && controls.restart.contains(px, py) {
        return Some(ControlsButtonAction::Restart);
    }
    None
}
