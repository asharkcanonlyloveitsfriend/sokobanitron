use crate::pathfinder::stats::PathfinderStats;
use crate::stat;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

impl Position {
    #[inline]
    pub const fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

#[derive(Debug, Clone)]
pub struct PlayerPathfinder {
    width: usize,
    height: usize,
    walkable: Vec<u8>,
    neighbors: Vec<[usize; 4]>,
    neighbor_counts: Vec<u8>,
    visited_stamp: Vec<u32>,
    queue: Vec<usize>,
    current_stamp: u32,
    stats: PathfinderStats,
}

impl PlayerPathfinder {
    pub fn from_rows(walkable_rows: Vec<Vec<bool>>) -> Self {
        let height = walkable_rows.len();
        let width = walkable_rows.first().map_or(0, Vec::len);

        for row in &walkable_rows {
            assert_eq!(
                row.len(),
                width,
                "all walkable rows must have the same width"
            );
        }

        let walkable = walkable_rows
            .into_iter()
            .flatten()
            .map(u8::from)
            .collect::<Vec<_>>();

        let cells = width * height;
        let mut neighbors = vec![[0usize; 4]; cells];
        let mut neighbor_counts = vec![0u8; cells];
        for row in 0..height {
            for col in 0..width {
                let idx = row * width + col;
                let mut count = 0u8;

                if row > 0 {
                    neighbors[idx][count as usize] = (row - 1) * width + col;
                    count += 1;
                }
                if row + 1 < height {
                    neighbors[idx][count as usize] = (row + 1) * width + col;
                    count += 1;
                }
                if col > 0 {
                    neighbors[idx][count as usize] = row * width + (col - 1);
                    count += 1;
                }
                if col + 1 < width {
                    neighbors[idx][count as usize] = row * width + (col + 1);
                    count += 1;
                }

                neighbor_counts[idx] = count;
            }
        }

        Self {
            width,
            height,
            neighbors,
            neighbor_counts,
            visited_stamp: vec![0; cells],
            queue: Vec::with_capacity(cells),
            walkable,
            current_stamp: 1,
            stats: PathfinderStats::default(),
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

    pub fn can_find_path(
        &mut self,
        from: Position,
        to: Position,
        blocked: Option<Position>,
    ) -> bool {
        if from == to {
            return true;
        }

        if from.row >= self.height
            || from.col >= self.width
            || to.row >= self.height
            || to.col >= self.width
        {
            return false;
        }

        let stamp = self.current_stamp;
        self.current_stamp = self.current_stamp.wrapping_add(1);

        if self.current_stamp == 0 {
            self.current_stamp = 1;
            self.visited_stamp.fill(0);
        }

        let blocked_idx = blocked.and_then(|pos| {
            if pos.row < self.height && pos.col < self.width {
                Some(self.idx(pos.row, pos.col))
            } else {
                None
            }
        });
        let start_idx = self.idx(from.row, from.col);
        let target_index = self.idx(to.row, to.col);

        self.queue.clear();
        self.queue.push(start_idx);
        self.visited_stamp[start_idx] = stamp;
        stat!(self, player_nodes_pushed += 1);
        let mut head = 0;

        while head < self.queue.len() {
            let current = self.queue[head];
            head += 1;
            stat!(self, player_nodes_expanded += 1);

            if current == target_index {
                return true;
            }

            let count = self.neighbor_counts[current] as usize;
            let neighbor_slice = &self.neighbors[current][..count];
            for &next in neighbor_slice {
                if blocked_idx != Some(next)
                    && self.walkable[next] != 0
                    && self.visited_stamp[next] != stamp
                {
                    self.visited_stamp[next] = stamp;
                    self.queue.push(next);
                    stat!(self, player_nodes_pushed += 1);
                }
            }
        }

        false
    }

    #[inline]
    fn idx(&self, row: usize, col: usize) -> usize {
        row * self.width + col
    }
}
