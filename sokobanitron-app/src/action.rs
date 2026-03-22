#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    Restart,
    Undo,
    ToggleOverlay,
    OpenOverlay,
    CloseOverlay,
    OpenLevelSelect,
    EnterEditorMode,
    EnterGameplayMode,
    SetLevelSelectPageStart(usize),
    SelectLevel(usize),
    AdvanceAfterSolved,
    TapBoardCell { x: u32, y: u32 },
    NoOp,
}
