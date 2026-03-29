use crate::layout::level_select_scrollbar::{ScrollbarState, ScrollbarTapTarget, tap_target_at};
use crate::layout::{
    level_select_menu_clamp_start, level_select_menu_indices, level_select_menu_slot_rects,
    level_select_menu_start_index, level_select_menu_step_start,
};

const SWIPE_PAGE_THRESHOLD_PX: i32 = 56;

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
    resume_level: usize,
    current_start: usize,
    action: MenuNavAction,
) -> usize {
    match action {
        MenuNavAction::First => 0,
        MenuNavAction::PageUp => level_select_menu_step_start(level_count, current_start, -1),
        MenuNavAction::Current => level_select_menu_start_index(level_count, resume_level),
        MenuNavAction::PageDown => level_select_menu_step_start(level_count, current_start, 1),
        MenuNavAction::Last => level_select_menu_clamp_start(level_count, usize::MAX),
    }
}

pub fn level_select_menu_nav_action_for_swipe(delta_x: i32, delta_y: i32) -> Option<MenuNavAction> {
    let vertical = delta_y.abs();
    if vertical < SWIPE_PAGE_THRESHOLD_PX || vertical <= delta_x.abs() {
        return None;
    }
    Some(if delta_y < 0 {
        MenuNavAction::PageDown
    } else {
        MenuNavAction::PageUp
    })
}

pub fn level_select_menu_nav_action_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
    level_count: usize,
    resume_level: usize,
    current_start: usize,
) -> Option<MenuNavAction> {
    tap_target_at(
        px,
        py,
        width,
        height,
        scrollbar_state(level_count, resume_level, current_start),
    )
    .map(|target| match target {
        ScrollbarTapTarget::First => MenuNavAction::First,
        ScrollbarTapTarget::PageUp => MenuNavAction::PageUp,
        ScrollbarTapTarget::Current => MenuNavAction::Current,
        ScrollbarTapTarget::PageDown => MenuNavAction::PageDown,
        ScrollbarTapTarget::Last => MenuNavAction::Last,
    })
}

fn scrollbar_state(level_count: usize, resume_level: usize, page_start: usize) -> ScrollbarState {
    ScrollbarState {
        level_count,
        visible_count: crate::layout::menu_slots_per_page().min(level_count).max(1),
        page_start,
        return_start: level_select_menu_start_index(level_count, resume_level),
    }
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

#[cfg(test)]
mod tests {
    use super::{MenuNavAction, level_select_menu_nav_action_for_swipe};

    #[test]
    fn upward_swipe_pages_down() {
        assert_eq!(
            level_select_menu_nav_action_for_swipe(8, -72),
            Some(MenuNavAction::PageDown)
        );
    }

    #[test]
    fn downward_swipe_pages_up() {
        assert_eq!(
            level_select_menu_nav_action_for_swipe(-12, 72),
            Some(MenuNavAction::PageUp)
        );
    }

    #[test]
    fn short_or_sideways_swipes_do_not_navigate() {
        assert_eq!(level_select_menu_nav_action_for_swipe(8, 32), None);
        assert_eq!(level_select_menu_nav_action_for_swipe(96, 72), None);
    }
}
