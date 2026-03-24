use crate::constants::{BUTTON_TEXT_COLOR, HINT_TEXT_COLOR, UI_TEXT_SCALE};
pub use renderer::ScreenRect;
use renderer::{
    PIXEL_FONT_HEIGHT, UI_BUTTON_MARGIN, UI_BUTTON_SIZE, UiIcon, draw_centered_text_in_rect,
    draw_ui_icon_in_rect, measure_text_width, top_left_level_button_rect,
};

pub enum ZoomButtonAction {
    ZoomOut,
    ZoomIn,
}

pub enum ManipulateButtonAction {
    Restart,
    Undo,
}

type ModeIcon = UiIcon;

pub fn zoom_button_action_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
    can_zoom_out: bool,
    can_zoom_in: bool,
) -> Option<ZoomButtonAction> {
    if can_zoom_out {
        let minus = zoom_out_button_rect(height);
        if minus.contains(px, py) {
            return Some(ZoomButtonAction::ZoomOut);
        }
    }
    if can_zoom_in {
        let plus = zoom_in_button_rect(width, height);
        if plus.contains(px, py) {
            return Some(ZoomButtonAction::ZoomIn);
        }
    }
    None
}

pub fn manipulate_button_action_at(
    px: f64,
    py: f64,
    width: u32,
    height: u32,
    can_undo: bool,
    can_restart: bool,
) -> Option<ManipulateButtonAction> {
    if can_undo {
        let undo = undo_button_rect(height);
        if undo.contains(px, py) {
            return Some(ManipulateButtonAction::Undo);
        }
    }
    if can_restart {
        let restart = restart_button_rect(width, height);
        if restart.contains(px, py) {
            return Some(ManipulateButtonAction::Restart);
        }
    }
    None
}

pub fn draw_box_move_count(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    count: u32,
) {
    let label = count.min(99).to_string();
    let max_text_width = measure_text_width("99", 1, 0).max(1);
    let scale_x = (rect.w as usize / max_text_width).max(1);
    let scale_y = (rect.h as usize / PIXEL_FONT_HEIGHT).max(1);
    let max_fit_scale = scale_x.min(scale_y).max(1);
    let scale = ((max_fit_scale * 6) / 25).max(1);
    draw_centered_label(frame, width, height, rect, &label, scale, BUTTON_TEXT_COLOR);
}

pub fn draw_move_hint_count(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    count: u32,
) {
    let label = count.min(99).to_string();
    let max_text_width = measure_text_width("99", 1, 0).max(1);
    let scale_x = (rect.w as usize / max_text_width).max(1);
    let scale_y = (rect.h as usize / PIXEL_FONT_HEIGHT).max(1);
    let max_fit_scale = scale_x.min(scale_y).max(1);
    let scale = ((max_fit_scale * 5) / 25).max(1);
    draw_centered_label(frame, width, height, rect, &label, scale, HINT_TEXT_COLOR);
}

pub fn draw_move_hint_pending(frame: &mut [u8], width: u32, height: u32, rect: ScreenRect) {
    let max_text_width = measure_text_width("99", 1, 0).max(1);
    let scale_x = (rect.w as usize / max_text_width).max(1);
    let scale_y = (rect.h as usize / PIXEL_FONT_HEIGHT).max(1);
    let max_fit_scale = scale_x.min(scale_y).max(1);
    let scale = ((max_fit_scale * 5) / 25).max(1);
    draw_centered_label(frame, width, height, rect, "?", scale, HINT_TEXT_COLOR);
}

#[allow(clippy::too_many_arguments)]
pub fn draw_controls(
    frame: &mut [u8],
    width: u32,
    height: u32,
    can_zoom_out: bool,
    can_zoom_in: bool,
    draw_mode_active: bool,
    can_undo: bool,
    can_restart: bool,
) {
    let mode_rect = top_left_level_button_rect();
    let mode_icon = if draw_mode_active {
        ModeIcon::Draw
    } else {
        ModeIcon::Manipulate
    };
    draw_mode_toggle_icon(frame, width, height, mode_rect, mode_icon);

    if draw_mode_active {
        if can_zoom_out {
            let minus = zoom_out_button_rect(height);
            draw_centered_label(
                frame,
                width,
                height,
                minus,
                "-",
                UI_TEXT_SCALE,
                BUTTON_TEXT_COLOR,
            );
        }
        if can_zoom_in {
            let plus = zoom_in_button_rect(width, height);
            draw_centered_label(
                frame,
                width,
                height,
                plus,
                "+",
                UI_TEXT_SCALE,
                BUTTON_TEXT_COLOR,
            );
        }
    } else {
        if can_undo {
            let undo = undo_button_rect(height);
            draw_ui_icon_in_rect(frame, width, height, undo, UiIcon::Undo, BUTTON_TEXT_COLOR);
        }
        if can_restart {
            let restart = restart_button_rect(width, height);
            draw_ui_icon_in_rect(
                frame,
                width,
                height,
                restart,
                UiIcon::Restart,
                BUTTON_TEXT_COLOR,
            );
        }
    }
}

fn zoom_out_button_rect(height: u32) -> ScreenRect {
    ScreenRect {
        x: UI_BUTTON_MARGIN,
        y: height.saturating_sub(UI_BUTTON_MARGIN + UI_BUTTON_SIZE),
        w: UI_BUTTON_SIZE,
        h: UI_BUTTON_SIZE,
    }
}

fn zoom_in_button_rect(width: u32, height: u32) -> ScreenRect {
    ScreenRect {
        x: width.saturating_sub(UI_BUTTON_MARGIN + UI_BUTTON_SIZE),
        y: height.saturating_sub(UI_BUTTON_MARGIN + UI_BUTTON_SIZE),
        w: UI_BUTTON_SIZE,
        h: UI_BUTTON_SIZE,
    }
}

fn restart_button_rect(width: u32, height: u32) -> ScreenRect {
    zoom_in_button_rect(width, height)
}

fn undo_button_rect(height: u32) -> ScreenRect {
    zoom_out_button_rect(height)
}

fn draw_mode_toggle_icon(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    mode_icon: ModeIcon,
) {
    draw_ui_icon_in_rect(frame, width, height, rect, mode_icon, BUTTON_TEXT_COLOR);
}

fn draw_centered_label(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    text: &str,
    scale: usize,
    color: [u8; 4],
) {
    draw_centered_text_in_rect(frame, width, height, rect, text, scale, 0, color);
}
