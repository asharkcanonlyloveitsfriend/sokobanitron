use crate::command::EditorMode;
use crate::world::{NonVoidBounds, Tile};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorSnapshot {
    pub board: EditorBoardSnapshot,
    pub mode: EditorMode,
    pub selected_box: Option<(i32, i32)>,
    pub can_enter_play: bool,
    pub can_save: bool,
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
    pub tile: Tile,
    pub has_box: bool,
}
