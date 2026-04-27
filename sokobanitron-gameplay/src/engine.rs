use crate::board_cell::BoardCell;
use crate::session::{GameplayMode, GameplayMoveDirection};
use sokobanitron_core::pathfinder::{BoxPathfinder, PlayerPathfinder, Position as GridPosition};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub(crate) struct GameEngine {
    width: usize,
    height: usize,
    base_walkable: Vec<Vec<bool>>,
    goals: HashSet<GridPosition>,
    initial_player: GridPosition,
    initial_boxes: HashSet<GridPosition>,
    player: GridPosition,
    boxes: HashSet<GridPosition>,
    box_move_history: Vec<Vec<GridPosition>>,
    mode: GameplayMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StepOutcome {
    None,
    PlayerMoved { to: BoardCell },
    BoxMoved { path: Vec<BoardCell> },
    BoxRemoved { to: BoardCell },
}

impl GameEngine {
    pub(crate) fn from_ascii_with_mode(ascii: &str, mode: GameplayMode) -> Option<Self> {
        let lines = ascii.lines().collect::<Vec<_>>();
        let height = lines.len();
        let width = lines.iter().map(|line| line.len()).max()?;
        if height == 0 || width == 0 {
            return None;
        }

        let mut player = None;
        let mut boxes = HashSet::new();
        let mut goals = HashSet::new();
        let mut base_walkable = vec![vec![false; width]; height];

        for (row, (line, walkable_row)) in lines.iter().zip(base_walkable.iter_mut()).enumerate() {
            for (col, cell) in walkable_row.iter_mut().enumerate().take(width) {
                let ch = line.as_bytes().get(col).copied().unwrap_or(b' ') as char;
                match ch {
                    '#' => {
                        *cell = false;
                    }
                    '@' | '+' => {
                        player = Some(GridPosition::new(row, col));
                        if ch == '+' {
                            goals.insert(GridPosition::new(row, col));
                        }
                        *cell = true;
                    }
                    '$' | '*' => {
                        boxes.insert(GridPosition::new(row, col));
                        if ch == '*' {
                            goals.insert(GridPosition::new(row, col));
                        }
                        *cell = true;
                    }
                    '.' => {
                        goals.insert(GridPosition::new(row, col));
                        *cell = true;
                    }
                    _ => {
                        *cell = true;
                    }
                }
            }
        }

        let initial_player = player?;
        let initial_boxes = boxes.clone();
        Some(Self {
            width,
            height,
            base_walkable,
            goals,
            initial_player,
            initial_boxes,
            player: initial_player,
            boxes,
            box_move_history: Vec::new(),
            mode,
        })
    }

    pub(crate) fn player(&self) -> BoardCell {
        board_cell(self.player)
    }

    pub(crate) fn boxes(&self) -> impl Iterator<Item = BoardCell> + '_ {
        self.boxes.iter().copied().map(board_cell)
    }

    pub(crate) fn box_move_history_cells(&self) -> Vec<Vec<BoardCell>> {
        self.box_move_history
            .iter()
            .map(|path| path.iter().copied().map(board_cell).collect())
            .collect()
    }

    pub(crate) fn last_box_move_destination(&self) -> Option<BoardCell> {
        self.last_box_move_destination_position().map(board_cell)
    }

    pub(crate) fn has_box(&self, cell: BoardCell) -> bool {
        self.boxes.contains(&grid_position(cell))
    }

    pub(crate) fn is_level_solved(&self) -> bool {
        self.boxes
            .iter()
            .all(|box_pos| self.goals.contains(box_pos))
    }

    pub(crate) fn is_clean_solution(&self) -> bool {
        self.is_level_solved() && self.boxes.len() == self.initial_boxes.len()
    }

    pub(crate) fn is_at_start(&self) -> bool {
        self.player == self.initial_player && self.boxes == self.initial_boxes
    }

    pub(crate) fn can_restart(&self) -> bool {
        !self.is_at_start()
    }

    pub(crate) fn can_undo(&self) -> bool {
        !self.box_move_history.is_empty()
    }

    pub(crate) fn move_player_to(&mut self, to: BoardCell) -> bool {
        self.move_player_to_position(grid_position(to))
    }

    pub(crate) fn move_box_to(&mut self, from: BoardCell, to: BoardCell) -> Option<Vec<BoardCell>> {
        self.move_box_to_position(grid_position(from), grid_position(to))
            .map(|path| path.into_iter().map(board_cell).collect())
    }

    pub(crate) fn push_box_into_void(&mut self, from: BoardCell, to: BoardCell) -> bool {
        self.push_box_into_void_position(grid_position(from), grid_position(to))
    }

    pub(crate) fn undo(&mut self) -> Option<Vec<BoardCell>> {
        self.undo_position()
            .map(|path| path.into_iter().map(board_cell).collect())
    }

    pub(crate) fn step_direction(&mut self, direction: GameplayMoveDirection) -> StepOutcome {
        let Some(next) = offset_position(self.player, direction) else {
            return StepOutcome::None;
        };
        if !self.is_inside(next.row, next.col) {
            return StepOutcome::None;
        }

        if self.boxes.contains(&next) {
            let Some(push_to) = offset_position(next, direction) else {
                return StepOutcome::None;
            };
            if !self.is_inside(push_to.row, push_to.col) {
                return StepOutcome::None;
            }
            if self.base_walkable[push_to.row][push_to.col] {
                if let Some(path) = self.move_box_to_position(next, push_to) {
                    return StepOutcome::BoxMoved {
                        path: path.into_iter().map(board_cell).collect(),
                    };
                }
            } else if self.push_box_into_void_position(next, push_to) {
                return StepOutcome::BoxRemoved {
                    to: board_cell(push_to),
                };
            }
            return StepOutcome::None;
        }

        if self.base_walkable[next.row][next.col] {
            self.player = next;
            StepOutcome::PlayerMoved {
                to: board_cell(next),
            }
        } else {
            StepOutcome::None
        }
    }

    fn last_box_move_destination_position(&self) -> Option<GridPosition> {
        self.box_move_history
            .last()
            .and_then(|path| path.last())
            .copied()
    }

    fn move_player_to_position(&mut self, to: GridPosition) -> bool {
        if !self.is_inside(to.row, to.col) || !self.base_walkable[to.row][to.col] {
            return false;
        }
        if to == self.player {
            return false;
        }

        let walkable = self.walkable_with_boxes();
        let mut pathfinder = PlayerPathfinder::from_rows(walkable);
        let can_find = pathfinder.can_find_path(self.player, to, None);
        if !can_find {
            return false;
        }

        self.player = to;
        true
    }

    fn move_box_to_position(
        &mut self,
        from: GridPosition,
        to: GridPosition,
    ) -> Option<Vec<GridPosition>> {
        if !self.boxes.contains(&from) {
            return None;
        }
        if !self.is_inside(to.row, to.col) || !self.base_walkable[to.row][to.col] {
            return None;
        }

        let full_grid = self.walkable_with_boxes();
        let mut pathfinder = BoxPathfinder::new(full_grid, from, self.player);
        let box_path = pathfinder.find_box_path(to)?;

        let final_player_pos = if box_path.len() >= 2 {
            box_path[box_path.len() - 2]
        } else {
            *box_path.last()?
        };

        self.boxes.remove(&from);
        self.boxes.insert(to);
        self.player = final_player_pos;
        self.box_move_history.push(box_path.clone());
        Some(box_path)
    }

    fn push_box_into_void_position(&mut self, from: GridPosition, to: GridPosition) -> bool {
        if self.mode == GameplayMode::Strict {
            return false;
        }
        if !self.boxes.contains(&from) {
            return false;
        }
        if !self.is_inside(to.row, to.col) {
            return false;
        }
        if self.base_walkable[to.row][to.col] {
            return false;
        }

        let dir_row = from.row as isize - self.player.row as isize;
        let dir_col = from.col as isize - self.player.col as isize;
        let is_adjacent_push = dir_row.unsigned_abs() + dir_col.unsigned_abs() == 1;
        if !is_adjacent_push {
            return false;
        }

        let Some(pushed_row) = from.row.checked_add_signed(dir_row) else {
            return false;
        };
        let Some(pushed_col) = from.col.checked_add_signed(dir_col) else {
            return false;
        };
        if pushed_row != to.row || pushed_col != to.col {
            return false;
        }

        self.boxes.remove(&from);
        self.player = from;
        self.box_move_history.push(vec![from, to]);
        true
    }

    fn undo_position(&mut self) -> Option<Vec<GridPosition>> {
        let path = self.box_move_history.pop()?;
        if path.len() < 2 {
            return None;
        }

        let box_from = *path.first()?;
        let box_to = *path.last()?;
        let first_step_row = path[1].row as isize - box_from.row as isize;
        let first_step_col = path[1].col as isize - box_from.col as isize;

        let new_player_row = box_from.row.checked_add_signed(-first_step_row)?;
        let new_player_col = box_from.col.checked_add_signed(-first_step_col)?;
        let new_player = GridPosition::new(new_player_row, new_player_col);

        self.boxes.remove(&box_to);
        self.boxes.insert(box_from);
        self.player = new_player;
        Some(path)
    }

    fn is_inside(&self, row: usize, col: usize) -> bool {
        row < self.height && col < self.width
    }

    fn walkable_with_boxes(&self) -> Vec<Vec<bool>> {
        let mut walkable = self.base_walkable.clone();
        for box_pos in &self.boxes {
            if self.is_inside(box_pos.row, box_pos.col) {
                walkable[box_pos.row][box_pos.col] = false;
            }
        }
        walkable
    }
}

