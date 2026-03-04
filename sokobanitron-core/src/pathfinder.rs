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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PathfinderStats {
    pub nodes_expanded: u64,
    pub nodes_pushed: u64,
}

#[derive(Debug, Clone)]
pub struct Pathfinder {
    width: usize,
    height: usize,
    walkable: Vec<u8>,
    visited_stamp: Vec<u32>,
    queue: Vec<usize>,
    current_stamp: u32,
    stats: PathfinderStats,
}

impl Pathfinder {
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
        Self {
            width,
            height,
            visited_stamp: vec![0; width * height],
            queue: Vec::with_capacity(width * height),
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
    pub fn reset_stats(&mut self) {
        self.stats = PathfinderStats::default();
    }

    #[inline]
    fn idx(&self, row: usize, col: usize) -> usize {
        row * self.width + col
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
        self.stats.nodes_pushed += 1;
        let mut head = 0;

        while head < self.queue.len() {
            let current = self.queue[head];
            head += 1;
            self.stats.nodes_expanded += 1;

            if current == target_index {
                return true;
            }

            let row = current / self.width;
            let col = current % self.width;

            if row > 0 {
                let up_row = row - 1;
                let up_idx = self.idx(up_row, col);
                if blocked_idx != Some(up_idx)
                    && self.walkable[up_idx] != 0
                    && self.visited_stamp[up_idx] != stamp
                {
                    self.visited_stamp[up_idx] = stamp;
                    self.queue.push(up_idx);
                    self.stats.nodes_pushed += 1;
                }
            }

            if row + 1 < self.height {
                let down_row = row + 1;
                let down_idx = self.idx(down_row, col);
                if blocked_idx != Some(down_idx)
                    && self.walkable[down_idx] != 0
                    && self.visited_stamp[down_idx] != stamp
                {
                    self.visited_stamp[down_idx] = stamp;
                    self.queue.push(down_idx);
                    self.stats.nodes_pushed += 1;
                }
            }

            if col > 0 {
                let left_col = col - 1;
                let left_idx = self.idx(row, left_col);
                if blocked_idx != Some(left_idx)
                    && self.walkable[left_idx] != 0
                    && self.visited_stamp[left_idx] != stamp
                {
                    self.visited_stamp[left_idx] = stamp;
                    self.queue.push(left_idx);
                    self.stats.nodes_pushed += 1;
                }
            }

            if col + 1 < self.width {
                let right_col = col + 1;
                let right_idx = self.idx(row, right_col);
                if blocked_idx != Some(right_idx)
                    && self.walkable[right_idx] != 0
                    && self.visited_stamp[right_idx] != stamp
                {
                    self.visited_stamp[right_idx] = stamp;
                    self.queue.push(right_idx);
                    self.stats.nodes_pushed += 1;
                }
            }
        }

        false
    }
}
