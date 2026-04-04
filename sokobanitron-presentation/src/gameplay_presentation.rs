//! Shared gameplay presentation state.
//!
//! This module owns the currently displayed gameplay scene at the presentation layer. It stores
//! the latest gameplay presentation update and delegates drawing to the shared gameplay renderer.

use crate::renderer::Renderer;
use crate::screen_requests::{GameplayPresentationUpdate, GameplayScreenRequest};

#[derive(Debug, Clone, Default)]
pub struct GameplayPresentationState {
    current_update: Option<GameplayPresentationUpdate>,
}

impl GameplayPresentationState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replace_update(&mut self, update: GameplayPresentationUpdate) {
        self.current_update = Some(update);
    }

    pub fn current_update(&self) -> Option<&GameplayPresentationUpdate> {
        self.current_update.as_ref()
    }

    pub fn current_scene(&self) -> Option<&GameplayScreenRequest> {
        self.current_update.as_ref().map(|update| &update.scene)
    }

    pub fn draw(&self, renderer: &mut Renderer, frame: &mut [u8], width: u32, height: u32) {
        let Some(update) = self.current_update.as_ref() else {
            return;
        };
        renderer.draw_gameplay_scene(frame, width, height, &update.scene);
    }
}

#[cfg(test)]
mod tests {
    use super::GameplayPresentationState;
    use crate::layout::fit_board_viewport_for_controls;
    use crate::renderer::Renderer;
    use crate::screen_requests::{
        GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenMode,
        GameplayScreenRequest, SolvedStateChange,
    };
    use sokobanitron_gameplay::{BoardView, TileKind};

    fn gameplay_scene(level_number: usize) -> GameplayPresentationUpdate {
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
        GameplayPresentationUpdate {
            scene: GameplayScreenRequest {
                viewport: fit_board_viewport_for_controls(64, 64, &board),
                board,
                level_number,
                mode: GameplayScreenMode::Normal,
            },
            cause: GameplayPresentationCause::CurrentState,
            solved_state_change: SolvedStateChange::Unchanged,
        }
    }

    #[test]
    fn replace_update_stores_current_update() {
        let mut state = GameplayPresentationState::new();
        let first = gameplay_scene(1);
        let second = gameplay_scene(2);

        state.replace_update(first);
        state.replace_update(second.clone());

        assert_eq!(state.current_update(), Some(&second));
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
}
