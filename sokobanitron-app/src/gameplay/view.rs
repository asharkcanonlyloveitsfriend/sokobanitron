//! App-owned gameplay view state and viewport shaping.
//!
//! This module keeps gameplay surface sizing on the app side of the boundary so both hit-testing
//! and rendering can use the same device-agnostic viewport computation.

use crate::persistence::LevelSetCatalogEntry;
use crate::shared::{DoubleTapTracker, TouchPointerState};
use presentation::layout::{BoardViewport, fit_board_viewport_for_controls_capped};
use sokobanitron_gameplay::{BoardCell, BoardView};
use std::time::Duration;

const DEFAULT_GAMEPLAY_WIDTH: u32 = 670;
const DEFAULT_GAMEPLAY_HEIGHT: u32 = 891;
const DEFAULT_GAMEPLAY_MAX_CELL_SIZE: u32 = 260;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayUiState {
    pub surface_width: u32,
    pub surface_height: u32,
    pub max_cell_size: u32,
    pub level_sets: Vec<LevelSetCatalogEntry>,
    pub active_level_set: Option<usize>,
    pub(crate) interaction: GameplayInteractionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GameplayInteractionState {
    pub(crate) touch: TouchPointerState,
    pub(crate) double_tap: DoubleTapTracker<GameplayDoubleTapTarget>,
    pub(crate) double_tap_window: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameplayDoubleTapTarget {
    BoardCell(BoardCell),
    Player(BoardCell),
}

impl Default for GameplayInteractionState {
    fn default() -> Self {
        Self {
            touch: TouchPointerState::default(),
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
            max_cell_size: DEFAULT_GAMEPLAY_MAX_CELL_SIZE,
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
    gameplay.interaction.touch.set_tap_slop(tap_slop_px);
}

pub fn set_gameplay_double_tap_window(gameplay: &mut GameplayUiState, window: Duration) {
    gameplay.interaction.double_tap_window = window;
}

pub fn set_gameplay_max_cell_size(gameplay: &mut GameplayUiState, max_cell_size: u32) {
    gameplay.max_cell_size = max_cell_size.max(1);
}

pub fn set_gameplay_level_sets(
    gameplay: &mut GameplayUiState,
    level_sets: Vec<LevelSetCatalogEntry>,
    active_level_set: Option<usize>,
) {
    gameplay.active_level_set = active_level_set.filter(|&index| index < level_sets.len());
    gameplay.level_sets = level_sets;
}

pub fn build_gameplay_board_viewport(
    gameplay: &GameplayUiState,
    board: &BoardView,
) -> BoardViewport {
    fit_board_viewport_for_controls_capped(
        gameplay.surface_width,
        gameplay.surface_height,
        board,
        gameplay.max_cell_size,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_GAMEPLAY_MAX_CELL_SIZE, GameplayUiState, build_gameplay_board_viewport,
        resize_gameplay_surface, set_gameplay_level_sets, set_gameplay_max_cell_size,
    };
    use crate::persistence::{LevelSetCatalogEntry, LevelSetKind};
    use sokobanitron_gameplay::{BoardView, TileKind};

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

    fn board_with_tile(width: u32, height: u32, tile: TileKind) -> BoardView {
        let len = (width * height) as usize;
        BoardView::new(
            width,
            height,
            vec![tile; len],
            vec![false; len],
            None,
            None,
            false,
        )
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

    #[test]
    fn gameplay_defaults_to_standard_max_cell_size() {
        let gameplay = GameplayUiState::default();

        assert_eq!(gameplay.max_cell_size, DEFAULT_GAMEPLAY_MAX_CELL_SIZE);
    }

    #[test]
    fn resize_gameplay_surface_clamps_to_nonzero_size() {
        let mut gameplay = GameplayUiState::default();

        resize_gameplay_surface(&mut gameplay, 0, 0);

        assert_eq!(gameplay.surface_width, 1);
        assert_eq!(gameplay.surface_height, 1);
    }

    #[test]
    fn gameplay_viewport_respects_configured_max_cell_size() {
        let mut gameplay = GameplayUiState::default();
        let board = board_with_tile(1, 1, TileKind::Floor);
        set_gameplay_max_cell_size(&mut gameplay, 12);

        let viewport = build_gameplay_board_viewport(&gameplay, &board);

        assert_eq!(viewport.cell_size, 12);
    }

    #[test]
    fn gameplay_board_viewport_uses_full_board_dimensions() {
        let gameplay = GameplayUiState {
            surface_width: 320,
            surface_height: 480,
            ..Default::default()
        };
        let board = board_with_tile(5, 4, TileKind::Floor);

        let viewport = build_gameplay_board_viewport(&gameplay, &board);

        assert_eq!(
            viewport.board_pixel_width,
            (board.width() + viewport.outer_margin_tiles * 2) * viewport.cell_size
        );
        assert_eq!(
            viewport.board_pixel_height,
            (board.height() + viewport.outer_margin_tiles * 2) * viewport.cell_size
        );
    }
}
