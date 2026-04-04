use crate::assets::{UiIcon, draw_ui_icon_in_rect};
use crate::layout::{
    ScreenRect, editor_bottom_left_button_rect, editor_bottom_right_button_rect,
    overlay_secondary_action_button_rect, top_left_level_button_rect,
};
use crate::screen_requests::{EditorMenuScreenRequest, EditorScreenRequest};
use sokobanitron_level_editor::PullHintStatus;

use super::chrome::{draw_overlay_primary_action_button, draw_top_menu_toggle};
use super::pixel_ui::{PIXEL_FONT_HEIGHT, draw_centered_text_in_rect, measure_text_width};
use super::{BoardSceneComposition, Renderer};

const BUTTON_TEXT_COLOR: [u8; 4] = [220, 220, 220, 255];
const HINT_TEXT_COLOR: [u8; 4] = [172, 172, 172, 255];
const UI_TEXT_SCALE: usize = 4;

struct EditorControlsState {
    draw_mode_active: bool,
    can_zoom_out: bool,
    can_zoom_in: bool,
    can_undo: bool,
    can_restart: bool,
}

impl Renderer {
    pub fn draw_editor_screen(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &EditorScreenRequest,
    ) {
        self.draw_board_scene_on_frame(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            BoardSceneComposition::static_scene(),
        );
        self.draw_editor_overlays_on_frame(frame, width, height, request);
        self.draw_editor_chrome_on_frame(frame, width, height, request);
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
        if request.show_save_button {
            draw_centered_text_in_rect(
                frame,
                width,
                height,
                overlay_secondary_action_button_rect(width, height),
                "SAVE",
                UI_TEXT_SCALE,
                1,
                BUTTON_TEXT_COLOR,
            );
        }
    }

    pub fn draw_editor_overlays_on_frame(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &EditorScreenRequest,
    ) {
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
    }

    pub fn draw_editor_chrome_on_frame(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &EditorScreenRequest,
    ) {
        draw_editor_controls(
            frame,
            width,
            height,
            EditorControlsState {
                draw_mode_active: request.draw_mode_active,
                can_zoom_out: request.can_zoom_out,
                can_zoom_in: request.can_zoom_in,
                can_undo: request.can_undo,
                can_restart: request.can_restart,
            },
        );
        draw_top_menu_toggle(frame, width, height, false);
    }
}

fn draw_editor_controls(frame: &mut [u8], width: u32, height: u32, controls: EditorControlsState) {
    let mode_icon = if controls.draw_mode_active {
        UiIcon::Draw
    } else {
        UiIcon::Select
    };
    draw_ui_icon_in_rect(
        frame,
        width,
        height,
        top_left_level_button_rect(),
        mode_icon,
        BUTTON_TEXT_COLOR,
    );

    if controls.draw_mode_active {
        if controls.can_zoom_out {
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
        if controls.can_zoom_in {
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
        if controls.can_undo {
            draw_ui_icon_in_rect(
                frame,
                width,
                height,
                editor_bottom_left_button_rect(height),
                UiIcon::Undo,
                BUTTON_TEXT_COLOR,
            );
        }
        if controls.can_restart {
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

#[cfg(test)]
mod tests {
    use super::Renderer;
    use crate::layout::fit_board_viewport_for_controls;
    use crate::screen_requests::EditorScreenRequest;
    use sokobanitron_gameplay::{BoardView, TileKind};

    fn solved_board() -> BoardView {
        BoardView::new(
            3,
            3,
            vec![
                TileKind::Void,
                TileKind::Floor,
                TileKind::Void,
                TileKind::Floor,
                TileKind::Goal,
                TileKind::Floor,
                TileKind::Void,
                TileKind::Floor,
                TileKind::Void,
            ],
            vec![false, false, false, false, true, false, false, false, false],
            Some((1, 1)),
            None,
            true,
        )
    }

    #[test]
    fn editor_render_does_not_opt_into_solved_visuals() {
        let board = solved_board();
        let request = EditorScreenRequest {
            viewport: fit_board_viewport_for_controls(64, 64, &board),
            board,
            move_counts: Vec::new(),
            pull_destination_hints: Vec::new(),
            draw_mode_active: false,
            can_zoom_out: false,
            can_zoom_in: false,
            can_undo: false,
            can_restart: false,
        };
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 64 * 64 * 4];

        renderer.draw_editor_screen(&mut frame, 64, 64, &request);

        assert!(renderer.solved_box_bitmap_cache.is_empty());
        assert!(renderer.squint_player_bitmap_cache.is_empty());
    }
}
