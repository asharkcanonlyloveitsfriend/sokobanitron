use crate::hit_test::MenuNavAction;
use crate::layout::level_select_scrollbar::{ScrollbarState, ScrollbarTapTarget, tap_target_at};
use crate::layout::{
    level_set_rows_per_page, level_set_select_clamp_start, level_set_select_indices,
    level_set_select_row_rects, level_set_select_start_index, level_set_select_step_start,
};

pub fn level_set_select_start_for_nav(
    level_set_count: usize,
    active_level_set: usize,
    current_start: usize,
    action: MenuNavAction,
) -> usize {
    match action {
        MenuNavAction::First => 0,
        MenuNavAction::PageUp => level_set_select_step_start(level_set_count, current_start, -1),
        MenuNavAction::Current => level_set_select_start_index(level_set_count, active_level_set),
        MenuNavAction::PageDown => level_set_select_step_start(level_set_count, current_start, 1),
        MenuNavAction::Last => level_set_select_clamp_start(level_set_count, usize::MAX),
    }
}

pub fn level_set_select_nav_action_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
    level_set_count: usize,
    active_level_set: usize,
    current_start: usize,
) -> Option<MenuNavAction> {
    tap_target_at(
        px,
        py,
        width,
        height,
        scrollbar_state(level_set_count, active_level_set, current_start),
    )
    .map(|target| match target {
        ScrollbarTapTarget::First => MenuNavAction::First,
        ScrollbarTapTarget::PageUp => MenuNavAction::PageUp,
        ScrollbarTapTarget::Current => MenuNavAction::Current,
        ScrollbarTapTarget::PageDown => MenuNavAction::PageDown,
        ScrollbarTapTarget::Last => MenuNavAction::Last,
    })
}

pub fn level_set_select_target_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
    level_set_count: usize,
    start: usize,
) -> Option<usize> {
    if px < 0.0 || py < 0.0 {
        return None;
    }
    let slot = level_set_select_row_rects(width, height)
        .into_iter()
        .enumerate()
        .find_map(|(index, rect)| rect.contains(px, py).then_some(index))?;
    level_set_select_indices(level_set_count, start)[slot]
}

fn scrollbar_state(
    level_set_count: usize,
    active_level_set: usize,
    page_start: usize,
) -> ScrollbarState {
    ScrollbarState {
        level_count: level_set_count,
        visible_count: level_set_rows_per_page().min(level_set_count).max(1),
        page_start,
        return_start: level_set_select_start_index(level_set_count, active_level_set),
    }
}
