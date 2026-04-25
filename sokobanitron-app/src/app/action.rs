use sokobanitron_gameplay::BoardCell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    Restart,
    Undo,
    ToggleOverlay,
    OpenOverlay,
    CloseOverlay,
    OpenLevelSelect,
    OpenLevelSetSelect,
    EnterEditorMode,
    EnterGameplayMode,
    SetLevelSelectPageStart(usize),
    SetLevelSetSelectPageStart(usize),
    SelectLevel(usize),
    SelectLevelSet(usize),
    AdvanceAfterSolved,
    TapBoardCell(BoardCell),
    DoubleTapBoardCell(BoardCell),
    NoOp,
}
