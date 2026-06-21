#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorMode {
    Draw,
    Move,
    Play,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawTool {
    Floor,
    Void,
    Goal,
    Box,
    GoalWithBox,
    RemoveBox,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorCommand {
    SetMode(EditorMode),
    ToggleMode,
    ClearSelection,
    PaintCell {
        cell_x: i32,
        cell_y: i32,
        tool: DrawTool,
    },
    PositionPlayer {
        cell_x: i32,
        cell_y: i32,
    },
    SelectBox {
        cell_x: i32,
        cell_y: i32,
    },
    MoveSelectedBoxTo {
        cell_x: i32,
        cell_y: i32,
    },
    PlayCell {
        cell_x: i32,
        cell_y: i32,
    },
    PlayDoubleTap {
        cell_x: i32,
        cell_y: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorEffects {
    pub world_changed: bool,
    pub selection_changed: bool,
    pub mode_changed: bool,
    pub hints_changed: bool,
    pub needs_revalidation: bool,
    pub move_rejected: bool,
    pub play_solved: bool,
}
