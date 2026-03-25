use crate::assets::{UiIcon, draw_ui_icon_in_rect};
use crate::layout::{
    ScreenRect, editor_bottom_left_button_rect, editor_bottom_right_button_rect,
    top_left_level_button_rect,
};
use crate::screen_requests::{EditorMenuScreenRequest, EditorScreenRequest};
use sokobanitron_level_editor::PullHintStatus;

use super::Renderer;
use super::chrome::{draw_overlay_primary_action_button, draw_top_menu_toggle};
use super::pixel_ui::{PIXEL_FONT_HEIGHT, draw_centered_text_in_rect, measure_text_width};

const BUTTON_TEXT_COLOR: [u8; 4] = [220, 220, 220, 255];
const HINT_TEXT_COLOR: [u8; 4] = [172, 172, 172, 255];
const UI_TEXT_SCALE: usize = 4;

impl Renderer {
    pub fn draw_editor_screen(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &EditorScreenRequest,
    ) {
        self.draw_background_only(frame, width, height);
        self.draw_board_on_frame(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            true,
            false,
        );
        for count in &request.move_counts {
            draw_count_label(
                frame,
                width,
                height,
                count.rect,
                &count.count.to_string(),
                BUTTON_TEXT_COLOR,
            );
        }
        for hint in &request.pull_destination_hints {
            let label = match hint.state {
                PullHintStatus::Pending => "?".to_string(),
                PullHintStatus::Ready(count) => count.min(99).to_string(),
            };
            draw_count_label(frame, width, height, hint.rect, &label, HINT_TEXT_COLOR);
        }
        draw_editor_controls(
            frame,
            width,
            height,
            request.draw_mode_active,
            request.can_zoom_out,
            request.can_zoom_in,
            request.can_undo,
            request.can_restart,
        );
        draw_top_menu_toggle(frame, width, height, false);
    }

    pub fn draw_editor_menu(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &EditorMenuScreenRequest,
    ) {
        self.draw_background_only(frame, width, height);
        draw_top_menu_toggle(frame, width, height, true);
        draw_overlay_primary_action_button(
            frame,
            width,
            height,
            request.primary_action_icon,
            BUTTON_TEXT_COLOR,
        );
    }
}

fn draw_editor_controls(
    frame: &mut [u8],
    width: u32,
    height: u32,
    draw_mode_active: bool,
    can_zoom_out: bool,
    can_zoom_in: bool,
    can_undo: bool,
    can_restart: bool,
) {
    let mode_icon = if draw_mode_active {
        UiIcon::Draw
    } else {
        UiIcon::Manipulate
    };
    draw_ui_icon_in_rect(
        frame,
        width,
        height,
        top_left_level_button_rect(),
        mode_icon,
        BUTTON_TEXT_COLOR,
    );

    if draw_mode_active {
        if can_zoom_out {
            draw_centered_text_in_rect(
                frame,
                width,
                height,
                editor_bottom_left_button_rect(height),
                "-",
                UI_TEXT_SCALE,
                0,
                BUTTON_TEXT_COLOR,
            );
        }
        if can_zoom_in {
            draw_centered_text_in_rect(
                frame,
                width,
                height,
                editor_bottom_right_button_rect(width, height),
                "+",
                UI_TEXT_SCALE,
                0,
                BUTTON_TEXT_COLOR,
            );
        }
    } else {
        if can_undo {
            draw_ui_icon_in_rect(
                frame,
                width,
                height,
                editor_bottom_left_button_rect(height),
                UiIcon::Undo,
                BUTTON_TEXT_COLOR,
            );
        }
        if can_restart {
            draw_ui_icon_in_rect(
                frame,
                width,
                height,
                editor_bottom_right_button_rect(width, height),
                UiIcon::Restart,
                BUTTON_TEXT_COLOR,
            );
        }
    }
}

fn draw_count_label(
    frame: &mut [u8],
    width: u32,
    height: u32,
    rect: ScreenRect,
    text: &str,
    color: [u8; 4],
) {
    let max_text_width = measure_text_width("99", 1, 0).max(1);
    let scale_x = (rect.w as usize / max_text_width).max(1);
    let scale_y = (rect.h as usize / PIXEL_FONT_HEIGHT).max(1);
    let max_fit_scale = scale_x.min(scale_y).max(1);
    let scale = ((max_fit_scale * 5) / 25).max(1);
    draw_centered_text_in_rect(frame, width, height, rect, text, scale, 0, color);
}
