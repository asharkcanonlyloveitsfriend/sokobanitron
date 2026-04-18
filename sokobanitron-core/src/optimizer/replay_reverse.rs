// Replay trace for reverse pull histories.
//
// This module is the legality oracle for candidate histories: proposed moves
// are re-run through pull pathfinding and realized paths/snapshots are emitted.
// The optimizer uses the trace to evaluate candidates; tests also use it to
// assert replay validity and state transitions.
#[cfg(test)]
use crate::optimizer::model::ReverseOptimizationInput;
use crate::optimizer::model::{BoxMovePath, Coord};
use crate::pathfinder::{PullPathfinder, WorldBounds, WorldGrid};
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

fn build_grid(
    walkable: &HashSet<Coord>,
    boxes: &HashSet<Coord>,
    from: Coord,
    to: Coord,
    player: Option<Coord>,
) -> Option<WorldGrid> {
    let mut bounds = WorldBounds::from_points(
        walkable
            .iter()
            .copied()
            .chain(boxes.iter().copied())
            .chain([from, to]),
    )?;
    if let Some(player) = player {
        bounds.include(player);
    }

    Some(WorldGrid::from_bounds(bounds, |pos| {
        let is_walkable = walkable.contains(&pos);
        let blocked_by_other_box = boxes.contains(&pos) && pos != from;
        is_walkable && !blocked_by_other_box
    }))
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

        let grid = build_grid(walkable, &boxes, from, to, player)?;
        let world_origin = grid.origin();
        let box_start = grid.to_grid_position(from)?;
        let origin = grid.to_grid_position(to)?;
        let player_start = player.and_then(|coord| grid.to_grid_position(coord));

        let mut pathfinder = PullPathfinder::new(grid.into_rows(), box_start, player_start);
        let result = pathfinder.find_pull_path(origin)?;
        let realized_path = result
            .box_path
            .into_iter()
            .map(|pos| world_origin.to_world_position(pos))
            .collect::<Vec<_>>();

        let before = snapshot_state(&boxes, player);
        boxes.remove(&from);
        boxes.insert(to);
        player = Some(world_origin.to_world_position(result.player_start));
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

    #[test]
    fn trace_replays_negative_world_coordinates_with_extended_pull_target() {
        let input = ReverseOptimizationInput {
            walkable_cells: (-3..=1).map(|x| (x, -1)).collect(),
            box_positions: vec![(1, -1)],
            player: Some((0, -1)),
        };
        let proposed = vec![vec![(1, -1), (-2, -1)]];

        let trace = replay_reverse_solution_trace(&input, &proposed).expect("trace should replay");

        assert_eq!(trace.steps.len(), 1);
        assert_eq!(
            trace.steps[0].realized_path,
            vec![(1, -1), (0, -1), (-1, -1), (-2, -1)]
        );
        assert_eq!(trace.steps[0].after.boxes, vec![(-2, -1)]);
        assert_eq!(trace.steps[0].after.player, Some((-3, -1)));
        assert_eq!(trace.final_state.boxes, vec![(-2, -1)]);
        assert_eq!(trace.final_state.player, Some((-3, -1)));
    }
}
