use crate::{
    BOARD_VERTICAL_MARGIN, BoardViewport, BoardViewportOptions, Renderer, UI_BUTTON_MARGIN,
    level_select_scrollbar::{self, ScrollbarTapTarget},
};
use sokobanitron_gameplay::BoardView;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuNavAction {
    First,
    PageUp,
    Current,
    PageDown,
    Last,
}

const MENU_SLOTS_PER_PAGE: usize = 4;
const MENU_PAGE_STEP: usize = MENU_SLOTS_PER_PAGE;

fn level_select_menu_max_start(level_count: usize) -> usize {
    level_count.saturating_sub(MENU_SLOTS_PER_PAGE)
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
        MenuNavAction::Last => level_select_menu_max_start(level_count),
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

fn menu_content_width(width: u32) -> u32 {
    width.saturating_sub(level_select_scrollbar::right_rail_width(width))
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
    let visible_count = MENU_SLOTS_PER_PAGE.min(level_count).max(1);
    level_select_scrollbar::tap_target_at(
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

fn draw_filled_rect(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: [u8; 4],
) {
    if w == 0 || h == 0 {
        return;
    }
    let fw = frame_width as i32;
    let fh = frame_height as i32;
    let x0 = x.clamp(0, fw);
    let y0 = y.clamp(0, fh);
    let x1 = (x + w as i32).clamp(0, fw);
    let y1 = (y + h as i32).clamp(0, fh);
    if x0 >= x1 || y0 >= y1 {
        return;
    }
    for yy in y0..y1 {
        for xx in x0..x1 {
            let idx = ((yy as u32 * frame_width + xx as u32) * 4) as usize;
            frame[idx..idx + 4].copy_from_slice(&color);
        }
    }
}

fn draw_selection_brackets(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
) {
    let len = (w.min(h) / 6).max(8) as i32;
    let thickness = 4;
    let color = [200, 200, 200, 255];
    draw_filled_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y,
        len as u32,
        thickness as u32,
        color,
    );
    draw_filled_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y,
        thickness as u32,
        len as u32,
        color,
    );
    draw_filled_rect(
        frame,
        frame_width,
        frame_height,
        x + w as i32 - len,
        y,
        len as u32,
        thickness as u32,
        color,
    );
    draw_filled_rect(
        frame,
        frame_width,
        frame_height,
        x + w as i32 - 1,
        y,
        thickness as u32,
        len as u32,
        color,
    );
    draw_filled_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y + h as i32 - 1,
        len as u32,
        thickness as u32,
        color,
    );
    draw_filled_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y + h as i32 - len,
        thickness as u32,
        len as u32,
        color,
    );
    draw_filled_rect(
        frame,
        frame_width,
        frame_height,
        x + w as i32 - len,
        y + h as i32 - 1,
        len as u32,
        thickness as u32,
        color,
    );
    draw_filled_rect(
        frame,
        frame_width,
        frame_height,
        x + w as i32 - 1,
        y + h as i32 - len,
        thickness as u32,
        len as u32,
        color,
    );
}

impl Renderer {
    pub fn draw_level_select_menu_contents(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        preview_boards: &[BoardView],
        current_level: usize,
        page_start: usize,
    ) {
        if preview_boards.is_empty() {
            return;
        }
        let current_level = current_level.min(preview_boards.len().saturating_sub(1));
        let slots = level_select_menu_slot_rects(width, height);
        let page_start = level_select_menu_clamp_start(preview_boards.len(), page_start);
        let indices = level_select_menu_indices(preview_boards.len(), page_start);
        for (slot_idx, level_idx) in indices.into_iter().enumerate() {
            let Some(level_idx) = level_idx else {
                continue;
            };
            let Some(board) = preview_boards.get(level_idx) else {
                continue;
            };
            let (sx, sy, sw, sh) = slots[slot_idx];
            let pad = UI_BUTTON_MARGIN.max(8);
            let inner_w = sw.saturating_sub(pad * 2).max(1);
            let inner_h = sh.saturating_sub(pad * 2).max(1);
            let mut viewport = BoardViewport::fit_to_window_with_options(
                inner_w,
                inner_h,
                board,
                BoardViewportOptions::fill_available_space(),
            );
            viewport.origin_x += sx + pad as i32;
            viewport.origin_y += sy + pad as i32;
            self.draw_board_on_frame(frame, width, height, board, &viewport, true, false);
            if level_idx == current_level {
                draw_selection_brackets(frame, width, height, sx, sy, sw, sh);
            }
        }

        level_select_scrollbar::draw(
            frame,
            width,
            height,
            preview_boards.len(),
            MENU_SLOTS_PER_PAGE,
            page_start,
            level_select_menu_start_index(preview_boards.len(), current_level),
        );
    }
}
