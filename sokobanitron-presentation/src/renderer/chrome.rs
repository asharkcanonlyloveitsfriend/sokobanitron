use crate::assets::{UiIcon, draw_ui_icon_in_rect};
use crate::layout::{
    ScreenRect, gameplay_menu_level_set_button_rect, overlay_primary_action_button_rect,
    top_left_level_button_rect, top_menu_toggle_button_rect,
};

use super::pixel_ui::draw_centered_text_in_rect;

const BUTTON_TEXT_COLOR: [u8; 4] = [220, 220, 220, 255];
const UI_MENU_TEXT_SCALE: usize = 4;
const UI_MENU_TEXT_SPACING: usize = 1;
const SLEEP_LABEL_SPACING: usize = 2;

pub fn draw_top_left_level_button(frame: &mut [u8], width: u32, height: u32, level_number: usize) {
    draw_button(
        frame,
        width,
        height,
        top_left_level_button_rect(),
        &format!("{level_number}"),
    );
}

pub fn draw_controls_ui(frame: &mut [u8], width: u32, height: u32, menu_open: bool) {
    if width == 0 || height == 0 {
        return;
    }

    draw_top_menu_toggle(frame, width, height, menu_open);
}

pub fn draw_top_menu_toggle(frame: &mut [u8], width: u32, height: u32, open: bool) {
    let glyph = if open { "/\\" } else { "\\/" };
    draw_button_scaled(
        frame,
        width,
        height,
        top_menu_toggle_button_rect(width),
        glyph,
        UI_MENU_TEXT_SCALE,
    );
}

pub fn draw_overlay_primary_action_button(
    frame: &mut [u8],
    width: u32,
    height: u32,
    icon: UiIcon,
    color: [u8; 4],
) {
    let rect = overlay_primary_action_button_rect(width, height);
    draw_ui_icon_in_rect(frame, width, height, rect, icon, color);
}

pub fn draw_gameplay_menu_level_set_button(frame: &mut [u8], width: u32, height: u32) {
    draw_centered_text_in_rect(
        frame,
        width,
        height,
        gameplay_menu_level_set_button_rect(width, height),
        "CHANGE SET",
        UI_MENU_TEXT_SCALE,
        UI_MENU_TEXT_SPACING,
        BUTTON_TEXT_COLOR,
    );
}

pub(crate) fn draw_sleep_label(frame: &mut [u8], width: u32, height: u32, rect: ScreenRect) {
    draw_centered_text_in_rect(
        frame,
        width,
        height,
        rect,
        "SLEEP",
        UI_MENU_TEXT_SCALE,
        SLEEP_LABEL_SPACING,
        BUTTON_TEXT_COLOR,
    );
}

fn draw_button(frame: &mut [u8], width: u32, height: u32, rect: ScreenRect, label: &str) {
    draw_button_scaled(frame, width, height, rect, label, UI_MENU_TEXT_SCALE);
}

fn draw_button_scaled(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    label: &str,
    scale: usize,
) {
    draw_centered_text_in_rect(
        frame,
        width,
        height,
        rect,
        label,
        scale,
        UI_MENU_TEXT_SPACING,
        BUTTON_TEXT_COLOR,
    );
}
