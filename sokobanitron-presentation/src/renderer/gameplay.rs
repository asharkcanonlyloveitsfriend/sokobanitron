//! Gameplay-specific scene composition.
//!
//! This module is the shared presentation entry point for gameplay board rendering. It keeps the
//! gameplay composition order in one place while delegating low-level drawing primitives to the
//! rest of the renderer.

use crate::layout::ControlsUiMode;
use crate::screen_requests::GameplayScreenRequest;

use super::{Renderer, chrome};

impl Renderer {
    pub(crate) fn draw_gameplay_scene(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayScreenRequest,
    ) {
        self.draw_gameplay_board_scene(frame, width, height, request);
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

    fn draw_gameplay_board_scene(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayScreenRequest,
    ) {
        self.draw_background_only(frame, width, height);
        self.draw_board_on_frame(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            true,
            request.show_solved_overlay,
        );
    }
}