fn grid_position(cell: BoardCell) -> GridPosition {
    GridPosition::new(cell.y as usize, cell.x as usize)
}

fn board_cell(position: GridPosition) -> BoardCell {
    BoardCell::new(position.col as u32, position.row as u32)
}

fn offset_position(
    position: GridPosition,
    direction: GameplayMoveDirection,
) -> Option<GridPosition> {
    let (row_delta, col_delta) = match direction {
        GameplayMoveDirection::Up => (-1, 0),
        GameplayMoveDirection::Down => (1, 0),
        GameplayMoveDirection::Left => (0, -1),
        GameplayMoveDirection::Right => (0, 1),
    };
    Some(GridPosition::new(
        position.row.checked_add_signed(row_delta)?,
        position.col.checked_add_signed(col_delta)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::GameEngine;
    use crate::{BoardCell, GameplayMode};

    fn cell(x: u32, y: u32) -> BoardCell {
        BoardCell::new(x, y)
    }

    #[test]
    fn push_box_into_void_then_undo_restores_box() {
        let ascii = "#####\n# @ #\n# $ #\n#####";
        let mut engine = GameEngine::from_ascii_with_mode(ascii, GameplayMode::Normal)
            .expect("expected valid level");

        let pushed = engine.push_box_into_void(cell(2, 2), cell(2, 3));
        assert!(pushed);
        assert_eq!(engine.player(), cell(2, 2));
        assert!(engine.boxes().next().is_none());

        let undo_path = engine.undo().expect("expected first undo to succeed");
        assert_eq!(undo_path, vec![cell(2, 2), cell(2, 3)]);
        assert_eq!(engine.player(), cell(2, 1));
        assert!(engine.has_box(cell(2, 2)));

        assert!(
            engine.undo().is_none(),
            "second undo should fail once history is empty"
        );
    }

    #[test]
    fn move_box_to_then_undo_restores_positions() {
        let ascii = "#####\n# @ #\n# $ #\n#   #\n#####";
        let mut engine = GameEngine::from_ascii_with_mode(ascii, GameplayMode::Normal)
            .expect("expected valid level");

        let path = engine
            .move_box_to(cell(2, 2), cell(2, 3))
            .expect("expected box move to succeed");
        assert_eq!(path.first().copied(), Some(cell(2, 2)));
        assert_eq!(path.last().copied(), Some(cell(2, 3)));
        assert!(engine.has_box(cell(2, 3)));

        let undo_path = engine.undo().expect("expected undo to succeed");
        assert_eq!(undo_path, path);
        assert_eq!(engine.player(), cell(2, 1));
        assert!(engine.has_box(cell(2, 2)));
        assert!(!engine.has_box(cell(2, 3)));
    }

    #[test]
    fn can_undo_is_true_after_level_is_solved_when_history_exists() {
        let ascii = "#####\n# @ #\n# $.#\n#####";
        let mut engine = GameEngine::from_ascii_with_mode(ascii, GameplayMode::Normal)
            .expect("expected valid level");

        assert!(
            engine.move_box_to(cell(2, 2), cell(3, 2)).is_some(),
            "expected box move to solve level"
        );
        assert!(engine.is_level_solved(), "expected level to be solved");
        assert!(
            engine.can_undo(),
            "undo should remain available while move history exists"
        );
        assert!(
            engine.can_restart(),
            "restart should remain available after solving"
        );
    }

    #[test]
    fn last_box_move_destination_tracks_latest_remaining_history() {
        let ascii = "########\n#@ $   #\n#  $ . #\n########";
        let mut engine = GameEngine::from_ascii_with_mode(ascii, GameplayMode::Normal)
            .expect("expected valid level");

        let first_path = engine
            .move_box_to(cell(3, 1), cell(4, 1))
            .expect("expected first box move");
        assert_eq!(
            engine.last_box_move_destination(),
            first_path.last().copied()
        );

        let second_path = engine
            .move_box_to(cell(3, 2), cell(4, 2))
            .expect("expected second box move");
        assert_eq!(
            engine.last_box_move_destination(),
            second_path.last().copied()
        );

        let _ = engine.undo().expect("expected undo to succeed");
        assert_eq!(
            engine.last_box_move_destination(),
            first_path.last().copied()
        );
    }

    #[test]
    fn undo_can_be_applied_until_history_is_empty() {
        let ascii = "########\n#@ $   #\n#  $ . #\n########";
        let mut engine = GameEngine::from_ascii_with_mode(ascii, GameplayMode::Normal)
            .expect("expected valid level");

        let first_path = engine
            .move_box_to(cell(3, 1), cell(4, 1))
            .expect("expected first box move");
        let second_path = engine
            .move_box_to(cell(3, 2), cell(4, 2))
            .expect("expected second box move");

        assert_eq!(engine.undo().expect("expected first undo"), second_path);
        assert!(
            engine.can_undo(),
            "history should still permit undo while earlier moves remain"
        );
        assert_eq!(engine.undo().expect("expected second undo"), first_path);
        assert!(
            !engine.can_undo(),
            "undo should become unavailable once history is empty"
        );
        assert_eq!(engine.last_box_move_destination(), None);
    }
}
