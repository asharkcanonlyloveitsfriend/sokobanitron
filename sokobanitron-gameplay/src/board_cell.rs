/// Gameplay-facing board coordinate, with `x` as the column and `y` as the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BoardCell {
    pub x: u32,
    pub y: u32,
}

impl BoardCell {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}
