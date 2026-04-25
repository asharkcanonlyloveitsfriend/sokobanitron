use crate::layout::{
    ScreenRect, gameplay_menu_level_set_button_rect, overlay_primary_action_button_rect,
    top_left_level_button_rect, top_menu_toggle_button_visible_rect,
};

use super::RendererTheme;
use super::pixel_ui::draw_centered_text_in_rect;
const UI_MENU_TEXT_SCALE: usize = 4;
const UI_MENU_TEXT_SPACING: usize = 1;
const SLEEP_LABEL_SPACING: usize = 2;

pub fn draw_top_left_level_button(
    frame: &mut [u8],
    width: u32,
    height: u32,
    level_number: usize,
    theme: RendererTheme,
) {
    draw_button(
        frame,
        width,
        height,
        top_left_level_button_rect(),
        &format!("{level_number}"),
        theme,
    );
}

pub fn draw_controls_ui(
    frame: &mut [u8],
    width: u32,
    height: u32,
    menu_open: bool,
    theme: RendererTheme,
) {
    if width == 0 || height == 0 {
        return;
    }

    draw_top_menu_toggle(frame, width, height, menu_open, theme);
}

pub fn draw_top_menu_toggle(
    frame: &mut [u8],
    width: u32,
    height: u32,
    open: bool,
    theme: RendererTheme,
) {
    let glyph = if open { "/\\" } else { "\\/" };
    draw_button_scaled(
        frame,
        width,
        height,
        top_menu_toggle_button_visible_rect(width),
        glyph,
        UI_MENU_TEXT_SCALE,
        theme,
    );
}

pub fn draw_overlay_primary_action_button_label(
    frame: &mut [u8],
    width: u32,
    height: u32,
    label: &str,
    theme: RendererTheme,
) {
    draw_button(
        frame,
        width,
        height,
        overlay_primary_action_button_rect(width, height),
        label,
        theme,
    );
}

pub fn draw_gameplay_menu_level_set_button(
    frame: &mut [u8],
    width: u32,
    height: u32,
    theme: RendererTheme,
) {
    draw_centered_text_in_rect(
        frame,
        width,
        height,
        gameplay_menu_level_set_button_rect(width, height),
        "CHANGE SET",
        UI_MENU_TEXT_SCALE,
        UI_MENU_TEXT_SPACING,
        button_text_color(theme),
    );
}

pub(crate) fn draw_sleep_label(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    theme: RendererTheme,
) {
    draw_centered_text_in_rect(
        frame,
        width,
        height,
        rect,
        "SLEEP",
        UI_MENU_TEXT_SCALE,
        SLEEP_LABEL_SPACING,
        button_text_color(theme),
    );
}

fn draw_button(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    label: &str,
    theme: RendererTheme,
) {
    draw_button_scaled(frame, width, height, rect, label, UI_MENU_TEXT_SCALE, theme);
}

fn draw_button_scaled(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    label: &str,
    scale: usize,
    theme: RendererTheme,
) {
    draw_centered_text_in_rect(
        frame,
        width,
        height,
        rect,
        label,
        scale,
        UI_MENU_TEXT_SPACING,
        button_text_color(theme),
    );
}

fn button_text_color(theme: RendererTheme) -> u8 {
    theme.gray_2
}
