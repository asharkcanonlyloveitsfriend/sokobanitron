//! Gameplay-specific scene composition.
//!
//! This module is the shared presentation entry point for gameplay board rendering. It keeps the
//! gameplay composition order in one place while delegating low-level drawing primitives to the
//! rest of the renderer.

use crate::layout::{ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE};
use crate::screen_requests::{GameplayScreenMode, GameplayScreenRequest};

use super::{BoardSceneComposition, EntityVisualStyle, Renderer, chrome};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GameplaySceneComposition {
    board: BoardSceneComposition,
    chrome: GameplayChromePhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GameplayChromePhase {
    GameplayControls { level_number: usize },
    Sleep,
}

impl GameplaySceneComposition {
    fn from_request(request: &GameplayScreenRequest) -> Self {
        let entity_visual_style = if request.board.is_solved() {
            EntityVisualStyle::Solved
        } else {
            EntityVisualStyle::Standard
        };
        Self {
            board: BoardSceneComposition::gameplay_snapshot(
                entity_visual_style,
                matches!(request.mode, GameplayScreenMode::Sleep),
            ),
            chrome: match request.mode {
                GameplayScreenMode::Normal => GameplayChromePhase::GameplayControls {
                    level_number: request.level_number,
                },
                GameplayScreenMode::Sleep => GameplayChromePhase::Sleep,
            },
        }
    }
}

impl Renderer {
    pub(crate) fn draw_gameplay_scene(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayScreenRequest,
    ) {
        let composition = GameplaySceneComposition::from_request(request);
        self.draw_gameplay_board_scene(frame, width, height, request, composition.board);
        match composition.chrome {
            GameplayChromePhase::GameplayControls { level_number } => {
                chrome::draw_controls_ui(frame, width, height, false);
                chrome::draw_top_left_level_button(frame, width, height, level_number);
            }
            GameplayChromePhase::Sleep => self.draw_gameplay_sleep_chrome(frame, width, height),
        }
    }

    fn draw_gameplay_board_scene(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayScreenRequest,
        composition: BoardSceneComposition,
    ) {
        self.draw_board_scene_on_frame(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            composition,
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
