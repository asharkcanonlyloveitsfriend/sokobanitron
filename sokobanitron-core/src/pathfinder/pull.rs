use crate::pathfinder::{PlayerPathfinder, Position};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct PullPathResult {
    pub box_path: Vec<Position>,
    pub player_start: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct State {
    box_pos: Position,
    player_pos: Position,
}

#[derive(Debug, Clone)]
pub struct PullPathfinder {
    width: usize,
    height: usize,
    cell_count: usize,
    walkable_grid: Vec<u8>,
    box_start: Position,
    player_start: Option<Position>,
    player_pathfinder: PlayerPathfinder,
    visited: Vec<u32>,
    generation: u32,
    parents: Vec<Option<State>>,
}

impl PullPathfinder {
    const DIRECTIONS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

    pub fn new(
        full_grid: Vec<Vec<bool>>,
        box_start: Position,
        player_start: Option<Position>,
    ) -> Self {
        let height = full_grid.len();
        let width = full_grid.first().map_or(0, Vec::len);
        for row in &full_grid {
            assert_eq!(row.len(), width, "all grid rows must have the same width");
        }
        assert!(
            box_start.row < height && box_start.col < width,
            "box_start must be in bounds"
        );
        if let Some(start) = player_start {
            assert!(
                start.row < height && start.col < width,
                "player_start must be in bounds"
            );
        }

        let walkable_grid = full_grid
            .into_iter()
            .flatten()
            .map(u8::from)
            .collect::<Vec<_>>();

        let planning_rows = walkable_grid
            .chunks(width)
            .map(|row| row.iter().map(|&cell| cell != 0).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        let cell_count = width * height;
        let state_count = cell_count * cell_count;

        Self {
            width,
            height,
            cell_count,
            walkable_grid,
            box_start,
            player_start,
            player_pathfinder: PlayerPathfinder::from_rows(planning_rows),
            visited: vec![0u32; state_count],
            generation: 1,
            parents: vec![None; state_count],
        }
    }

    #[inline]
    fn idx(&self, pos: Position) -> usize {
        pos.row * self.width + pos.col
    }

    #[inline]
    fn state_index(&self, state: State) -> usize {
        let box_idx = self.idx(state.box_pos);
        let player_idx = self.idx(state.player_pos);
        box_idx * self.cell_count + player_idx
    }

    #[inline]
    fn is_inside(&self, pos: Position) -> bool {
        pos.row < self.height && pos.col < self.width
    }

    #[inline]
    fn is_walkable(&self, pos: Position) -> bool {
        self.walkable_grid[self.idx(pos)] != 0
    }

    #[inline]
    fn offset_pos(pos: Position, dr: isize, dc: isize) -> Option<Position> {
        let row = pos.row.checked_add_signed(dr)?;
        let col = pos.col.checked_add_signed(dc)?;
        Some(Position::new(row, col))
    }

    fn build_result(&self, end_state: State) -> PullPathResult {
        let mut reversed = Vec::new();
        let mut current = Some(end_state);
        while let Some(state) = current {
            reversed.push(state.box_pos);
            let idx = self.state_index(state);
            current = self.parents[idx];
        }
        reversed.reverse();
        PullPathResult {
            box_path: reversed,
            player_start: end_state.player_pos,
        }
    }

    fn enqueue_start_state(&mut self, queue: &mut VecDeque<State>, state: State, generation: u32) {
        if !self.is_walkable(state.player_pos) || state.player_pos == state.box_pos {
            return;
        }
        let idx = self.state_index(state);
        if self.visited[idx] == generation {
            return;
        }
        self.visited[idx] = generation;
        self.parents[idx] = None;
        queue.push_back(state);
    }

    fn seed_start_states(&mut self, queue: &mut VecDeque<State>, generation: u32) {
        if let Some(player_start) = self.player_start {
            self.enqueue_start_state(
                queue,
                State {
                    box_pos: self.box_start,
                    player_pos: player_start,
                },
                generation,
            );
        } else {
            for row in 0..self.height {
                for col in 0..self.width {
                    self.enqueue_start_state(
                        queue,
                        State {
                            box_pos: self.box_start,
                            player_pos: Position::new(row, col),
                        },
                        generation,
                    );
                }
            }
        }
    }

    fn traverse_pull_states<Visit>(&mut self, mut visit: Visit)
    where
        Visit: FnMut(&Self, State) -> bool,
    {
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        let mut queue = VecDeque::new();
        self.seed_start_states(&mut queue, generation);

        while let Some(state) = queue.pop_front() {
            if !visit(self, state) {
                return;
            }

            for (dr, dc) in Self::DIRECTIONS {
                // Reverse transition:
                // current box at B, previous box at B-dr/dc, previous player at B-2*dir.
                let Some(prev_box) = Self::offset_pos(state.box_pos, -dr, -dc) else {
                    continue;
                };
                let Some(prev_player) = Self::offset_pos(prev_box, -dr, -dc) else {
                    continue;
                };
                if !self.is_inside(prev_box)
                    || !self.is_inside(prev_player)
                    || !self.is_walkable(prev_box)
                    || !self.is_walkable(prev_player)
                {
                    continue;
                }

                // Current player must be able to reach the box's previous cell while box stays fixed.
                if !self.player_pathfinder.can_find_path(
                    state.player_pos,
                    prev_box,
                    Some(state.box_pos),
                ) {
                    continue;
                }

                let next_state = State {
                    box_pos: prev_box,
                    player_pos: prev_player,
                };
                let idx = self.state_index(next_state);
                if self.visited[idx] == generation {
                    continue;
                }
                self.visited[idx] = generation;
                self.parents[idx] = Some(state);
                queue.push_back(next_state);
            }
        }
    }

    pub fn find_pull_path(&mut self, origin: Position) -> Option<PullPathResult> {
        if self.box_start == origin {
            return None;
        }
        if !self.is_inside(origin) || !self.is_walkable(origin) {
            return None;
        }
        let mut found = None;
        self.traverse_pull_states(|pathfinder, state| {
            if state.box_pos == origin {
                found = Some(pathfinder.build_result(state));
                return false;
            }
            true
        });
        found
    }

    pub fn find_all_pull_paths(&mut self) -> Vec<(Position, PullPathResult)> {
        let mut found_origin = vec![false; self.cell_count];
        let mut results = Vec::new();

        self.traverse_pull_states(|pathfinder, state| {
            let box_idx = pathfinder.idx(state.box_pos);
            if state.box_pos != pathfinder.box_start && !found_origin[box_idx] {
                found_origin[box_idx] = true;
                results.push((state.box_pos, pathfinder.build_result(state)));
            }
            true
        });

        results
    }
}

#[cfg(test)]
mod tests {
    use super::PullPathfinder;
    use crate::pathfinder::Position;

    #[test]
    fn returns_none_when_origin_matches_destination() {
        let grid = vec![vec![true, true], vec![true, true]];
        let mut pathfinder = PullPathfinder::new(grid, Position::new(0, 0), None);
        assert!(pathfinder.find_pull_path(Position::new(0, 0)).is_none());
    }

    #[test]
    fn wildcard_player_finds_path_and_player_start() {
        let grid = vec![vec![true; 5]; 5];
        let mut pathfinder = PullPathfinder::new(grid, Position::new(2, 3), None);
        let result = pathfinder
            .find_pull_path(Position::new(2, 1))
            .expect("expected pull path");
        assert_eq!(result.box_path.first().copied(), Some(Position::new(2, 3)));
        assert_eq!(result.box_path.last().copied(), Some(Position::new(2, 1)));
        assert_ne!(result.player_start, Position::new(2, 1));
    }

    #[test]
    fn explicit_player_start_is_used() {
        let grid = vec![vec![true; 5]; 5];
        let current_player = Position::new(2, 2);
        let mut pathfinder = PullPathfinder::new(grid, Position::new(2, 3), Some(current_player));
        let result = pathfinder
            .find_pull_path(Position::new(2, 1))
            .expect("expected pull path");
        assert_eq!(result.player_start, Position::new(2, 0));
    }

    #[test]
    fn find_all_pull_paths_returns_reachable_origins() {
        let grid = vec![vec![true; 5]; 5];
        let mut pathfinder = PullPathfinder::new(grid, Position::new(2, 3), None);
        let all = pathfinder.find_all_pull_paths();
        assert!(all.iter().any(|(origin, _)| *origin == Position::new(2, 1)));
        assert!(all.iter().all(|(origin, _)| *origin != Position::new(2, 3)));
    }
}
