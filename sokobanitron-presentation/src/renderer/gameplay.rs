//! Gameplay-specific scene composition.
//!
//! This module is the shared presentation entry point for gameplay board rendering. It keeps the
//! gameplay composition order in one place while delegating low-level drawing primitives to the
//! rest of the renderer.

use crate::layout::{ScreenRect, UI_BUTTON_MARGIN, UI_BUTTON_SIZE};
use crate::screen_requests::{GameplayScreenMode, GameplayScreenRequest};
use sokobanitron_gameplay::BoardCell;

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
    fn from_request(
        request: &GameplayScreenRequest,
        entity_visual_style: EntityVisualStyle,
    ) -> Self {
        Self {
            board: BoardSceneComposition::gameplay_snapshot(
                entity_visual_style,
                request.sleeping_player || matches!(request.mode, GameplayScreenMode::Sleep),
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
        self.draw_gameplay_scene_with_style_and_animation(
            frame,
            width,
            height,
            request,
            EntityVisualStyle::Standard,
            &GameplayAnimationRunner::default(),
        );
    }

    pub(crate) fn draw_gameplay_scene_with_style_and_animation(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayScreenRequest,
        entity_visual_style: EntityVisualStyle,
        animation_runner: &GameplayAnimationRunner,
    ) {
        let composition = GameplaySceneComposition::from_request(request, entity_visual_style);
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
                chrome::draw_controls_ui(frame, width, height, false, self.theme);
                chrome::draw_top_left_level_button(frame, width, height, level_number, self.theme);
            }
            GameplayChromePhase::Sleep => self.draw_gameplay_sleep_chrome(frame, width, height),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_gameplay_scene_cells(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayScreenRequest,
        cells: &[BoardCell],
        entity_visual_style: EntityVisualStyle,
        animation_runner: &GameplayAnimationRunner,
    ) {
        let composition = GameplaySceneComposition::from_request(request, entity_visual_style);
        assert!(
            matches!(
                composition.chrome,
                GameplayChromePhase::GameplayControls { .. }
            ),
            "cell gameplay redraw requires a normal gameplay scene"
        );
        for &cell in cells {
            self.draw_gameplay_board_cell_scene(
                frame,
                width,
                height,
                request,
                composition.board,
                cell,
                animation_runner,
            );
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
            composition,
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
        animation_runner.draw_under_entities(self, frame, width, height, request, None);
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
        animation_runner.draw_over_entities(self, frame, width, height, request, None);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_gameplay_board_cell_scene(
        &mut self,
        frame: &mut [u8],
        width: u32,
        height: u32,
        request: &GameplayScreenRequest,
        composition: BoardSceneComposition,
        cell: BoardCell,
        animation_runner: &GameplayAnimationRunner,
    ) {
        let mut composition = composition;
        if animation_runner.hides_player() {
            composition.player.visible = false;
        }
        self.draw_floor_tile_cell(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            cell,
            composition.tile_borders,
        );
        animation_runner.draw_under_entities(self, frame, width, height, request, Some(cell));
        self.draw_box_at(
            frame,
            width,
            height,
            &request.board,
            &request.viewport,
            composition.over_entities.entity_visual_style,
            cell,
        );
        if composition.player.visible && request.board.player() == Some(cell) {
            self.draw_player(
                frame,
                width,
                height,
                &request.board,
                &request.viewport,
                composition.over_entities.entity_visual_style,
                composition.player.sleeping,
            );
        }
        animation_runner.draw_over_entities(self, frame, width, height, request, Some(cell));
    }

    fn draw_gameplay_sleep_chrome(&mut self, frame: &mut [u8], width: u32, height: u32) {
        let rect = ScreenRect {
            x: 0,
            y: 0,
            w: width,
            h: UI_BUTTON_MARGIN + UI_BUTTON_SIZE,
        };
        self.restore_background_rect(frame, width, height, rect);
        chrome::draw_sleep_label(frame, width, height, rect, self.theme);
    }
}
