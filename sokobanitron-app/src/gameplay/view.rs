//! App-owned gameplay view state and viewport shaping.
//!
//! This module keeps gameplay surface sizing on the app side of the boundary so both hit-testing
//! and rendering can use the same device-agnostic viewport computation.

use crate::shared::SinglePointerGestureState;
use presentation::layout::{BoardViewport, fit_board_viewport_for_controls};
use sokobanitron_gameplay::BoardView;

const DEFAULT_GAMEPLAY_WIDTH: u32 = 670;
const DEFAULT_GAMEPLAY_HEIGHT: u32 = 891;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayUiState {
    pub surface_width: u32,
    pub surface_height: u32,
    pub(crate) interaction: GameplayInteractionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GameplayInteractionState {
    pub(crate) pointer: SinglePointerGestureState,
}

impl Default for GameplayUiState {
    fn default() -> Self {
        Self {
            surface_width: DEFAULT_GAMEPLAY_WIDTH,
            surface_height: DEFAULT_GAMEPLAY_HEIGHT,
            interaction: GameplayInteractionState::default(),
        }
    }
}

pub fn resize_gameplay_surface(gameplay: &mut GameplayUiState, width: u32, height: u32) {
    gameplay.surface_width = width.max(1);
    gameplay.surface_height = height.max(1);
}

pub fn set_gameplay_touch_slop(gameplay: &mut GameplayUiState, tap_slop_px: i32) {
    gameplay.interaction.pointer.set_tap_slop(tap_slop_px);
}

pub fn build_gameplay_viewport(gameplay: &GameplayUiState, board: &BoardView) -> BoardViewport {
    fit_board_viewport_for_controls(gameplay.surface_width, gameplay.surface_height, board)
}
