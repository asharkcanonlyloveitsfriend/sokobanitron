use crate::assets::UiIcon;
use crate::layout::{BoardViewport, ScreenRect};
use sokobanitron_gameplay::BoardView;
use sokobanitron_level_editor::PullHintStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayScreenRequest {
    pub can_undo: bool,
    pub can_restart: bool,
    pub level_number: usize,
    pub show_solved_overlay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayMenuScreenRequest {
    pub primary_action_icon: Option<UiIcon>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelSelectScreenRequest {
    pub page_start: usize,
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
