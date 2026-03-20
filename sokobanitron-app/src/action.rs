#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    Restart,
    Undo,
    ToggleMenu,
    OpenMenu,
    CloseMenu,
    SetMenuPageStart(usize),
    SelectLevel(usize),
    AdvanceAfterSolved,
    TapBoardCell { x: u32, y: u32 },
    NoOp,
}
