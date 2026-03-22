pub type Coord = (i32, i32);
pub type BoxMovePath = Vec<Coord>;

#[derive(Clone, Debug)]
pub struct ReverseOptimizationInput {
    pub walkable_cells: Vec<Coord>,
    pub box_positions: Vec<Coord>,
    pub player: Option<Coord>,
}
