//! App-side interaction helper for draw-mode gestures.
//!
//! `PaintMode` is not editor-domain state. It exists only to translate app-owned input
//! policy into `EditorCommand`s consumed by `sokobanitron-level-editor`.

use sokobanitron_level_editor::{DrawTool, EditableTile, EditorCommand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintMode {
    Floor,
    BoxOnGoal,
    Void,
}

impl PaintMode {
    pub fn from_start_tile(tile: EditableTile) -> Self {
        if matches!(tile, EditableTile::Void) {
            PaintMode::Floor
        } else {
            PaintMode::Void
        }
    }

    pub fn to_command(self, cell_x: i32, cell_y: i32) -> EditorCommand {
        let tool = match self {
            PaintMode::Floor => DrawTool::Floor,
            PaintMode::BoxOnGoal => DrawTool::BoxOnGoal,
            PaintMode::Void => DrawTool::Void,
        };
        EditorCommand::PaintCell {
            cell_x,
            cell_y,
            tool,
        }
    }
}
