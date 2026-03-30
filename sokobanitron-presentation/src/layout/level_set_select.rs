use super::{BOARD_VERTICAL_MARGIN, ScreenRect, UI_BUTTON_MARGIN, level_select_scrollbar};

const LEVEL_SET_ROWS_PER_PAGE: usize = 20;
const LEVEL_SET_PAGE_STEP: usize = LEVEL_SET_ROWS_PER_PAGE;

pub(crate) const fn level_set_rows_per_page() -> usize {
    LEVEL_SET_ROWS_PER_PAGE
}

pub fn level_set_select_start_index(
    level_set_count: usize,
    active_level_set: Option<usize>,
) -> usize {
    let Some(active_level_set) = active_level_set else {
        return 0;
    };
    if level_set_count <= LEVEL_SET_ROWS_PER_PAGE || active_level_set == 0 {
        0
    } else if active_level_set >= level_set_count.saturating_sub(1) {
        level_set_count.saturating_sub(LEVEL_SET_ROWS_PER_PAGE)
    } else {
        active_level_set.saturating_sub(1)
    }
}

pub fn level_set_select_clamp_start(level_set_count: usize, start: usize) -> usize {
    start.min(level_set_select_max_start(level_set_count))
}

pub fn level_set_select_step_start(level_set_count: usize, start: usize, direction: i32) -> usize {
    let start = level_set_select_clamp_start(level_set_count, start);
    if direction < 0 {
        start.saturating_sub(LEVEL_SET_PAGE_STEP)
    } else if direction > 0 {
        start
            .saturating_add(LEVEL_SET_PAGE_STEP)
            .min(level_set_select_max_start(level_set_count))
    } else {
        start
    }
}

pub fn level_set_select_indices(
    level_set_count: usize,
    start: usize,
) -> [Option<usize>; LEVEL_SET_ROWS_PER_PAGE] {
    let start = level_set_select_clamp_start(level_set_count, start);
    let mut out = [None; LEVEL_SET_ROWS_PER_PAGE];
    for (slot, idx) in out.iter_mut().zip(start..start + LEVEL_SET_ROWS_PER_PAGE) {
        if idx < level_set_count {
            *slot = Some(idx);
        }
    }
    out
}

pub fn level_set_select_row_rects(
    width: u32,
    height: u32,
) -> [ScreenRect; LEVEL_SET_ROWS_PER_PAGE] {
    let top = BOARD_VERTICAL_MARGIN.saturating_add(UI_BUTTON_MARGIN / 2);
    let bottom = height.saturating_sub(UI_BUTTON_MARGIN);
    let content_h = bottom.saturating_sub(top).max(1);
    let row_h = (content_h / LEVEL_SET_ROWS_PER_PAGE as u32).max(1);
    let content_w = width.saturating_sub(level_select_scrollbar::right_rail_width(width));

    std::array::from_fn(|index| {
        let y = top.saturating_add(row_h.saturating_mul(index as u32));
        let h = if index + 1 == LEVEL_SET_ROWS_PER_PAGE {
            bottom.saturating_sub(y)
        } else {
            row_h
        };
        ScreenRect {
            x: 0,
            y,
            w: content_w,
            h,
        }
    })
}

fn level_set_select_max_start(level_set_count: usize) -> usize {
    level_set_count.saturating_sub(LEVEL_SET_ROWS_PER_PAGE)
}
