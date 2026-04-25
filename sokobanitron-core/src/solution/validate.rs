use crate::pathfinder::Position;

pub type SolutionPath = Vec<Position>;
pub type Solution = Vec<SolutionPath>;
pub type IndexedSolutionPath = Vec<usize>;
pub type IndexedSolution = Vec<IndexedSolutionPath>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValidationError {
    IllegalMove,
    Unsolved,
}

#[derive(Clone, Debug)]
pub struct PreparedPuzzle {
    width: usize,
    height: usize,
    walkable: Vec<u8>,
    goals: Vec<u8>,
    initial_boxes: Vec<u8>,
    neighbors: Vec<[usize; 4]>,
    player: usize,
}

#[derive(Clone, Debug)]
pub struct ValidationScratch {
    boxes: Vec<u8>,
    seen: Vec<u32>,
    queue: Vec<usize>,
    stamp: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

const NO_NEIGHBOR: usize = usize::MAX;

impl PreparedPuzzle {
    pub fn from_normalized_lines(lines: Vec<String>) -> Self {
        assert!(!lines.is_empty(), "normalized grid must not be empty");

        let height = lines.len();
        let width = lines[0].len();
        assert!(width > 0, "normalized grid must not have empty rows");
        let cell_count = width * height;
        let mut walkable = vec![0; cell_count];
        let mut goals = vec![0; cell_count];
        let mut initial_boxes = vec![0; cell_count];
        let mut player = None;

        for (row, line) in lines.iter().enumerate() {
            assert_eq!(
                line.len(),
                width,
                "normalized grid row {row} must have width {width}"
            );

            for (col, ch) in line.bytes().enumerate() {
                let idx = row * width + col;
                match ch {
                    b'#' => {}
                    b' ' => {
                        walkable[idx] = 1;
                    }
                    b'.' => {
                        walkable[idx] = 1;
                        goals[idx] = 1;
                    }
                    b'$' => {
                        walkable[idx] = 1;
                        initial_boxes[idx] = 1;
                    }
                    b'*' => {
                        walkable[idx] = 1;
                        goals[idx] = 1;
                        initial_boxes[idx] = 1;
                    }
                    b'@' => {
                        walkable[idx] = 1;
                        set_player(&mut player, idx);
                    }
                    b'+' => {
                        walkable[idx] = 1;
                        goals[idx] = 1;
                        set_player(&mut player, idx);
                    }
                    _ => {
                        panic!("normalized grid contains invalid byte {ch} at row {row}, col {col}")
                    }
                }
            }
        }

        Self {
            width,
            height,
            walkable,
            goals,
            initial_boxes,
            neighbors: build_neighbors(width, height),
            player: player.expect("normalized grid must contain one player start"),
        }
    }

    #[inline]
    pub fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub fn height(&self) -> usize {
        self.height
    }

    #[inline]
    pub fn cell_count(&self) -> usize {
        self.walkable.len()
    }

    pub fn scratch(&self) -> ValidationScratch {
        ValidationScratch::new(self.cell_count())
    }

    pub fn validate_indexed_solution(
        &self,
        solution: &[IndexedSolutionPath],
    ) -> Result<(), ValidationError> {
        let mut scratch = self.scratch();
        self.validate_indexed_solution_with_scratch(solution, &mut scratch)
    }

    pub fn validate_indexed_solution_with_scratch(
        &self,
        solution: &[IndexedSolutionPath],
        scratch: &mut ValidationScratch,
    ) -> Result<(), ValidationError> {
        scratch.reset_boxes(&self.initial_boxes);
        let mut player = self.player;

        for path in solution {
            self.validate_path(&mut player, path, scratch)?;
        }

        for idx in 0..self.cell_count() {
            if scratch.boxes[idx] != 0 && self.goals[idx] == 0 {
                return Err(ValidationError::Unsolved);
            }
        }

        Ok(())
    }

