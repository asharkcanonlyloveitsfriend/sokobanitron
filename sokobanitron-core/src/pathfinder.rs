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
    walkable: Vec<bool>,
    visited_stamp: Vec<u32>,
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

        let walkable = walkable_rows.into_iter().flatten().collect::<Vec<_>>();
        Self {
            width,
            height,
            visited_stamp: vec![0; width * height],
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

    #[inline]
    fn is_walkable(&self, row: usize, col: usize) -> bool {
        self.walkable[self.idx(row, col)]
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

        let mut queue = std::collections::VecDeque::new();
        queue.push_back(self.idx(from.row, from.col));
        self.stats.nodes_pushed += 1;

        let target_index = self.idx(to.row, to.col);

        while let Some(current) = queue.pop_front() {
            self.stats.nodes_expanded += 1;

            if current == target_index {
                return true;
            }

            let row = current / self.width;
            let col = current % self.width;
            let index = self.idx(row, col);

            if self.visited_stamp[index] == stamp {
                continue;
            }
            self.visited_stamp[index] = stamp;

            if row > 0 {
                let up_row = row - 1;
                let up = Position::new(up_row, col);
                let up_idx = self.idx(up_row, col);
                if blocked != Some(up)
                    && self.is_walkable(up_row, col)
                    && self.visited_stamp[up_idx] != stamp
                {
                    queue.push_back(up_idx);
                    self.stats.nodes_pushed += 1;
                }
            }

            if row + 1 < self.height {
                let down_row = row + 1;
                let down = Position::new(down_row, col);
                let down_idx = self.idx(down_row, col);
                if blocked != Some(down)
                    && self.is_walkable(down_row, col)
                    && self.visited_stamp[down_idx] != stamp
                {
                    queue.push_back(down_idx);
                    self.stats.nodes_pushed += 1;
                }
            }

            if col > 0 {
                let left_col = col - 1;
                let left = Position::new(row, left_col);
                let left_idx = self.idx(row, left_col);
                if blocked != Some(left)
                    && self.is_walkable(row, left_col)
                    && self.visited_stamp[left_idx] != stamp
                {
                    queue.push_back(left_idx);
                    self.stats.nodes_pushed += 1;
                }
            }

            if col + 1 < self.width {
                let right_col = col + 1;
                let right = Position::new(row, right_col);
                let right_idx = self.idx(row, right_col);
                if blocked != Some(right)
                    && self.is_walkable(row, right_col)
                    && self.visited_stamp[right_idx] != stamp
                {
                    queue.push_back(right_idx);
                    self.stats.nodes_pushed += 1;
                }
            }
        }

        false
    }
}
