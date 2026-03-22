mod model;
mod passes;
mod replay_reverse;

pub use model::{BoxMovePath, Coord, ReverseOptimizationInput};

use passes::{
    DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE, DEFAULT_MAX_REWRITE_PROPOSALS,
    DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL, DEFAULT_MAX_REWRITE_PROPOSALS_PER_WINDOW,
    apply_rewrite_plan, for_each_k_minus_one_rewrite_plan, normalize_paths_in_place,
    optimize_adjacent_merge_in_place,
};
use replay_reverse::replay_reverse_solution_trace_from_state;
use std::collections::HashSet;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SolutionScore {
    discrete_moves: usize,
    total_path_steps: usize,
}

/// Debug/profiling counters for reverse-optimizer runs.
///
/// This stays public intentionally so clients (like the level creator) can
/// inspect optimizer cost while keeping optimization behavior identical.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReverseOptimizationStats {
    pub iterations: usize,
    pub rewrite_plan_count: usize,
    pub rewrite_plan_generation_time: Duration,
    pub replay_count: usize,
    pub replay_time: Duration,
    pub total_time: Duration,
}

fn score(paths: &[BoxMovePath]) -> SolutionScore {
    SolutionScore {
        discrete_moves: paths.len(),
        total_path_steps: paths
            .iter()
            .map(|path| path.len().saturating_sub(1))
            .sum::<usize>(),
    }
}

// Cheap invariant helper used to compare terminal occupied box sets.
// Candidate legality and realized paths still come from replay tracing.
fn final_box_positions(start_boxes: &[Coord], paths: &[BoxMovePath]) -> Option<HashSet<Coord>> {
    let mut boxes = start_boxes.iter().copied().collect::<HashSet<_>>();
    for path in paths {
        let from = path.first().copied()?;
        let to = path.last().copied()?;
        if from == to {
            continue;
        }
        if !boxes.remove(&from) {
            return None;
        }
        if boxes.contains(&to) {
            return None;
        }
        boxes.insert(to);
    }
    Some(boxes)
}

pub fn optimize_box_move_paths_in_place(paths: &mut Vec<BoxMovePath>) {
    optimize_adjacent_merge_in_place(paths);
}

fn evaluate_candidate_paths(
    walkable: &HashSet<Coord>,
    prefix_paths: &[BoxMovePath],
    state_boxes: &[Coord],
    state_player: Option<Coord>,
    suffix_paths: &[BoxMovePath],
    current_score: SolutionScore,
    target_final_boxes: &HashSet<Coord>,
    stats: &mut ReverseOptimizationStats,
) -> Option<(Vec<BoxMovePath>, SolutionScore)> {
    stats.replay_count += 1;
    let replay_started = Instant::now();
    let replay_trace =
        replay_reverse_solution_trace_from_state(walkable, state_boxes, state_player, suffix_paths);
    stats.replay_time += replay_started.elapsed();
    let replay_trace = replay_trace?;

    let mut realized = Vec::with_capacity(prefix_paths.len() + replay_trace.steps.len());
    realized.extend(prefix_paths.iter().cloned());
    realized.extend(
        replay_trace
            .steps
            .into_iter()
            .map(|step| step.realized_path),
    );
    normalize_paths_in_place(&mut realized);
    optimize_adjacent_merge_in_place(&mut realized);

    let candidate_final_boxes = replay_trace
        .final_state
        .boxes
        .into_iter()
        .collect::<HashSet<_>>();
    if &candidate_final_boxes != target_final_boxes {
        return None;
    }

    let candidate_score = score(&realized);
    (candidate_score < current_score).then_some((realized, candidate_score))
}

