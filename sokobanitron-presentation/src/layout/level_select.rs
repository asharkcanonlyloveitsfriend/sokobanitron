use super::{BOARD_VERTICAL_MARGIN, level_select_scrollbar};

const MENU_SLOTS_PER_PAGE: usize = 4;
const MENU_PAGE_STEP: usize = MENU_SLOTS_PER_PAGE;

pub(crate) const fn menu_slots_per_page() -> usize {
    MENU_SLOTS_PER_PAGE
}

pub fn level_select_menu_start_index(level_count: usize, current_level: usize) -> usize {
    if level_count <= MENU_SLOTS_PER_PAGE || current_level == 0 {
        0
    } else if current_level >= level_count.saturating_sub(1) {
        level_count.saturating_sub(MENU_SLOTS_PER_PAGE)
    } else {
        current_level.saturating_sub(1)
    }
}

pub fn level_select_menu_clamp_start(level_count: usize, start: usize) -> usize {
    start.min(level_select_menu_max_start(level_count))
}

pub fn level_select_menu_step_start(level_count: usize, start: usize, direction: i32) -> usize {
    let start = level_select_menu_clamp_start(level_count, start);
    if direction < 0 {
        start.saturating_sub(MENU_PAGE_STEP)
    } else if direction > 0 {
        start
            .saturating_add(MENU_PAGE_STEP)
            .min(level_select_menu_max_start(level_count))
    } else {
        start
    }
}

pub fn level_select_menu_indices(level_count: usize, start: usize) -> [Option<usize>; 4] {
    let start = level_select_menu_clamp_start(level_count, start);
    let mut out = [None; 4];
    for (slot, idx) in out.iter_mut().zip(start..start + MENU_SLOTS_PER_PAGE) {
        if idx < level_count {
            *slot = Some(idx);
        }
    }
    out
}

pub fn level_select_menu_slot_rects(width: u32, height: u32) -> [(i32, i32, u32, u32); 4] {
    let content_w = menu_content_width(width);
    let top = BOARD_VERTICAL_MARGIN;
    let content_h = height.saturating_sub(top);
    let half_w = content_w / 2;
    let half_h = content_h / 2;
    [
        (0, top as i32, half_w, half_h),
        (
            half_w as i32,
            top as i32,
            content_w.saturating_sub(half_w),
            half_h,
        ),
        (
            0,
            (top + half_h) as i32,
            half_w,
            content_h.saturating_sub(half_h),
        ),
        (
            half_w as i32,
            (top + half_h) as i32,
            content_w.saturating_sub(half_w),
            content_h.saturating_sub(half_h),
        ),
    ]
}

fn level_select_menu_max_start(level_count: usize) -> usize {
    level_count.saturating_sub(MENU_SLOTS_PER_PAGE)
}

fn menu_content_width(width: u32) -> u32 {
    width.saturating_sub(level_select_scrollbar::right_rail_width(width))
}
