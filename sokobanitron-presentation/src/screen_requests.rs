use crate::layout::BoardViewport;
use sokobanitron_gameplay::{BoardCell, BoardView};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameplayScreenMode {
    #[default]
    Normal,
    Sleep,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayScreenRequest {
    pub board: BoardView,
    pub viewport: BoardViewport,
    pub level_number: usize,
    pub mode: GameplayScreenMode,
}

/// Records the primary reason this gameplay presentation update was produced.
///
/// It is not presentation state and is consumed when the update is received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayPresentationCause {
    /// Render the current gameplay scene without attributing it to a specific gameplay action.
    CurrentState,
    SelectionChanged {
        selected_box: Option<BoardCell>,
    },
    PlayerMoved {
        to: BoardCell,
    },
    BoxMoved {
        path: Vec<BoardCell>,
    },
    BoxRemoved {
        to: BoardCell,
    },
    BoxMoveRejected,
    PuzzleSolved {
        clean: bool,
    },
    UndoApplied,
    Restarted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayPresentationUpdate {
    pub scene: GameplayScreenRequest,
    pub cause: GameplayPresentationCause,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameRequest {
    Gameplay { update: GameplayPresentationUpdate },
    GameplayMenu { screen: GameplayMenuScreenRequest },
    LevelSelect { screen: LevelSelectScreenRequest },
    LevelSetSelect { screen: LevelSetSelectScreenRequest },
    Editor { screen: EditorScreenRequest },
    EditorModeMenu { screen: EditorModeMenuScreenRequest },
    EditorMenu { screen: EditorMenuScreenRequest },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayMenuScreenRequest {
    pub primary_action_label: Option<&'static str>,
    pub show_change_level_set: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelSelectScreenRequest {
    pub page_start: usize,
    pub resume_level: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelSetSelectScreenRequest {
    pub page_start: usize,
    pub active_level_set: Option<usize>,
    pub entries: Vec<LevelSetListEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelSetListEntry {
    pub title: String,
    pub completed_puzzle_count: usize,
    pub total_puzzle_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorScreenRequest {
    pub board: BoardView,
    pub viewport: BoardViewport,
    pub mode_indicator: EditorModeIndicator,
    pub can_zoom_out: bool,
    pub can_zoom_in: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorMenuScreenRequest {
    pub primary_action_label: &'static str,
    pub show_save_button: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorModeMenuScreenRequest {
    pub editor: EditorScreenRequest,
    pub can_enter_play: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorModeIndicator {
    Draw,
    Move,
    Play,
}
