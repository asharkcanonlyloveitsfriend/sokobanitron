use sokobanitron_gameplay::BoardCell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    Restart,
    Undo,
    ZoomGameplayIn {
        zoom_origin_x: u32,
        zoom_origin_y: u32,
    },
    ZoomGameplayOut,
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
    PanZoomedGameplay {
        delta_x: i32,
        delta_y: i32,
    },
    NoOp,
}
