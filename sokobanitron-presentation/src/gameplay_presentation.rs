//! Shared gameplay presentation state.
//!
//! This module owns the currently displayed gameplay scene at the presentation layer. It stores
//! the latest gameplay scene and delegates drawing to the shared gameplay renderer.

use crate::gameplay_animation::{GameplayAnimationRunner, GameplayPresentationConfig};
use crate::renderer::Renderer;
use crate::screen_requests::{GameplayPresentationUpdate, GameplayScreenRequest};
use std::time::Instant;

pub struct GameplayPresentationState {
    config: GameplayPresentationConfig,
    current_scene: Option<GameplayScreenRequest>,
    animation_runner: GameplayAnimationRunner,
}

impl Default for GameplayPresentationState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameplayPresentationState {
    pub fn new() -> Self {
        Self::with_config(GameplayPresentationConfig::default())
    }

    pub fn with_config(config: GameplayPresentationConfig) -> Self {
        Self {
            config,
            current_scene: None,
            animation_runner: GameplayAnimationRunner::default(),
        }
    }

    pub fn replace_update(&mut self, update: GameplayPresentationUpdate) {
        self.replace_update_at(update, Instant::now());
    }

    pub(crate) fn replace_update_at(&mut self, update: GameplayPresentationUpdate, now: Instant) {
        let scene_unchanged = self.current_scene.as_ref() == Some(&update.scene);
        let previous_scene = self.current_scene.as_ref();
        if scene_unchanged {
            self.animation_runner.advance_to(now);
        } else {
            self.animation_runner.clear();
        }
        let animation_enqueued =
            self.animation_runner
                .enqueue_for_update(previous_scene, &update, self.config, now);
        if scene_unchanged && !animation_enqueued {
            return;
        }
        self.current_scene = Some(update.scene);
    }

    pub fn current_scene(&self) -> Option<&GameplayScreenRequest> {
        self.current_scene.as_ref()
    }

    pub fn has_active_animation(&self) -> bool {
        self.animation_runner.has_active_animation()
    }

    pub fn draw(&mut self, renderer: &mut Renderer, frame: &mut [u8], width: u32, height: u32) {
        self.draw_at(renderer, frame, width, height, Instant::now());
    }

    pub(crate) fn draw_at(
        &mut self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        now: Instant,
    ) {
        let Some(scene) = self.current_scene.as_ref() else {
            return;
        };
        self.animation_runner.advance_to(now);
        renderer.draw_gameplay_scene_with_animation(
            frame,
            width,
            height,
            scene,
            &self.animation_runner,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::GameplayPresentationState;
    use crate::layout::fit_board_viewport_for_controls;
    use crate::renderer::Renderer;
    use crate::screen_requests::{
        GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenMode,
        GameplayScreenRequest,
    };
    use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};
    use std::time::{Duration, Instant};

    fn gameplay_scene(level_number: usize) -> GameplayPresentationUpdate {
        gameplay_scene_with_player(level_number, Some(BoardCell::new(1, 1)))
    }

    fn gameplay_scene_with_player(
        level_number: usize,
        player: Option<BoardCell>,
    ) -> GameplayPresentationUpdate {
        let board = BoardView::new(
            3,
            3,
            vec![
                TileKind::Void,
                TileKind::Floor,
                TileKind::Void,
                TileKind::Floor,
                TileKind::Floor,
                TileKind::Floor,
                TileKind::Void,
                TileKind::Goal,
                TileKind::Void,
            ],
            vec![false; 9],
            player,
            None,
            false,
        );
        GameplayPresentationUpdate {
            scene: GameplayScreenRequest {
                viewport: fit_board_viewport_for_controls(64, 64, &board),
                board,
                level_number,
                mode: GameplayScreenMode::Normal,
            },
            cause: GameplayPresentationCause::CurrentState,
        }
    }

    #[test]
    fn replace_update_stores_current_scene() {
        let mut state = GameplayPresentationState::new();
        let first = gameplay_scene(1);
        let second = gameplay_scene(2);

        state.replace_update(first);
        state.replace_update(second.clone());

        assert_eq!(state.current_scene(), Some(&second.scene));
    }

    #[test]
    fn draw_renders_current_scene() {
        let mut state = GameplayPresentationState::new();
        state.replace_update(gameplay_scene(1));
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 64 * 64 * 4];

        state.draw(&mut renderer, &mut frame, 64, 64);

        assert!(frame.iter().any(|pixel| *pixel != 0));
    }

    #[test]
    fn draw_matches_shared_gameplay_renderer_behavior() {
        let update = gameplay_scene(1);
        let mut state = GameplayPresentationState::new();
        state.replace_update(update.clone());
        let mut state_renderer = Renderer::new();
        let mut direct_renderer = Renderer::new();
        let mut state_frame = vec![0; 64 * 64 * 4];
        let mut direct_frame = vec![0; 64 * 64 * 4];

        state.draw(&mut state_renderer, &mut state_frame, 64, 64);
        direct_renderer.draw_gameplay_scene(&mut direct_frame, 64, 64, &update.scene);

        assert_eq!(state_frame, direct_frame);
    }

    #[test]
    fn box_move_rejected_blink_animation_becomes_visible_after_wait() {
        let mut update = gameplay_scene(1);
        update.cause = GameplayPresentationCause::BoxMoveRejected;
        let mut state = GameplayPresentationState::new();
        let mut renderer = Renderer::new();
        let mut waiting_frame = vec![0; 64 * 64 * 4];
        let mut blinking_frame = vec![0; 64 * 64 * 4];
        let start = Instant::now();

        state.replace_update_at(update, start);
        state.draw_at(&mut renderer, &mut waiting_frame, 64, 64, start);
        state.draw_at(
            &mut renderer,
            &mut blinking_frame,
            64,
            64,
            start + Duration::from_millis(400),
        );

        assert_ne!(waiting_frame, blinking_frame);
        assert!(state.has_active_animation());
    }

    #[test]
    fn repeated_animated_update_replays_for_unchanged_scene() {
        let mut update = gameplay_scene(1);
        update.cause = GameplayPresentationCause::BoxMoveRejected;
        let mut state = GameplayPresentationState::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 64 * 64 * 4];
        let start = Instant::now();

        state.replace_update_at(update.clone(), start);
        state.draw_at(
            &mut renderer,
            &mut frame,
            64,
            64,
            start + Duration::from_millis(400),
        );
        state.draw_at(
            &mut renderer,
            &mut frame,
            64,
            64,
            start + Duration::from_millis(700),
        );
        assert!(!state.has_active_animation());

        state.replace_update_at(update, start + Duration::from_millis(800));

        assert!(state.has_active_animation());
    }

    #[test]
    fn scene_change_drops_pending_animation() {
        let mut rejected_update = gameplay_scene(1);
        rejected_update.cause = GameplayPresentationCause::BoxMoveRejected;
        let moved_update = gameplay_scene(2);
        let mut state = GameplayPresentationState::new();
        let start = Instant::now();

        state.replace_update_at(rejected_update, start);
        assert!(state.has_active_animation());
        state.replace_update_at(moved_update.clone(), start + Duration::from_millis(100));

        assert!(!state.has_active_animation());
        assert_eq!(state.current_scene(), Some(&moved_update.scene));
    }
}
