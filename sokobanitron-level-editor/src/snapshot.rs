use crate::command::EditorMode;
use crate::world::{EditableTile, NonVoidBounds};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSnapshot {
    pub board: EditorBoardSnapshot,
    pub mode: EditorMode,
    pub selected_box: Option<(i32, i32)>,
    pub pull_destination_hints: Vec<PullHintSnapshot>,
    pub move_counts: Vec<BoxMoveCountSnapshot>,
    pub can_undo: bool,
    pub can_restart: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorBoardSnapshot {
    pub bounds: Option<NonVoidBounds>,
    pub cells: Vec<EditorCellSnapshot>,
    pub player: Option<(i32, i32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditorCellSnapshot {
    pub world_x: i32,
    pub world_y: i32,
    pub tile: EditableTile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PullHintSnapshot {
    pub world_x: i32,
    pub world_y: i32,
    pub state: PullHintStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullHintStatus {
    Pending,
    Ready(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxMoveCountSnapshot {
    pub world_x: i32,
    pub world_y: i32,
    pub count: u32,
}
