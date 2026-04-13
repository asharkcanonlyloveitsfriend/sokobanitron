use crate::assets::UiIcon;
use crate::layout::{BoardViewport, ScreenRect};
use sokobanitron_gameplay::BoardView;
use sokobanitron_level_editor::PullHintStatus;

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
        selected_box: Option<(u32, u32)>,
    },
    PlayerMoved {
        to_x: u32,
        to_y: u32,
    },
    BoxMoved {
        path: Vec<(u32, u32)>,
    },
    BoxRemoved {
        to_x: u32,
        to_y: u32,
    },
    BoxMoveRejected,
    UndoApplied,
    Restarted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayPresentationUpdate {
    pub scene: GameplayScreenRequest,
    pub cause: GameplayPresentationCause,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayMenuScreenRequest {
    pub primary_action_icon: Option<UiIcon>,
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
    pub move_counts: Vec<EditorCountOverlay>,
    pub pull_destination_hints: Vec<EditorHintOverlay>,
    pub draw_mode_active: bool,
    pub can_zoom_out: bool,
    pub can_zoom_in: bool,
    pub can_undo: bool,
    pub can_restart: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorMenuScreenRequest {
    pub primary_action_icon: UiIcon,
    pub show_save_button: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorCountOverlay {
    pub rect: ScreenRect,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorHintOverlay {
    pub rect: ScreenRect,
    pub state: PullHintStatus,
}
