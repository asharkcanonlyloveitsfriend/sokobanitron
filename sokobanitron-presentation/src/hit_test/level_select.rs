use crate::layout::level_select_scrollbar::{ScrollbarTapTarget, tap_target_at};
use crate::layout::{
    level_select_menu_clamp_start, level_select_menu_indices, level_select_menu_slot_rects,
    level_select_menu_start_index, level_select_menu_step_start,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuNavAction {
    First,
    PageUp,
    Current,
    PageDown,
    Last,
}

pub fn level_select_menu_start_for_nav(
    level_count: usize,
    current_level: usize,
    current_start: usize,
    action: MenuNavAction,
) -> usize {
    match action {
        MenuNavAction::First => 0,
        MenuNavAction::PageUp => level_select_menu_step_start(level_count, current_start, -1),
        MenuNavAction::Current => level_select_menu_start_index(level_count, current_level),
        MenuNavAction::PageDown => level_select_menu_step_start(level_count, current_start, 1),
        MenuNavAction::Last => level_select_menu_clamp_start(level_count, usize::MAX),
    }
}

pub fn level_select_menu_nav_action_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
    level_count: usize,
    current_level: usize,
    current_start: usize,
) -> Option<MenuNavAction> {
    let return_start = level_select_menu_start_index(level_count, current_level);
    let visible_count = crate::layout::menu_slots_per_page().min(level_count).max(1);
    tap_target_at(
        px,
        py,
        width,
        height,
        level_count,
        visible_count,
        current_start,
        return_start,
    )
    .map(|target| match target {
        ScrollbarTapTarget::First => MenuNavAction::First,
        ScrollbarTapTarget::PageUp => MenuNavAction::PageUp,
        ScrollbarTapTarget::Current => MenuNavAction::Current,
        ScrollbarTapTarget::PageDown => MenuNavAction::PageDown,
        ScrollbarTapTarget::Last => MenuNavAction::Last,
    })
}

pub fn level_select_menu_target_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
    level_count: usize,
    start: usize,
) -> Option<usize> {
    if px < 0.0 || py < 0.0 {
        return None;
    }
    let x = px as i32;
    let y = py as i32;
    let slot = level_select_menu_slot_rects(width, height)
        .into_iter()
        .enumerate()
        .find_map(|(idx, (rx, ry, rw, rh))| {
            (x >= rx && y >= ry && x < rx + rw as i32 && y < ry + rh as i32).then_some(idx)
        })?;
    level_select_menu_indices(level_count, start)[slot]
}
