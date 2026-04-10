//! Gameplay-specific scene composition.
//!
//! This module is the shared presentation entry point for gameplay board rendering. It keeps the
//! gameplay composition order in one place while delegating low-level drawing primitives to the
//! rest of the renderer.

use crate::layout::{ControlsUiMode, ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE};
use crate::screen_requests::{GameplayScreenMode, GameplayScreenRequest};

use super::{EntityVisualStyle, Renderer, chrome};

impl Renderer {
    pub(crate) fn draw_gameplay_scene(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayScreenRequest,
    ) {
        self.draw_gameplay_board_scene(frame, width, height, request);
        match request.mode {
            GameplayScreenMode::Normal => {
                chrome::draw_controls_ui(
                    frame,
                    width,
                    height,
                    ControlsUiMode::Gameplay,
                    request.can_undo,
                    request.can_restart,
                );
                chrome::draw_top_left_level_button(frame, width, height, request.level_number);
            }
            GameplayScreenMode::Sleep => self.draw_gameplay_sleep_chrome(frame, width, height),
        }
    }

    fn draw_gameplay_board_scene(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayScreenRequest,
    ) {
        let entity_visual_style = if request.board.is_solved() {
            EntityVisualStyle::Solved
        } else {
            EntityVisualStyle::Standard
        };
        self.draw_board_scene_on_frame(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            true,
            entity_visual_style,
            matches!(request.mode, GameplayScreenMode::Sleep),
        );
    }

    fn draw_gameplay_sleep_chrome(&mut self, frame: &mut [u8], width: u32, height: u32) {
        let rect = ScreenRect {
            x: 0,
            y: 0,
            w: width,
            h: UI_BUTTON_MARGIN + UI_BUTTON_SIZE,
        };
        self.restore_background_rect(frame, width, height, rect);
        chrome::draw_sleep_label(frame, width, height, rect);
    }
}