fn optimize_reverse_solution_in_place_internal(
    input: &ReverseOptimizationInput,
    paths: &mut Vec<BoxMovePath>,
    stats: Option<&mut ReverseOptimizationStats>,
) -> bool {
    let started = Instant::now();
    let mut local_stats = ReverseOptimizationStats::default();
    let before = paths.clone();
    normalize_paths_in_place(paths);
    optimize_adjacent_merge_in_place(paths);
    let Some(target_final_boxes) = final_box_positions(&input.box_positions, paths) else {
        return false;
    };
    let walkable = input.walkable_cells.iter().copied().collect::<HashSet<_>>();
    let mut sorted_start_boxes = input.box_positions.clone();
    sorted_start_boxes.sort_unstable();

    loop {
        local_stats.iterations += 1;
        let current_score = score(paths);
        let mut best_candidate: Option<(Vec<BoxMovePath>, SolutionScore)> = None;
        let Some(base_trace) = replay_reverse_solution_trace_from_state(
            &walkable,
            &input.box_positions,
            input.player,
            paths,
        ) else {
            return false;
        };
        if base_trace.steps.len() != paths.len() {
            return false;
        }

        let plan_stats = for_each_k_minus_one_rewrite_plan(
            paths,
            &input.walkable_cells,
            DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE,
            DEFAULT_MAX_REWRITE_PROPOSALS,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_WINDOW,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL,
            |plan| {
                let Some(rewritten) = apply_rewrite_plan(paths, plan) else {
                    return true;
                };
                let (prefix_boxes, prefix_player) = if plan.window_start == 0 {
                    (sorted_start_boxes.as_slice(), input.player)
                } else {
                    let after = &base_trace.steps[plan.window_start - 1].after;
                    (after.boxes.as_slice(), after.player)
                };
                let Some((realized, candidate_score)) = evaluate_candidate_paths(
                    &walkable,
                    &paths[..plan.window_start],
                    prefix_boxes,
                    prefix_player,
                    &rewritten[plan.window_start..],
                    current_score,
                    &target_final_boxes,
                    &mut local_stats,
                ) else {
                    return true;
                };
                let should_replace = match &best_candidate {
                    None => true,
                    Some((_, best_score)) => candidate_score < *best_score,
                };
                if should_replace {
                    best_candidate = Some((realized, candidate_score));
                }
                true
            },
        );
        local_stats.rewrite_plan_count += plan_stats.emitted_plans;
        local_stats.rewrite_plan_generation_time += plan_stats.generation_time();

        if let Some((candidate, _)) = best_candidate {
            *paths = candidate;
        } else {
            break;
        }
    }

    local_stats.total_time = started.elapsed();
    if let Some(stats) = stats {
        *stats = local_stats;
    }
    *paths != before
}

pub fn optimize_reverse_solution_in_place(
    input: &ReverseOptimizationInput,
    paths: &mut Vec<BoxMovePath>,
) -> bool {
    optimize_reverse_solution_in_place_internal(input, paths, None)
}

/// Same optimizer as `optimize_reverse_solution_in_place`, plus profiling stats.
pub fn optimize_reverse_solution_in_place_with_stats(
    input: &ReverseOptimizationInput,
    paths: &mut Vec<BoxMovePath>,
) -> (bool, ReverseOptimizationStats) {
    let mut stats = ReverseOptimizationStats::default();
    let changed = optimize_reverse_solution_in_place_internal(input, paths, Some(&mut stats));
    (changed, stats)
}

#[cfg(test)]
mod tests {
    use super::{
        ReverseOptimizationInput, final_box_positions, optimize_box_move_paths_in_place,
        optimize_reverse_solution_in_place,
    };
    use crate::optimizer::passes::{
        DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE, DEFAULT_MAX_REWRITE_PROPOSALS,
        DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL, DEFAULT_MAX_REWRITE_PROPOSALS_PER_WINDOW,
        RewritePlan, apply_rewrite_plan, generate_k_minus_one_rewrite_plans,
    };
    use crate::optimizer::replay_reverse::replay_reverse_solution_trace;

    fn score(paths: &[Vec<(i32, i32)>]) -> (usize, usize) {
        (
            paths.len(),
            paths
                .iter()
                .map(|path| path.len().saturating_sub(1))
                .sum::<usize>(),
        )
    }

    fn proposals(input: &ReverseOptimizationInput, paths: &[Vec<(i32, i32)>]) -> Vec<RewritePlan> {
        generate_k_minus_one_rewrite_plans(
            paths,
            &input.walkable_cells,
            DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE,
            DEFAULT_MAX_REWRITE_PROPOSALS,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_WINDOW,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL,
        )
    }

    #[test]
    fn merges_contiguous_adjacent_paths() {
        let mut paths = vec![vec![(0, 0), (1, 0)], vec![(1, 0), (2, 0)]];
        optimize_box_move_paths_in_place(&mut paths);
        assert_eq!(paths, vec![vec![(0, 0), (1, 0), (2, 0)]]);
    }

    #[test]
    fn three_to_two_swap_reduces_triplet_solution() {
        let mut walkable_cells = Vec::new();
        for y in 0..=6 {
            for x in 0..=6 {
                walkable_cells.push((x, y));
            }
        }
        let input = ReverseOptimizationInput {
            walkable_cells,
            box_positions: vec![(2, 2), (4, 2)],
            player: Some((3, 3)),
        };
        let mut paths = vec![
            vec![(2, 2), (2, 3)],
            vec![(4, 2), (4, 3)],
            vec![(2, 3), (2, 4)],
        ];

        let changed = optimize_reverse_solution_in_place(&input, &mut paths);
        assert!(changed);
        assert_eq!(paths.len(), 2);
        let mut starts = paths
            .iter()
            .filter_map(|path| path.first().copied())
            .collect::<Vec<_>>();
        starts.sort_unstable();
        assert_eq!(starts, vec![(2, 2), (4, 2)]);

        let mut ends = paths
            .iter()
            .filter_map(|path| path.last().copied())
            .collect::<Vec<_>>();
        ends.sort_unstable();
        assert_eq!(ends, vec![(2, 4), (4, 3)]);
    }

