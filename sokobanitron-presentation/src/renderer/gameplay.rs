//! Gameplay-specific scene composition.
//!
//! This module is the shared presentation entry point for gameplay board rendering. It keeps the
//! gameplay composition order in one place while delegating low-level drawing primitives to the
//! rest of the renderer.

use crate::layout::{ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE};
use crate::screen_requests::{GameplayScreenMode, GameplayScreenRequest};

use super::{BoardSceneComposition, EntityVisualStyle, Renderer, chrome};
use crate::gameplay_animation::GameplayAnimationRunner;

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
    #[allow(dead_code)]
    pub(crate) fn draw_gameplay_scene(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayScreenRequest,
    ) {
        self.draw_gameplay_scene_with_animation(
            frame,
            width,
            height,
            request,
            &GameplayAnimationRunner::default(),
        );
    }

    pub(crate) fn draw_gameplay_scene_with_animation(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayScreenRequest,
        animation_runner: &GameplayAnimationRunner,
    ) {
        let composition = GameplaySceneComposition::from_request(request);
        self.draw_gameplay_board_scene(
            frame,
            width,
            height,
            request,
            composition.board,
            animation_runner,
        );
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
        animation_runner: &GameplayAnimationRunner,
    ) {
        let mut composition = composition;
        if animation_runner.hides_player() {
            composition.player.visible = false;
        }
        self.draw_board_base_layer_on_frame(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            super::BoardBaseLayer::CachedScene,
        );
        self.draw_board_under_entity_layer_on_frame(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            composition.under_entities,
        );
        animation_runner.draw_under_entities(self, frame, width, height, request);
        self.draw_board_entity_layer_on_frame(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            composition.player,
            composition.over_entities,
        );
        self.draw_board_over_entity_layer_on_frame(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            composition.over_entities,
        );
        animation_runner.draw_over_entities(self, frame, width, height, request);
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