    fn validate_path(
        &self,
        player: &mut usize,
        path: &IndexedSolutionPath,
        scratch: &mut ValidationScratch,
    ) -> Result<(), ValidationError> {
        if path.len() < 2 {
            return Err(ValidationError::IllegalMove);
        }

        let mut from = path[0];
        if from >= self.cell_count() {
            return Err(ValidationError::IllegalMove);
        }
        if scratch.boxes[from] == 0 {
            return Err(ValidationError::IllegalMove);
        }

        let mut previous_direction = None;
        for &to in &path[1..] {
            let direction = self
                .step_direction(from, to)
                .ok_or(ValidationError::IllegalMove)?;

            if self.walkable[to] == 0 || scratch.boxes[to] != 0 {
                return Err(ValidationError::IllegalMove);
            }

            if previous_direction != Some(direction) {
                let stand = self
                    .player_stand_position(from, direction)
                    .ok_or(ValidationError::IllegalMove)?;
                if !self.can_player_reach(*player, stand, scratch) {
                    return Err(ValidationError::IllegalMove);
                }
            }

            scratch.boxes[from] = 0;
            scratch.boxes[to] = 1;
            *player = from;
            previous_direction = Some(direction);
            from = to;
        }

        Ok(())
    }

    fn can_player_reach(
        &self,
        start: usize,
        target: usize,
        scratch: &mut ValidationScratch,
    ) -> bool {
        if start == target {
            return true;
        }
        if !self.is_open_for_player(&scratch.boxes, target) {
            return false;
        }

        let stamp = scratch.next_stamp();
        scratch.queue.clear();
        scratch.seen[start] = stamp;
        scratch.queue.push(start);

        let mut head = 0;
        while head < scratch.queue.len() {
            let current = scratch.queue[head];
            head += 1;

            for &next in &self.neighbors[current] {
                if next != NO_NEIGHBOR && self.try_reach_neighbor(next, target, stamp, scratch) {
                    return true;
                }
            }
        }

        false
    }

    fn try_reach_neighbor(
        &self,
        next: usize,
        target: usize,
        stamp: u32,
        scratch: &mut ValidationScratch,
    ) -> bool {
        if scratch.seen[next] == stamp || !self.is_open_for_player(&scratch.boxes, next) {
            return false;
        }
        if next == target {
            return true;
        }
        scratch.seen[next] = stamp;
        scratch.queue.push(next);
        false
    }

    #[inline]
    fn is_open_for_player(&self, boxes: &[u8], idx: usize) -> bool {
        self.walkable[idx] != 0 && boxes[idx] == 0
    }

    fn step_direction(&self, from: usize, to: usize) -> Option<Direction> {
        if from >= self.cell_count() || to >= self.cell_count() {
            return None;
        }
        if to + self.width == from {
            Some(Direction::Up)
        } else if from + self.width == to {
            Some(Direction::Down)
        } else if !from.is_multiple_of(self.width) && from - 1 == to {
            Some(Direction::Left)
        } else if from % self.width + 1 < self.width && from + 1 == to {
            Some(Direction::Right)
        } else {
            None
        }
    }

    fn player_stand_position(&self, from: usize, direction: Direction) -> Option<usize> {
        match direction {
            Direction::Up => (from + self.width < self.cell_count()).then_some(from + self.width),
            Direction::Down => (from >= self.width).then_some(from - self.width),
            Direction::Left => (from % self.width + 1 < self.width).then_some(from + 1),
            Direction::Right => (!from.is_multiple_of(self.width)).then_some(from - 1),
        }
    }

    fn position_idx(&self, position: Position) -> Option<usize> {
        (position.row < self.height && position.col < self.width)
            .then_some(position.row * self.width + position.col)
    }
}

impl ValidationScratch {
    fn new(cell_count: usize) -> Self {
        Self {
            boxes: vec![0; cell_count],
            seen: vec![0; cell_count],
            queue: Vec::with_capacity(cell_count),
            stamp: 0,
        }
    }

