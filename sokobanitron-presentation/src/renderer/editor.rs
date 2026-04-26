use crate::assets::{UiIcon, draw_ui_icon_in_rect};
use crate::layout::{
    editor_bottom_left_button_rect, editor_bottom_right_button_rect,
    overlay_secondary_action_button_rect, top_left_level_button_rect,
};
use crate::screen_requests::{EditorMenuScreenRequest, EditorModeIndicator, EditorScreenRequest};

use super::chrome::{draw_overlay_primary_action_button_label, draw_top_menu_toggle};
use super::pixel_ui::draw_centered_text_in_rect;
use super::{BoardSceneComposition, Renderer, RendererTheme};

const UI_TEXT_SCALE: usize = 4;

struct EditorControlsState {
    mode_indicator: EditorModeIndicator,
    can_zoom_out: bool,
    can_zoom_in: bool,
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
        draw_top_menu_toggle(frame, width, height, true, self.theme);
        draw_overlay_primary_action_button_label(
            frame,
            width,
            height,
            request.primary_action_label,
            self.theme,
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
                button_text_color(self.theme),
            );
        }
    }

    pub fn draw_editor_overlays_on_frame(
        &mut self,
        _frame: &mut [u8],
        _width: u32,
        _height: u32,
        request: &EditorScreenRequest,
    ) {
        let _ = request;
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
            self.theme,
            EditorControlsState {
                mode_indicator: request.mode_indicator,
                can_zoom_out: request.can_zoom_out,
                can_zoom_in: request.can_zoom_in,
            },
        );
        draw_top_menu_toggle(frame, width, height, false, self.theme);
    }
}

fn draw_editor_controls(
    frame: &mut [u8],
    width: u32,
    height: u32,
    theme: RendererTheme,
    controls: EditorControlsState,
) {
    let mode_icon = match controls.mode_indicator {
        EditorModeIndicator::Draw => UiIcon::Draw,
        EditorModeIndicator::Move => UiIcon::Select,
        EditorModeIndicator::Play => UiIcon::Play,
    };
    draw_ui_icon_in_rect(
        frame,
        width,
        height,
        top_left_level_button_rect(),
        mode_icon,
        button_text_color(theme),
    );

    if matches!(controls.mode_indicator, EditorModeIndicator::Draw) {
        if controls.can_zoom_out {
            draw_centered_text_in_rect(
                frame,
                width,
                height,
                editor_bottom_left_button_rect(height),
                "-",
                UI_TEXT_SCALE,
                1,
                button_text_color(theme),
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
                1,
                button_text_color(theme),
            );
        }
    }
}

fn button_text_color(theme: RendererTheme) -> u8 {
    theme.gray_2
}

#[cfg(test)]
mod tests {
    use super::Renderer;
    use crate::layout::fit_board_viewport_for_controls;
    use crate::screen_requests::{EditorModeIndicator, EditorScreenRequest};
    use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};

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
            Some(BoardCell::new(1, 1)),
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
            mode_indicator: EditorModeIndicator::Move,
            can_zoom_out: false,
            can_zoom_in: false,
        };
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 64 * 64];

        renderer.draw_editor_screen(&mut frame, 64, 64, &request);

        assert!(renderer.solved_box_bitmap_cache.is_empty());
        assert!(renderer.squint_player_bitmap_cache.is_empty());
    }
}
