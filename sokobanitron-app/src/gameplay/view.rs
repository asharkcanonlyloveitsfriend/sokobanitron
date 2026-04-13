//! App-owned gameplay view state and viewport shaping.
//!
//! This module keeps gameplay surface sizing on the app side of the boundary so both hit-testing
//! and rendering can use the same device-agnostic viewport computation.

use crate::persistence::LevelSetCatalogEntry;
use crate::shared::{DoubleTapTracker, SinglePointerGestureState};
use presentation::layout::{BoardViewport, fit_board_viewport_for_controls};
use sokobanitron_gameplay::{BoardCell, BoardView};
use std::time::Duration;

const DEFAULT_GAMEPLAY_WIDTH: u32 = 670;
const DEFAULT_GAMEPLAY_HEIGHT: u32 = 891;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayUiState {
    pub surface_width: u32,
    pub surface_height: u32,
    pub level_sets: Vec<LevelSetCatalogEntry>,
    pub active_level_set: Option<usize>,
    pub(crate) interaction: GameplayInteractionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GameplayInteractionState {
    pub(crate) pointer: SinglePointerGestureState,
    pub(crate) double_tap: DoubleTapTracker<BoardCell>,
    pub(crate) double_tap_window: Duration,
}

impl Default for GameplayInteractionState {
    fn default() -> Self {
        Self {
            pointer: SinglePointerGestureState::default(),
            double_tap: DoubleTapTracker::default(),
            double_tap_window: Duration::from_millis(325),
        }
    }
}

impl Default for GameplayUiState {
    fn default() -> Self {
        Self {
            surface_width: DEFAULT_GAMEPLAY_WIDTH,
            surface_height: DEFAULT_GAMEPLAY_HEIGHT,
            level_sets: Vec::new(),
            active_level_set: None,
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

pub fn set_gameplay_double_tap_window(gameplay: &mut GameplayUiState, window: Duration) {
    gameplay.interaction.double_tap_window = window;
}

pub fn set_gameplay_level_sets(
    gameplay: &mut GameplayUiState,
    level_sets: Vec<LevelSetCatalogEntry>,
    active_level_set: Option<usize>,
) {
    gameplay.active_level_set = active_level_set.filter(|&index| index < level_sets.len());
    gameplay.level_sets = level_sets;
}

pub fn build_gameplay_viewport(gameplay: &GameplayUiState, board: &BoardView) -> BoardViewport {
    fit_board_viewport_for_controls(gameplay.surface_width, gameplay.surface_height, board)
}

#[cfg(test)]
mod tests {
    use super::{GameplayUiState, set_gameplay_level_sets};
    use crate::persistence::{LevelSetCatalogEntry, LevelSetKind};

    fn sample_level_sets() -> Vec<LevelSetCatalogEntry> {
        vec![
            LevelSetCatalogEntry {
                kind: LevelSetKind::Imported,
                title: "Set 1".to_string(),
                completed_puzzle_count: 0,
                total_puzzle_count: 10,
            },
            LevelSetCatalogEntry {
                kind: LevelSetKind::Imported,
                title: "Set 2".to_string(),
                completed_puzzle_count: 2,
                total_puzzle_count: 12,
            },
        ]
    }

    #[test]
    fn set_gameplay_level_sets_keeps_valid_active_index() {
        let mut gameplay = GameplayUiState::default();

        set_gameplay_level_sets(&mut gameplay, sample_level_sets(), Some(1));

        assert_eq!(gameplay.active_level_set, Some(1));
        assert_eq!(gameplay.level_sets.len(), 2);
    }

    #[test]
    fn set_gameplay_level_sets_keeps_none_for_non_empty_catalog() {
        let mut gameplay = GameplayUiState::default();

        set_gameplay_level_sets(&mut gameplay, sample_level_sets(), None);

        assert_eq!(gameplay.active_level_set, None);
        assert_eq!(gameplay.level_sets.len(), 2);
    }

    #[test]
    fn set_gameplay_level_sets_drops_out_of_bounds_active_index() {
        let mut gameplay = GameplayUiState::default();

        set_gameplay_level_sets(&mut gameplay, sample_level_sets(), Some(99));

        assert_eq!(gameplay.active_level_set, None);
        assert_eq!(gameplay.level_sets.len(), 2);
    }
}
