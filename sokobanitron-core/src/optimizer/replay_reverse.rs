// Replay trace for reverse pull histories.
//
// This module is the legality oracle for candidate histories: proposed moves
// are re-run through pull pathfinding and realized paths/snapshots are emitted.
// The optimizer uses the trace to evaluate candidates; tests also use it to
// assert replay validity and state transitions.
#[cfg(test)]
use crate::optimizer::model::ReverseOptimizationInput;
use crate::optimizer::model::{BoxMovePath, Coord};
use crate::pathfinder::{Position, PullPathfinder};
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReplayStateSnapshot {
    pub boxes: Vec<Coord>,
    pub player: Option<Coord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReplayStepSnapshot {
    pub before: ReplayStateSnapshot,
    pub after: ReplayStateSnapshot,
    pub realized_path: BoxMovePath,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReplaySolutionTrace {
    pub steps: Vec<ReplayStepSnapshot>,
    pub final_state: ReplayStateSnapshot,
}

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

fn snapshot_state(boxes: &HashSet<Coord>, player: Option<Coord>) -> ReplayStateSnapshot {
    let mut sorted_boxes = boxes.iter().copied().collect::<Vec<_>>();
    sorted_boxes.sort_unstable();
    ReplayStateSnapshot {
        boxes: sorted_boxes,
        player,
    }
}

pub(crate) fn replay_reverse_solution_trace_from_state(
    walkable: &HashSet<Coord>,
    box_positions: &[Coord],
    player: Option<Coord>,
    proposed_paths: &[BoxMovePath],
) -> Option<ReplaySolutionTrace> {
    let mut boxes = box_positions.iter().copied().collect::<HashSet<_>>();
    let mut player = player;
    let mut steps = Vec::with_capacity(proposed_paths.len());

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

        let (grid, min_x, min_y) = build_grid(walkable, &boxes, from, to, player)?;
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

        let before = snapshot_state(&boxes, player);
        boxes.remove(&from);
        boxes.insert(to);
        player = Some(to_world_position(result.player_start, min_x, min_y));
        let after = snapshot_state(&boxes, player);
        steps.push(ReplayStepSnapshot {
            before,
            after,
            realized_path,
        });
    }

    Some(ReplaySolutionTrace {
        steps,
        final_state: snapshot_state(&boxes, player),
    })
}

#[cfg(test)]
pub(crate) fn replay_reverse_solution_trace(
    input: &ReverseOptimizationInput,
    proposed_paths: &[BoxMovePath],
) -> Option<ReplaySolutionTrace> {
    let walkable = input.walkable_cells.iter().copied().collect::<HashSet<_>>();
    replay_reverse_solution_trace_from_state(
        &walkable,
        &input.box_positions,
        input.player,
        proposed_paths,
    )
}

#[cfg(test)]
mod tests {
    use super::replay_reverse_solution_trace;
    use crate::optimizer::ReverseOptimizationInput;

    fn walkable_square(size: i32) -> Vec<(i32, i32)> {
        let mut walkable = Vec::new();
        for y in 0..size {
            for x in 0..size {
                walkable.push((x, y));
            }
        }
        walkable
    }

    #[test]
    fn trace_records_before_and_after_for_each_move() {
        let input = ReverseOptimizationInput {
            walkable_cells: walkable_square(5),
            box_positions: vec![(3, 2)],
            player: None,
        };
        let proposed = vec![vec![(3, 2), (1, 2)]];

        let trace = replay_reverse_solution_trace(&input, &proposed).expect("trace should replay");
        assert_eq!(trace.steps.len(), 1);
        let step = &trace.steps[0];
        assert_eq!(step.before.boxes, vec![(3, 2)]);
        assert_eq!(step.after.boxes, vec![(1, 2)]);
        assert_eq!(step.realized_path.first().copied(), Some((3, 2)));
        assert_eq!(step.realized_path.last().copied(), Some((1, 2)));
        assert_eq!(trace.final_state.boxes, vec![(1, 2)]);
    }

    #[test]
    fn trace_keeps_step_boundaries_in_order() {
        let input = ReverseOptimizationInput {
            walkable_cells: walkable_square(5),
            box_positions: vec![(3, 2)],
            player: None,
        };
        let proposed = vec![vec![(3, 2), (2, 2)], vec![(2, 2), (1, 2)]];

        let trace = replay_reverse_solution_trace(&input, &proposed).expect("trace should replay");
        assert_eq!(trace.steps.len(), 2);
        assert_eq!(trace.steps[0].before.boxes, vec![(3, 2)]);
        assert_eq!(trace.steps[0].after.boxes, vec![(2, 2)]);
        assert_eq!(trace.steps[1].before.boxes, trace.steps[0].after.boxes);
        assert_eq!(trace.steps[1].after.boxes, vec![(1, 2)]);
        assert_eq!(trace.final_state.boxes, vec![(1, 2)]);
    }
}