    #[test]
    fn proposal_collection_includes_destination_swaps() {
        let paths = vec![
            vec![(0, 0), (1, 0)],
            vec![(5, 0), (4, 0)],
            vec![(1, 0), (3, 0)],
        ];
        let expected = vec![vec![(0, 0), (4, 0)], vec![(5, 0), (3, 0)]];

        let input = ReverseOptimizationInput {
            walkable_cells: vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0)],
            box_positions: vec![(0, 0), (5, 0)],
            player: None,
        };
        let collected = proposals(&input, &paths);
        assert!(
            collected.iter().any(|plan| {
                apply_rewrite_plan(&paths, plan)
                    .as_ref()
                    .is_some_and(|proposal| *proposal == expected)
            }),
            "destination swap proposal was not included in optimizer proposal set"
        );
    }

    #[test]
    fn proposal_collection_includes_invented_destinations() {
        let input = ReverseOptimizationInput {
            walkable_cells: vec![(0, 0), (1, 0), (2, 0), (3, 0), (1, 1), (2, 1)],
            box_positions: vec![(0, 0), (3, 0)],
            player: None,
        };
        let paths = vec![
            vec![(0, 0), (1, 0)],
            vec![(3, 0), (2, 0)],
            vec![(1, 0), (1, 1)],
        ];

        let collected = proposals(&input, &paths);
        assert!(
            collected.iter().any(|plan| {
                apply_rewrite_plan(&paths, plan)
                    .as_ref()
                    .is_some_and(|proposal| {
                        proposal
                            .iter()
                            .any(|path| path.last().copied() == Some((1, 1)))
                    })
            }),
            "invented destination proposal was not included in optimizer proposal set"
        );
    }

    #[test]
    fn optimizer_preserves_terminal_box_positions() {
        let mut walkable_cells = Vec::new();
        for y in 0..=6 {
            for x in 0..=6 {
                walkable_cells.push((x, y));
            }
        }
        let input = ReverseOptimizationInput {
            walkable_cells,
            box_positions: vec![(2, 2), (4, 2)],
            player: Some((3, 3)),
        };
        let mut paths = vec![
            vec![(2, 2), (2, 3)],
            vec![(4, 2), (4, 3)],
            vec![(2, 3), (2, 4)],
            vec![(4, 3), (4, 4)],
            vec![(2, 4), (2, 5)],
            vec![(4, 4), (4, 5)],
        ];
        let expected_final =
            final_box_positions(&input.box_positions, &paths).expect("valid baseline final boxes");

        optimize_reverse_solution_in_place(&input, &mut paths);

        let actual_final =
            final_box_positions(&input.box_positions, &paths).expect("valid optimized final boxes");
        assert_eq!(actual_final, expected_final);
    }

    #[test]
    fn rewrite_only_optimizer_preserves_validity_and_monotonic_score() {
        let mut walkable_cells = Vec::new();
        for y in 0..=8 {
            for x in 0..=8 {
                walkable_cells.push((x, y));
            }
        }
        let input = ReverseOptimizationInput {
            walkable_cells,
            box_positions: vec![(2, 2), (4, 2), (6, 2)],
            player: Some((4, 4)),
        };
        let mut paths = vec![
            vec![(2, 2), (2, 3)],
            vec![(4, 2), (4, 3)],
            vec![(6, 2), (6, 3)],
            vec![(2, 3), (2, 4)],
            vec![(4, 3), (4, 4)],
            vec![(6, 3), (6, 4)],
        ];
        let before_score = score(&paths);
        let expected_final =
            final_box_positions(&input.box_positions, &paths).expect("valid baseline final boxes");

        optimize_reverse_solution_in_place(&input, &mut paths);
        let after_score = score(&paths);

        assert!(
            after_score <= before_score,
            "optimizer score must not get worse"
        );
        let actual_final =
            final_box_positions(&input.box_positions, &paths).expect("valid optimized final boxes");
        assert_eq!(actual_final, expected_final);
        assert!(
            replay_reverse_solution_trace(&input, &paths).is_some(),
            "optimized history must replay successfully"
        );

        let mut second_pass = paths.clone();
        optimize_reverse_solution_in_place(&input, &mut second_pass);
        let second_score = score(&second_pass);
        assert!(
            second_score <= after_score,
            "second optimizer pass must not increase score"
        );
        assert!(
            replay_reverse_solution_trace(&input, &second_pass).is_some(),
            "second-pass history must replay successfully"
        );
    }
}
