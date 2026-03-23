use crate::level::{LevelCell, ParsedLevel};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKind {
    Void,
    Floor,
    Goal,
}

#[derive(Debug, Clone)]
pub struct BoardView {
    width: u32,
    height: u32,
    tiles: Vec<TileKind>,
    boxes: Vec<bool>,
    player: Option<(u32, u32)>,
    selected_box: Option<(u32, u32)>,
    is_solved: bool,
}

impl BoardView {
    pub fn new(
        width: u32,
        height: u32,
        tiles: Vec<TileKind>,
        boxes: Vec<bool>,
        player: Option<(u32, u32)>,
        selected_box: Option<(u32, u32)>,
        is_solved: bool,
    ) -> Self {
        assert_eq!(tiles.len(), (width as usize) * (height as usize));
        assert_eq!(boxes.len(), (width as usize) * (height as usize));
        Self {
            width,
            height,
            tiles,
            boxes,
            player,
            selected_box,
            is_solved,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn tile(&self, x: u32, y: u32) -> TileKind {
        self.tiles[(y * self.width + x) as usize]
    }

    pub fn has_box(&self, x: u32, y: u32) -> bool {
        self.boxes[(y * self.width + x) as usize]
    }

    pub fn player(&self) -> Option<(u32, u32)> {
        self.player
    }

    pub fn selected_box(&self) -> Option<(u32, u32)> {
        self.selected_box
    }

    pub fn is_solved(&self) -> bool {
        self.is_solved
    }
}

pub struct GameBoardPresenter {
    level: ParsedLevel,
}

impl GameBoardPresenter {
    pub fn new(level: ParsedLevel) -> Self {
        Self { level }
    }

    pub fn render_board(
        &self,
        player: Option<(u32, u32)>,
        box_positions: &HashSet<(u32, u32)>,
        selected_box: Option<(u32, u32)>,
        is_solved: bool,
    ) -> BoardView {
        let mut tiles = Vec::with_capacity((self.level.width * self.level.height) as usize);
        let mut boxes = Vec::with_capacity((self.level.width * self.level.height) as usize);
        for y in 0..self.level.height {
            for x in 0..self.level.width {
                let tile = match self.level.cell(x, y) {
                    LevelCell::Void => TileKind::Void,
                    LevelCell::Floor if self.level.is_goal(x, y) => TileKind::Goal,
                    LevelCell::Floor => TileKind::Floor,
                };
                tiles.push(tile);
                boxes.push(box_positions.contains(&(x, y)));
            }
        }
        let selected_box = selected_box.filter(|pos| box_positions.contains(pos));
        BoardView::new(
            self.level.width,
            self.level.height,
            tiles,
            boxes,
            player,
            selected_box,
            is_solved,
        )
    }
}
