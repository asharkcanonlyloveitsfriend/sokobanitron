use crate::optimizer::model::{BoxMovePath, Coord, ReverseOptimizationInput};
use crate::pathfinder::{Position, PullPathfinder};
use std::collections::HashSet;

#[inline]
fn to_grid_position(coord: Coord, min_x: i32, min_y: i32) -> Option<Position> {
    let col = coord.0.checked_sub(min_x)?;
    let row = coord.1.checked_sub(min_y)?;
    Some(Position::new(row as usize, col as usize))
}

#[inline]
fn to_world_position(grid: Position, min_x: i32, min_y: i32) -> Coord {
    (min_x + grid.col as i32, min_y + grid.row as i32)
}

fn planning_bounds(
    walkable: &HashSet<Coord>,
    boxes: &HashSet<Coord>,
    from: Coord,
    to: Coord,
    player: Option<Coord>,
) -> Option<(i32, i32, i32, i32)> {
    let mut iter = walkable.iter().copied();
    let first = iter
        .next()
        .or_else(|| boxes.iter().copied().next())
        .or(player)?;
    let mut min_x = first.0.min(from.0).min(to.0);
    let mut max_x = first.0.max(from.0).max(to.0);
    let mut min_y = first.1.min(from.1).min(to.1);
    let mut max_y = first.1.max(from.1).max(to.1);

    for (x, y) in iter {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    for (x, y) in boxes {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }
    if let Some((x, y)) = player {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    Some((min_x, max_x, min_y, max_y))
}

fn build_grid(
    walkable: &HashSet<Coord>,
    boxes: &HashSet<Coord>,
    from: Coord,
    to: Coord,
    player: Option<Coord>,
) -> Option<(Vec<Vec<bool>>, i32, i32)> {
    let (min_x, max_x, min_y, max_y) = planning_bounds(walkable, boxes, from, to, player)?;
    let width = (max_x - min_x + 1) as usize;
    let height = (max_y - min_y + 1) as usize;
    if width == 0 || height == 0 {
        return None;
    }

    let mut grid = vec![vec![false; width]; height];
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let pos = (x, y);
            let idx_y = (y - min_y) as usize;
            let idx_x = (x - min_x) as usize;
            let is_walkable = walkable.contains(&pos);
            let blocked_by_other_box = boxes.contains(&pos) && pos != from;
            grid[idx_y][idx_x] = is_walkable && !blocked_by_other_box;
        }
    }

    Some((grid, min_x, min_y))
}

pub(crate) fn replay_reverse_solution(
    input: &ReverseOptimizationInput,
    proposed_paths: &[BoxMovePath],
) -> Option<Vec<BoxMovePath>> {
    let walkable = input.walkable_cells.iter().copied().collect::<HashSet<_>>();
    let mut boxes = input.box_positions.iter().copied().collect::<HashSet<_>>();
    let mut player = input.player;
    let mut realized = Vec::with_capacity(proposed_paths.len());

    for proposed in proposed_paths {
        let from = proposed.first().copied()?;
        let to = proposed.last().copied()?;
        if from == to {
            continue;
        }
        if !boxes.contains(&from) {
            return None;
        }
        if !walkable.contains(&to) {
            return None;
        }
        if boxes.contains(&to) && to != from {
            return None;
        }

        let (grid, min_x, min_y) = build_grid(&walkable, &boxes, from, to, player)?;
        let box_start = to_grid_position(from, min_x, min_y)?;
        let origin = to_grid_position(to, min_x, min_y)?;
        let player_start = player.and_then(|coord| to_grid_position(coord, min_x, min_y));

        let mut pathfinder = PullPathfinder::new(grid, box_start, player_start);
        let result = pathfinder.find_pull_path(origin)?;
        let realized_path = result
            .box_path
            .into_iter()
            .map(|pos| to_world_position(pos, min_x, min_y))
            .collect::<Vec<_>>();

        boxes.remove(&from);
        boxes.insert(to);
        player = Some(to_world_position(result.player_start, min_x, min_y));
        realized.push(realized_path);
    }

    Some(realized)
}
