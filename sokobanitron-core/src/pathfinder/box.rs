use crate::pathfinder::stats::PathfinderStats;
use crate::pathfinder::{PlayerPathfinder, Position};
use crate::stat;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct State {
    box_pos: Position,
    player_pos: Position,
}

#[derive(Debug, Clone)]
pub struct BoxPathfinder {
    width: usize,
    height: usize,
    cell_count: usize,
    // Static walkable floor grid used for box planning.
    // The current box position is handled dynamically in the player pathfinder.
    walkable_grid: Vec<u8>,
    start_state: State,
    player_pathfinder: PlayerPathfinder,
    stats: PathfinderStats,
    visited: Vec<u32>,
    generation: u32,
    parents: Vec<Option<State>>,
}

impl BoxPathfinder {
    const DEAD_ENABLE_AFTER_EXPANDED: u64 = 16;
    const DIRECTIONS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

    pub fn new(full_grid: Vec<Vec<bool>>, box_start: Position, player_start: Position) -> Self {
        let height = full_grid.len();
        let width = full_grid.first().map_or(0, Vec::len);
        for row in &full_grid {
            assert_eq!(row.len(), width, "all grid rows must have the same width");
        }
        assert!(
            box_start.row < height && box_start.col < width,
            "box_start must be in bounds"
        );
        assert!(
            player_start.row < height && player_start.col < width,
            "player_start must be in bounds"
        );

        let mut walkable_grid = full_grid
            .into_iter()
            .flatten()
            .map(u8::from)
            .collect::<Vec<_>>();
        walkable_grid[box_start.row * width + box_start.col] = 1;

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
            start_state: State {
                box_pos: box_start,
                player_pos: player_start,
            },
            player_pathfinder: PlayerPathfinder::from_rows(planning_rows),
            stats: PathfinderStats::default(),
            visited: vec![0u32; state_count],
            generation: 1,
            parents: vec![None; state_count],
        }
    }

    #[inline]
    pub fn stats(&self) -> &PathfinderStats {
        &self.stats
    }

    #[inline]
    pub fn stats_mut(&mut self) -> &mut PathfinderStats {
        &mut self.stats
    }

    #[inline]
    pub fn reset_stats(&mut self) {
        self.stats = PathfinderStats::default();
    }

    pub fn find_box_path(&mut self, to: Position) -> Option<Vec<Position>> {
        if self.start_state.box_pos == to {
            return None;
        }

        let mut dead: Option<Vec<u8>> = None;
        let mut expanded_count = 0u64;

        // Increment generation instead of clearing the visited array.
        // A state is considered visited in the current search when
        // visited[idx] == generation.
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;

        let mut queue = VecDeque::new();
        queue.push_back(self.start_state);
        stat!(self, states_pushed += 1);

        let start_idx = self.state_index(self.start_state);
        self.visited[start_idx] = generation;
        self.parents[start_idx] = None;

        while let Some(state) = queue.pop_front() {
            let State {
                box_pos,
                player_pos,
            } = state;
            stat!(self, states_expanded += 1);
            expanded_count += 1;

            if dead.is_none() && expanded_count >= Self::DEAD_ENABLE_AFTER_EXPANDED {
                dead = Some(self.compute_dead_squares(to));
            }

            if box_pos == to {
                return Some(self.build_box_path(state));
            }

            for (dr, dc) in Self::DIRECTIONS {
                stat!(self, push_attempts += 1);

                let Some(new_box) = Self::offset_pos(box_pos, dr, dc) else {
                    continue;
                };
                let Some(player_push_from) = Self::offset_pos(box_pos, -dr, -dc) else {
                    continue;
                };

                // Basic legality: bounds and floor checks
                if !self.is_inside(new_box)
                    || !self.is_inside(player_push_from)
                    || !self.is_walkable(new_box)
                    || !self.is_walkable(player_push_from)
                {
                    continue;
                }

                // Dead-square pruning
                if let Some(dead_grid) = &dead
                    && dead_grid[self.idx(new_box)] != 0
                {
                    continue;
                }

                // Player must be able to reach the square behind the box
                stat!(self, player_pathfinder_calls += 1);
                if !self.player_pathfinder.can_find_path(
                    player_pos,
                    player_push_from,
                    Some(box_pos),
                ) {
                    continue;
                }

                stat!(self, player_pathfinder_successes += 1);

                let new_state = State {
                    box_pos: new_box,
                    player_pos: box_pos,
                };

                let idx = self.state_index(new_state);
                if self.visited[idx] != generation {
                    self.visited[idx] = generation;
                    self.parents[idx] = Some(state);
                    queue.push_back(new_state);
                    stat!(self, states_pushed += 1);
                }
            }
        }

        None
    }

    fn build_box_path(&self, end_state: State) -> Vec<Position> {
        let mut reversed = Vec::new();
        let mut current = Some(end_state);

        while let Some(state) = current {
            reversed.push(state.box_pos);
            let idx = self.state_index(state);
            current = self.parents[idx];
        }

        reversed.reverse();
        reversed
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

    fn offset_pos(pos: Position, dr: isize, dc: isize) -> Option<Position> {
        let row = pos.row.checked_add_signed(dr)?;
        let col = pos.col.checked_add_signed(dc)?;
        Some(Position::new(row, col))
    }

    fn compute_dead_squares(&self, goal: Position) -> Vec<u8> {
        let mut alive = vec![0u8; self.width * self.height];
        let mut queue = VecDeque::new();

        let enqueue = |p: Position, alive: &mut [u8], queue: &mut VecDeque<Position>| {
            let idx = self.idx(p);
            if alive[idx] == 0 {
                alive[idx] = 1;
                queue.push_back(p);
            }
        };

        if !self.is_inside(goal) || !self.is_walkable(goal) {
            let mut dead = vec![0u8; self.width * self.height];
            for row in 0..self.height {
                for col in 0..self.width {
                    let pos = Position::new(row, col);
                    if self.is_walkable(pos) {
                        dead[self.idx(pos)] = 1;
                    }
                }
            }
            return dead;
        }

        enqueue(goal, &mut alive, &mut queue);

        while let Some(cur) = queue.pop_front() {
            for (dr, dc) in Self::DIRECTIONS {
                let Some(prev) = Self::offset_pos(cur, -dr, -dc) else {
                    continue;
                };
                let Some(push_pos) = Self::offset_pos(prev, -dr, -dc) else {
                    continue;
                };

                if !self.is_inside(prev) || !self.is_inside(push_pos) {
                    continue;
                }
                if !self.is_walkable(prev) || !self.is_walkable(push_pos) {
                    continue;
                }

                enqueue(prev, &mut alive, &mut queue);
            }
        }

        let mut dead = vec![0u8; self.width * self.height];
        for row in 0..self.height {
            for col in 0..self.width {
                let pos = Position::new(row, col);
                let idx = self.idx(pos);
                if pos != goal && self.is_walkable(pos) && alive[idx] == 0 {
                    dead[idx] = 1;
                }
            }
        }
        dead
    }
}
