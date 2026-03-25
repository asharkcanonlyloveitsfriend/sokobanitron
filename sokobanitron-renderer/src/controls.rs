use crate::icons::{UiIcon, draw_ui_icon_in_rect};
use crate::pixel_ui::draw_centered_text_in_rect;

#[derive(Clone, Copy)]
struct Rect {
    x: usize,
    y: usize,
    w: usize,
    h: usize,
}

impl Rect {
    fn contains(self, px: usize, py: usize) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlsButtonAction {
    ShowMenu,
    Restart,
    Undo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlsUiMode {
    Gameplay,
    MenuOpen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScreenRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl ScreenRect {
    pub fn contains(self, px: f64, py: f64) -> bool {
        if px < 0.0 || py < 0.0 {
            return false;
        }
        let x = px as usize;
        let y = py as usize;
        x >= self.x as usize
            && x < self.x.saturating_add(self.w) as usize
            && y >= self.y as usize
            && y < self.y.saturating_add(self.h) as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlsButtonRects {
    pub menu: ScreenRect,
    pub restart: ScreenRect,
    pub undo: ScreenRect,
}

pub const UI_BUTTON_SIZE: u32 = 76;
pub const UI_BUTTON_MARGIN: u32 = 16;
pub const UI_MENU_BUTTON_HEIGHT: u32 = UI_BUTTON_SIZE / 2;
pub const BOARD_HORIZONTAL_MARGIN: u32 = UI_BUTTON_MARGIN;
pub const BOARD_VERTICAL_MARGIN: u32 = UI_BUTTON_MARGIN + UI_MENU_BUTTON_HEIGHT;

const UI_MENU_TEXT_SCALE: usize = 4;
const UI_TEXT_SCALE: usize = UI_MENU_TEXT_SCALE;

pub fn board_viewport_margins() -> (u32, u32) {
    (BOARD_HORIZONTAL_MARGIN, BOARD_VERTICAL_MARGIN)
}

pub fn controls_button_rects(width: u32, height: u32) -> ControlsButtonRects {
    let width = width as usize;
    let height = height as usize;
    ControlsButtonRects {
        menu: to_screen_rect(top_menu_button_rect(width)),
        undo: to_screen_rect(bottom_left_button_rect(height)),
        restart: to_screen_rect(bottom_right_button_rect(width, height)),
    }
}

pub fn top_menu_toggle_button_rect(width: u32) -> ScreenRect {
    to_screen_rect(top_menu_button_rect(width as usize))
}

pub fn top_menu_toggle_button_hit_rect(width: u32) -> ScreenRect {
    to_screen_rect(top_menu_button_hit_rect(width as usize))
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
    if px < 0.0 || py < 0.0 {
        return None;
    }

    let width = width as usize;
    let height = height as usize;
    let px = px as usize;
    let py = py as usize;

    if top_menu_button_hit_rect(width).contains(px, py) {
        return Some(ControlsButtonAction::ShowMenu);
    }
    if can_undo && bottom_left_button_rect(height).contains(px, py) {
        return Some(ControlsButtonAction::Undo);
    }
    if can_restart && bottom_right_button_rect(width, height).contains(px, py) {
        return Some(ControlsButtonAction::Restart);
    }
    None
}

pub fn top_left_level_button_rect() -> ScreenRect {
    to_screen_rect(top_left_level_button_rect_inner())
}

pub fn bottom_left_corner_button_rect(height: u32) -> ScreenRect {
    to_screen_rect(bottom_left_button_rect(height as usize))
}

pub fn bottom_right_corner_button_rect(width: u32, height: u32) -> ScreenRect {
    to_screen_rect(bottom_right_button_rect(width as usize, height as usize))
}

pub fn draw_top_left_level_button(frame: &mut [u8], width: u32, height: u32, level_number: usize) {
    draw_button(
        frame,
        width as usize,
        height as usize,
        top_left_level_button_rect_inner(),
        &format!("{level_number}"),
    );
}

pub fn draw_controls_ui(
    frame: &mut [u8],
    width: u32,
    height: u32,
    mode: ControlsUiMode,
    can_undo: bool,
    can_restart: bool,
) {
    let width = width as usize;
    let height = height as usize;
    if width == 0 || height == 0 {
        return;
    }

    draw_top_menu_toggle(
        frame,
        width as u32,
        height as u32,
        matches!(mode, ControlsUiMode::MenuOpen),
    );
    if matches!(mode, ControlsUiMode::Gameplay) {
        if can_undo {
            draw_ui_icon_in_rect(
                frame,
                width as u32,
                height as u32,
                to_screen_rect(bottom_left_button_rect(height)),
                UiIcon::Undo,
                [220, 220, 220, 255],
            );
        }
        if can_restart {
            draw_ui_icon_in_rect(
                frame,
                width as u32,
                height as u32,
                to_screen_rect(bottom_right_button_rect(width, height)),
                UiIcon::Restart,
                [220, 220, 220, 255],
            );
        }
    }
}

pub fn draw_top_menu_toggle(frame: &mut [u8], width: u32, height: u32, open: bool) {
    let glyph = if open { "/\\" } else { "\\/" };
    draw_button_scaled(
        frame,
        width as usize,
        height as usize,
        top_menu_button_rect(width as usize),
        glyph,
        UI_MENU_TEXT_SCALE,
    );
}

fn ui_button_size() -> usize {
    UI_BUTTON_SIZE as usize
}

fn ui_button_margin() -> usize {
    UI_BUTTON_MARGIN as usize
}

fn ui_menu_button_height() -> usize {
    UI_MENU_BUTTON_HEIGHT as usize
}

fn to_screen_rect(rect: Rect) -> ScreenRect {
    ScreenRect {
        x: rect.x as u32,
        y: rect.y as u32,
        w: rect.w as u32,
        h: rect.h as u32,
    }
}

fn top_row_y() -> usize {
    ui_button_margin()
}

fn top_row_center_x(width: usize) -> usize {
    width.saturating_sub(ui_button_size()) / 2
}

fn top_menu_button_rect(width: usize) -> Rect {
    Rect {
        x: top_row_center_x(width),
        y: top_row_y(),
        w: ui_button_size(),
        h: ui_menu_button_height(),
    }
}

fn top_left_level_button_rect_inner() -> Rect {
    Rect {
        x: ui_button_margin(),
        y: ui_button_margin(),
        w: ui_button_size(),
        h: ui_button_size(),
    }
}

fn top_menu_button_hit_rect(width: usize) -> Rect {
    let base = top_menu_button_rect(width);
    let extra_w = base.w / 2;
    let extra_h = base.h;
    let hit_x = base.x.saturating_sub(extra_w / 2);
    let hit_y = base.y.saturating_sub(extra_h / 2);
    let hit_w = base.w + extra_w;
    let hit_h = base.h + extra_h;
    Rect {
        x: hit_x,
        y: hit_y,
        w: hit_w,
        h: hit_h,
    }
}

fn bottom_left_button_rect(height: usize) -> Rect {
    Rect {
        x: ui_button_margin(),
        y: height.saturating_sub(ui_button_margin() + ui_button_size()),
        w: ui_button_size(),
        h: ui_button_size(),
    }
}

fn bottom_right_button_rect(width: usize, height: usize) -> Rect {
    Rect {
        x: width.saturating_sub(ui_button_margin() + ui_button_size()),
        y: height.saturating_sub(ui_button_margin() + ui_button_size()),
        w: ui_button_size(),
        h: ui_button_size(),
    }
}

fn draw_button(frame: &mut [u8], width: usize, height: usize, rect: Rect, label: &str) {
    draw_button_scaled(frame, width, height, rect, label, UI_TEXT_SCALE);
}

fn draw_button_scaled(
    frame: &mut [u8],
    width: usize,
    height: usize,
    rect: Rect,
    label: &str,
    scale: usize,
) {
    draw_centered_text_in_rect(
        frame,
        width as u32,
        height as u32,
        to_screen_rect(rect),
        label,
        scale,
        0,
        [220, 220, 220, 255],
    );
}
