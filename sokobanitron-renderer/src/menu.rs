use crate::{
    BOARD_VERTICAL_MARGIN, BoardViewport, BoardViewportOptions, Renderer, UI_BUTTON_MARGIN,
    UI_BUTTON_SIZE,
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

fn menu_right_rail_width(width: u32) -> u32 {
    (UI_BUTTON_SIZE + UI_BUTTON_MARGIN)
        .min(width / 3)
        .max(UI_BUTTON_SIZE / 2)
}

fn menu_content_width(width: u32) -> u32 {
    width.saturating_sub(menu_right_rail_width(width))
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

pub fn level_select_menu_nav_button_rects(width: u32, height: u32) -> [(i32, i32, u32, u32); 5] {
    let rail_w = menu_right_rail_width(width);
    let rail_x = width.saturating_sub(rail_w) as i32;
    let btn_w = UI_BUTTON_SIZE
        .min(rail_w.saturating_sub(UI_BUTTON_MARGIN))
        .max(24);
    let btn_h = btn_w;
    let content_top = BOARD_VERTICAL_MARGIN.saturating_add(UI_BUTTON_MARGIN);
    let content_bottom = height.saturating_sub(UI_BUTTON_MARGIN);
    let available_h = content_bottom.saturating_sub(content_top);
    let total_btn_h = btn_h.saturating_mul(5);
    let gap = if available_h > total_btn_h {
        (available_h - total_btn_h) / 4
    } else {
        0
    };
    let offset_x = (rail_w.saturating_sub(btn_w) / 2) as i32;
    let mut out = [(0, 0, 0, 0); 5];
    for (i, slot) in out.iter_mut().enumerate() {
        let y = content_top.saturating_add((i as u32).saturating_mul(btn_h.saturating_add(gap)));
        *slot = (rail_x + offset_x, y as i32, btn_w, btn_h);
    }
    out
}

pub fn level_select_menu_nav_action_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
) -> Option<MenuNavAction> {
    if px < 0.0 || py < 0.0 {
        return None;
    }
    let x = px as i32;
    let y = py as i32;
    let actions = [
        MenuNavAction::First,
        MenuNavAction::PageUp,
        MenuNavAction::Current,
        MenuNavAction::PageDown,
        MenuNavAction::Last,
    ];
    level_select_menu_nav_button_rects(width, height)
        .into_iter()
        .zip(actions)
        .find_map(|((rx, ry, rw, rh), action)| {
            (x >= rx && y >= ry && x < rx + rw as i32 && y < ry + rh as i32).then_some(action)
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

const MENU_ICON_SCALE: i32 = 4;
const MENU_ICON_COLS: usize = 7;
const MENU_ICON_ROWS: usize = 10;
type MenuIcon = [[bool; MENU_ICON_COLS]; MENU_ICON_ROWS];

fn set_icon_pixel(icon: &mut MenuIcon, x: i32, y: i32) {
    if x >= 0 && y >= 0 && (x as usize) < MENU_ICON_COLS && (y as usize) < MENU_ICON_ROWS {
        icon[y as usize][x as usize] = true;
    }
}

fn draw_icon_points(icon: &mut MenuIcon, points: &[(i32, i32)], dx: i32, dy: i32) {
    for (x, y) in points {
        set_icon_pixel(icon, x + dx, y + dy);
    }
}

// These points are the exact `/\` menu-caret pixels rotated 90 degrees.
const RIGHT_CARET_POINTS: [(i32, i32); 10] = [
    (2, 0),
    (3, 1),
    (4, 2),
    (5, 3),
    (6, 4),
    (6, 5),
    (5, 6),
    (4, 7),
    (3, 8),
    (2, 9),
];

const LEFT_CARET_POINTS: [(i32, i32); 10] = [
    (4, 0),
    (3, 1),
    (2, 2),
    (1, 3),
    (0, 4),
    (0, 5),
    (1, 6),
    (2, 7),
    (3, 8),
    (4, 9),
];

fn build_menu_icon(action: MenuNavAction) -> MenuIcon {
    let mut icon = [[false; MENU_ICON_COLS]; MENU_ICON_ROWS];

    match action {
        MenuNavAction::PageUp => {
            // Centered "<", same pixel style as the top caret rotated 90 degrees.
            draw_icon_points(&mut icon, &LEFT_CARET_POINTS, 1, 0);
        }
        MenuNavAction::PageDown => {
            // Centered ">", same pixel style as the top caret rotated 90 degrees.
            draw_icon_points(&mut icon, &RIGHT_CARET_POINTS, -1, 0);
        }
        MenuNavAction::First => {
            draw_icon_points(&mut icon, &LEFT_CARET_POINTS, 1, 0);
            for y in 0..MENU_ICON_ROWS as i32 {
                set_icon_pixel(&mut icon, 0, y);
            }
        }
        MenuNavAction::Last => {
            draw_icon_points(&mut icon, &RIGHT_CARET_POINTS, -1, 0);
            for y in 0..MENU_ICON_ROWS as i32 {
                set_icon_pixel(&mut icon, 6, y);
            }
        }
        MenuNavAction::Current => {
            // Closed triangle: exact ">" caret plus a straight closing bar.
            draw_icon_points(&mut icon, &RIGHT_CARET_POINTS, -1, 0);
            for y in 0..MENU_ICON_ROWS as i32 {
                set_icon_pixel(&mut icon, 0, y);
            }
        }
    }

    icon
}

fn draw_menu_nav_icon(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    rect: (i32, i32, u32, u32),
    action: MenuNavAction,
) {
    let (x, y, w, h) = rect;
    let color = [220, 220, 220, 255];
    let icon = build_menu_icon(action);
    let icon_w = MENU_ICON_COLS as i32 * MENU_ICON_SCALE;
    let icon_h = MENU_ICON_ROWS as i32 * MENU_ICON_SCALE;
    let origin_x = x + ((w as i32 - icon_w) / 2);
    let origin_y = y + ((h as i32 - icon_h) / 2);

    for (row, row_pixels) in icon.iter().enumerate() {
        for (col, on) in row_pixels.iter().enumerate() {
            if *on {
                draw_filled_rect(
                    frame,
                    frame_width,
                    frame_height,
                    origin_x + col as i32 * MENU_ICON_SCALE,
                    origin_y + row as i32 * MENU_ICON_SCALE,
                    MENU_ICON_SCALE as u32,
                    MENU_ICON_SCALE as u32,
                    color,
                );
            }
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

        let actions = [
            MenuNavAction::First,
            MenuNavAction::PageUp,
            MenuNavAction::Current,
            MenuNavAction::PageDown,
            MenuNavAction::Last,
        ];
        for (rect, action) in level_select_menu_nav_button_rects(width, height)
            .into_iter()
            .zip(actions)
        {
            draw_menu_nav_icon(frame, width, height, rect, action);
        }
    }
}
