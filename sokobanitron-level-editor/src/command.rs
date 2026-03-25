#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorMode {
    Draw,
    Manipulate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawTool {
    Floor,
    BoxOnGoal,
    Void,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorCommand {
    SetMode(EditorMode),
    ToggleMode,
    Undo,
    RestartToGoals,
    ClearSelection,
    RecomputeHints,
    AdvanceHintJob {
        steps: usize,
    },
    PaintCell {
        cell_x: i32,
        cell_y: i32,
        tool: DrawTool,
    },
    SelectBox {
        cell_x: i32,
        cell_y: i32,
    },
    MoveSelectedBoxTo {
        cell_x: i32,
        cell_y: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorEffects {
    pub world_changed: bool,
    pub selection_changed: bool,
    pub mode_changed: bool,
    pub history_changed: bool,
    pub hints_changed: bool,
    pub needs_revalidation: bool,
}