    fn reset_boxes(&mut self, initial_boxes: &[u8]) {
        self.boxes.clear();
        self.boxes.extend_from_slice(initial_boxes);
    }

    fn next_stamp(&mut self) -> u32 {
        self.stamp = self.stamp.wrapping_add(1);
        if self.stamp == 0 {
            self.seen.fill(0);
            self.stamp = 1;
        }
        self.stamp
    }
}

pub fn validate_indexed_solution_lines(
    lines: Vec<String>,
    solution: &[IndexedSolutionPath],
) -> Result<(), ValidationError> {
    PreparedPuzzle::from_normalized_lines(lines).validate_indexed_solution(solution)
}

pub fn validate_solution_lines(
    lines: Vec<String>,
    solution: &[SolutionPath],
) -> Result<(), ValidationError> {
    let puzzle = PreparedPuzzle::from_normalized_lines(lines);
    let mut indexed_solution = Vec::with_capacity(solution.len());

    for path in solution {
        let mut indexed_path = Vec::with_capacity(path.len());
        for &position in path {
            let idx = puzzle
                .position_idx(position)
                .ok_or(ValidationError::IllegalMove)?;
            indexed_path.push(idx);
        }
        indexed_solution.push(indexed_path);
    }

    puzzle.validate_indexed_solution(&indexed_solution)
}

fn set_player(player: &mut Option<usize>, idx: usize) {
    assert!(
        player.replace(idx).is_none(),
        "normalized grid must contain exactly one player start"
    );
}

fn build_neighbors(width: usize, height: usize) -> Vec<[usize; 4]> {
    let mut neighbors = vec![[NO_NEIGHBOR; 4]; width * height];

    for row in 0..height {
        for col in 0..width {
            let idx = row * width + col;
            if row > 0 {
                neighbors[idx][0] = idx - width;
            }
            if row + 1 < height {
                neighbors[idx][1] = idx + width;
            }
            if col > 0 {
                neighbors[idx][2] = idx - 1;
            }
            if col + 1 < width {
                neighbors[idx][3] = idx + 1;
            }
        }
    }

    neighbors
}

#[cfg(test)]
mod tests {
    use super::{ValidationError, validate_solution_lines};
    use crate::pathfinder::Position;

    fn lines(grid: &str) -> Vec<String> {
        grid.trim_matches('\n')
            .lines()
            .map(|line| line.to_string())
            .collect()
    }

    #[test]
    fn validates_straight_push_solution() {
        let grid = lines(
            "
######
#@ $.#
######
",
        );
        let solution = vec![vec![Position::new(1, 3), Position::new(1, 4)]];

        assert_eq!(validate_solution_lines(grid, &solution), Ok(()));
    }

    #[test]
    fn validates_corner_push_when_player_can_reposition() {
        let grid = lines(
            "
#####
#@  #
# $ #
#  .#
#####
",
        );
        let solution = vec![vec![
            Position::new(2, 2),
            Position::new(2, 3),
            Position::new(3, 3),
        ]];

        assert_eq!(validate_solution_lines(grid, &solution), Ok(()));
    }

    #[test]
    fn rejects_corner_push_when_player_cannot_reposition() {
        let grid = lines(
            "
#####
#@###
# $ #
###.#
#####
",
        );
        let solution = vec![vec![
            Position::new(2, 2),
            Position::new(2, 3),
            Position::new(3, 3),
        ]];

        assert!(matches!(
            validate_solution_lines(grid, &solution),
            Err(ValidationError::IllegalMove)
        ));
    }

    #[test]
    fn rejects_solution_that_leaves_box_off_goal() {
        let grid = lines(
            "
#######
#@ $ .#
#######
",
        );
        let solution = vec![vec![Position::new(1, 3), Position::new(1, 4)]];

        assert_eq!(
            validate_solution_lines(grid, &solution),
            Err(ValidationError::Unsolved)
        );
    }
}
