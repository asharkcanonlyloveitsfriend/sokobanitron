use sokobanitron_core::pathfinder::{BoxPathfinder, PlayerPathfinder, Position};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct GameEngine {
    width: usize,
    height: usize,
    base_walkable: Vec<Vec<bool>>,
    goals: HashSet<Position>,
    initial_player: Position,
    initial_boxes: HashSet<Position>,
    player: Position,
    boxes: HashSet<Position>,
    box_move_history: Vec<Vec<Position>>,
}

impl GameEngine {
    pub fn from_ascii(ascii: &str) -> Option<Self> {
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
                        player = Some(Position::new(row, col));
                        if ch == '+' {
                            goals.insert(Position::new(row, col));
                        }
                        *cell = true;
                    }
                    '$' | '*' => {
                        boxes.insert(Position::new(row, col));
                        if ch == '*' {
                            goals.insert(Position::new(row, col));
                        }
                        *cell = true;
                    }
                    '.' => {
                        goals.insert(Position::new(row, col));
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
        })
    }

    pub fn player(&self) -> Position {
        self.player
    }

    pub fn boxes(&self) -> &HashSet<Position> {
        &self.boxes
    }

    pub fn box_move_history(&self) -> &[Vec<Position>] {
        &self.box_move_history
    }

    pub fn last_box_move_destination(&self) -> Option<Position> {
        self.box_move_history
            .last()
            .and_then(|path| path.last())
            .copied()
    }

    pub fn is_level_solved(&self) -> bool {
        self.boxes
            .iter()
            .all(|box_pos| self.goals.contains(box_pos))
    }

    pub fn is_clean_solution(&self) -> bool {
        self.is_level_solved() && self.boxes.len() == self.initial_boxes.len()
    }

    pub fn is_at_start(&self) -> bool {
        self.player == self.initial_player && self.boxes == self.initial_boxes
    }

    pub fn can_restart(&self) -> bool {
        !self.is_at_start()
    }

    pub fn can_undo(&self) -> bool {
        !self.box_move_history.is_empty()
    }

    pub fn move_player_to(&mut self, to: Position) -> bool {
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

    pub fn move_box_to(&mut self, from: Position, to: Position) -> Option<Vec<Position>> {
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

    pub fn push_box_into_void(&mut self, from: Position, to: Position) -> bool {
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

    pub fn undo(&mut self) -> Option<Vec<Position>> {
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
        let new_player = Position::new(new_player_row, new_player_col);

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

#[cfg(test)]
mod tests {
    use super::GameEngine;
    use sokobanitron_core::pathfinder::Position;

    #[test]
    fn push_box_into_void_then_undo_restores_box() {
        let ascii = "#####\n# @ #\n# $ #\n#####";
        let mut engine = GameEngine::from_ascii(ascii).expect("expected valid level");

        let pushed = engine.push_box_into_void(Position::new(2, 2), Position::new(3, 2));
        assert!(pushed);
        assert_eq!(engine.player(), Position::new(2, 2));
        assert!(engine.boxes().is_empty());

        let undo_path = engine.undo().expect("expected first undo to succeed");
        assert_eq!(undo_path, vec![Position::new(2, 2), Position::new(3, 2)]);
        assert_eq!(engine.player(), Position::new(1, 2));
        assert!(engine.boxes().contains(&Position::new(2, 2)));

        assert!(
            engine.undo().is_none(),
            "second undo should fail once history is empty"
        );
    }

    #[test]
    fn move_box_to_then_undo_restores_positions() {
        let ascii = "#####\n# @ #\n# $ #\n#   #\n#####";
        let mut engine = GameEngine::from_ascii(ascii).expect("expected valid level");

        let path = engine
            .move_box_to(Position::new(2, 2), Position::new(3, 2))
            .expect("expected box move to succeed");
        assert_eq!(path.first().copied(), Some(Position::new(2, 2)));
        assert_eq!(path.last().copied(), Some(Position::new(3, 2)));
        assert!(engine.boxes().contains(&Position::new(3, 2)));

        let undo_path = engine.undo().expect("expected undo to succeed");
        assert_eq!(undo_path, path);
        assert_eq!(engine.player(), Position::new(1, 2));
        assert!(engine.boxes().contains(&Position::new(2, 2)));
        assert!(!engine.boxes().contains(&Position::new(3, 2)));
    }

    #[test]
    fn can_undo_is_true_after_level_is_solved_when_history_exists() {
        let ascii = "#####\n# @ #\n# $.#\n#####";
        let mut engine = GameEngine::from_ascii(ascii).expect("expected valid level");

        assert!(
            engine
                .move_box_to(Position::new(2, 2), Position::new(2, 3))
                .is_some(),
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
        let mut engine = GameEngine::from_ascii(ascii).expect("expected valid level");

        let first_path = engine
            .move_box_to(Position::new(1, 3), Position::new(1, 4))
            .expect("expected first box move");
        assert_eq!(
            engine.last_box_move_destination(),
            first_path.last().copied()
        );

        let second_path = engine
            .move_box_to(Position::new(2, 3), Position::new(2, 4))
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
        let mut engine = GameEngine::from_ascii(ascii).expect("expected valid level");

        let first_path = engine
            .move_box_to(Position::new(1, 3), Position::new(1, 4))
            .expect("expected first box move");
        let second_path = engine
            .move_box_to(Position::new(2, 3), Position::new(2, 4))
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
