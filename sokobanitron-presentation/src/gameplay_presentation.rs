//! Shared gameplay presentation state.
//!
//! This module owns the currently displayed gameplay scene at the presentation layer. It stores
//! the latest gameplay screen request and delegates drawing to the shared gameplay renderer.

use crate::renderer::Renderer;
use crate::screen_requests::GameplayScreenRequest;

#[derive(Debug, Clone, Default)]
pub struct GameplayPresentationState {
    current_scene: Option<GameplayScreenRequest>,
}

impl GameplayPresentationState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replace_scene(&mut self, scene: GameplayScreenRequest) {
        self.current_scene = Some(scene);
    }

    pub fn current_scene(&self) -> Option<&GameplayScreenRequest> {
        self.current_scene.as_ref()
    }

    pub fn draw(&self, renderer: &mut Renderer, frame: &mut [u8], width: u32, height: u32) {
        let Some(scene) = self.current_scene.as_ref() else {
            return;
        };
        renderer.draw_gameplay_scene(frame, width, height, scene);
    }
}

#[cfg(test)]
mod tests {
    use super::GameplayPresentationState;
    use crate::layout::fit_board_viewport_for_controls;
    use crate::renderer::Renderer;
    use crate::screen_requests::GameplayScreenRequest;
    use sokobanitron_gameplay::{BoardView, TileKind};

    fn gameplay_scene(level_number: usize) -> GameplayScreenRequest {
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
            Some((1, 1)),
            None,
            false,
        );
        GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(64, 64, &board),
            board,
            can_undo: false,
            can_restart: false,
            level_number,
            show_solved_overlay: false,
        }
    }

    #[test]
    fn replace_scene_stores_current_scene() {
        let mut state = GameplayPresentationState::new();
        let first = gameplay_scene(1);
        let second = gameplay_scene(2);

        state.replace_scene(first);
        state.replace_scene(second.clone());

        assert_eq!(state.current_scene(), Some(&second));
    }

    #[test]
    fn draw_renders_current_scene() {
        let mut state = GameplayPresentationState::new();
        state.replace_scene(gameplay_scene(1));
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 64 * 64 * 4];

        state.draw(&mut renderer, &mut frame, 64, 64);

        assert!(frame.iter().any(|pixel| *pixel != 0));
    }

    #[test]
    fn draw_matches_shared_gameplay_renderer_behavior() {
        let scene = gameplay_scene(1);
        let mut state = GameplayPresentationState::new();
        state.replace_scene(scene.clone());
        let mut state_renderer = Renderer::new();
        let mut direct_renderer = Renderer::new();
        let mut state_frame = vec![0; 64 * 64 * 4];
        let mut direct_frame = vec![0; 64 * 64 * 4];

        state.draw(&mut state_renderer, &mut state_frame, 64, 64);
        direct_renderer.draw_gameplay_scene(&mut direct_frame, 64, 64, &scene);

        assert_eq!(state_frame, direct_frame);
    }
}
