// Rewrite-plan generation for the reverse optimizer.
//
// Strategy:
// - contiguous windows only
// - bounded k -> k - 1 proposal generation (k in WINDOW_SIZES)
// - endpoint hypotheses are speculative and cheap
// - proposals are replay-validated by the caller
// - caller keeps only strict lexicographic score improvements
use crate::optimizer::model::{BoxMovePath, Coord};
use std::collections::HashSet;
use std::time::{Duration, Instant};

pub(crate) const DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE: usize = 3;
pub(crate) const DEFAULT_MAX_REWRITE_PROPOSALS: usize = 160;
pub(crate) const DEFAULT_MAX_REWRITE_PROPOSALS_PER_WINDOW: usize = 64;
pub(crate) const DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL: usize = 24;

const DEFAULT_INVENTED_SEARCH_RADIUS: i32 = 2;
const WINDOW_SIZES: [usize; 3] = [3, 4, 5];
const MAX_WINDOW_SIZE: usize = 5;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RewritePlan {
    pub(crate) window_start: usize,
    pub(crate) window_size: usize,
    pub(crate) removed_index: usize,
    pub(crate) replacements: Vec<(usize, Coord)>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RewritePlanEnumerationStats {
    pub(crate) emitted_plans: usize,
    pub(crate) total_time: Duration,
    pub(crate) emit_callback_time: Duration,
}

impl RewritePlanEnumerationStats {
    pub(crate) fn generation_time(self) -> Duration {
        self.total_time.saturating_sub(self.emit_callback_time)
    }
}

#[inline]
fn path_start(path: &BoxMovePath) -> Option<Coord> {
    path.first().copied()
}

#[inline]
fn path_end(path: &BoxMovePath) -> Option<Coord> {
    path.last().copied()
}

#[inline]
fn manhattan(a: Coord, b: Coord) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

pub(crate) fn normalize_paths_in_place(paths: &mut Vec<BoxMovePath>) {
    paths.retain(|path| path.len() >= 2 && path_start(path) != path_end(path));
}

pub(crate) fn optimize_adjacent_merge_in_place(paths: &mut Vec<BoxMovePath>) {
    normalize_paths_in_place(paths);
    if paths.len() < 2 {
        return;
    }

    let mut i = 0usize;
    while i + 1 < paths.len() {
        if path_end(&paths[i]) == path_start(&paths[i + 1]) {
            let mut next = paths.remove(i + 1);
            paths[i].extend(next.drain(1..));
            i = i.saturating_sub(1);
        } else {
            i += 1;
        }
    }
}

fn insert_ranked_destination(
    ranked: &mut Vec<(i32, i32, i32, Coord)>,
    candidate: (i32, i32, i32, Coord),
    max: usize,
) {
    let insert_at = ranked
        .iter()
        .position(|existing| candidate < *existing)
        .unwrap_or(ranked.len());
    if insert_at >= max {
        return;
    }
    ranked.insert(insert_at, candidate);
    if ranked.len() > max {
        ranked.pop();
    }
}

fn invented_destinations_for_removal(
    paths: &[BoxMovePath],
    walkable_cells: &[Coord],
    window_endpoints: &[(usize, Coord)],
    removed_index: usize,
    max_invented_destinations_per_move: usize,
) -> Vec<(usize, Vec<Coord>)> {
    if max_invented_destinations_per_move == 0 || walkable_cells.is_empty() {
        return Vec::new();
    }

    let removed_end = window_endpoints
        .iter()
        .find_map(|(move_index, end)| (*move_index == removed_index).then_some(*end));

    let survivors = window_endpoints
        .iter()
        .copied()
        .filter(|(move_index, _)| *move_index != removed_index)
        .filter_map(|(move_index, end)| {
            path_start(&paths[move_index]).map(|start| (move_index, start, end))
        })
        .collect::<Vec<_>>();
    if survivors.is_empty() {
        return Vec::new();
    }

    let other_window_ends_by_survivor = survivors
        .iter()
        .map(|(move_index, _, _)| {
            window_endpoints
                .iter()
                .filter_map(|(other_idx, other_end)| {
                    (*other_idx != *move_index).then_some(*other_end)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut rankings = survivors
        .iter()
        .map(|_| Vec::with_capacity(max_invented_destinations_per_move))
        .collect::<Vec<_>>();

    for &coord in walkable_cells {
        for (survivor_slot, (_, start, end)) in survivors.iter().enumerate() {
            if coord == *start || coord == *end {
                continue;
            }
            let dist_to_end = manhattan(coord, *end);
            let dist_to_removed =
                removed_end.map_or(i32::MAX / 4, |removed| manhattan(coord, removed));
            let dist_to_window = other_window_ends_by_survivor[survivor_slot]
                .iter()
                .map(|&anchor| manhattan(coord, anchor))
                .min()
                .unwrap_or(i32::MAX / 4);
            if dist_to_end > DEFAULT_INVENTED_SEARCH_RADIUS
                && dist_to_removed > DEFAULT_INVENTED_SEARCH_RADIUS
                && dist_to_window > DEFAULT_INVENTED_SEARCH_RADIUS
            {
                continue;
            }
            insert_ranked_destination(
                &mut rankings[survivor_slot],
                (
                    dist_to_end,
                    dist_to_removed.min(dist_to_window),
                    dist_to_window,
                    coord,
                ),
                max_invented_destinations_per_move,
            );
        }
    }

    survivors
        .into_iter()
        .zip(rankings)
        .map(|((move_index, _, _), ranked)| {
            let invented = ranked
                .into_iter()
                .map(|(_, _, _, coord)| coord)
                .collect::<Vec<_>>();
            (move_index, invented)
        })
        .collect()
}

fn build_endpoint_options(
    paths: &[BoxMovePath],
    move_index: usize,
    removed_index: usize,
    invented_destinations: &[Coord],
) -> Vec<Coord> {
    let mut options = Vec::new();
    let mut seen = HashSet::new();
    let Some(move_end) = path_end(&paths[move_index]) else {
        return options;
    };

    // 1) original endpoint
    options.push(move_end);
    seen.insert(move_end);

    // 2) same-box continuation endpoint
    if move_index + 1 < paths.len()
        && path_start(&paths[move_index + 1]) == Some(move_end)
        && let Some(continuation_end) = path_end(&paths[move_index + 1])
        && seen.insert(continuation_end)
    {
        options.push(continuation_end);
    }

    // 3) removed-move continuation endpoint
    if removed_index != move_index
        && path_start(&paths[removed_index]) == Some(move_end)
        && let Some(removed_end) = path_end(&paths[removed_index])
        && seen.insert(removed_end)
    {
        options.push(removed_end);
    }

    // 4) nearby invented endpoints
    for &invented in invented_destinations {
        if seen.insert(invented) {
            options.push(invented);
        }
    }

    options
}

fn build_rewrite_plan(
    window_start: usize,
    window_size: usize,
    removed_index: usize,
    endpoint_assignment: &[Option<Coord>],
) -> Option<RewritePlan> {
    let window_end = window_start + window_size;
    let mut replacements = Vec::with_capacity(window_size.saturating_sub(1));
    for move_index in window_start..window_end {
        if move_index == removed_index {
            continue;
        }
        replacements.push((move_index, endpoint_assignment[move_index]?));
    }

    Some(RewritePlan {
        window_start,
        window_size,
        removed_index,
        replacements,
    })
}

pub(crate) fn apply_rewrite_plan(
    paths: &[BoxMovePath],
    plan: &RewritePlan,
) -> Option<Vec<BoxMovePath>> {
    if plan.window_size == 0 {
        return None;
    }
    let window_end = plan.window_start.checked_add(plan.window_size)?;
    if window_end > paths.len()
        || plan.removed_index < plan.window_start
        || plan.removed_index >= window_end
        || plan.window_size > MAX_WINDOW_SIZE
        || plan.replacements.len() + 1 != plan.window_size
    {
        return None;
    }

    let mut replacement_lookup = [None; MAX_WINDOW_SIZE];
    for &(move_index, destination) in &plan.replacements {
        if move_index < plan.window_start
            || move_index >= window_end
            || move_index == plan.removed_index
        {
            return None;
        }
        let local = move_index - plan.window_start;
        if replacement_lookup[local].is_some() {
            return None;
        }
        replacement_lookup[local] = Some(destination);
    }

    let mut rewritten = Vec::with_capacity(paths.len().saturating_sub(1));
    for (idx, path) in paths.iter().enumerate() {
        if idx == plan.removed_index {
            continue;
        }
        if idx >= plan.window_start && idx < window_end {
            let local = idx - plan.window_start;
            let destination = replacement_lookup[local]?;
            let from = path_start(path)?;
            rewritten.push(vec![from, destination]);
        } else {
            rewritten.push(path.clone());
        }
    }
    Some(rewritten)
}

#[allow(clippy::too_many_arguments)]
fn enumerate_window_assignments<Emit>(
    window_start: usize,
    window_size: usize,
    removed_index: usize,
    options_by_move: &[(usize, Vec<Coord>)],
    depth: usize,
    endpoint_assignment: &mut [Option<Coord>],
    seen: &mut HashSet<RewritePlan>,
    generated_total: &mut usize,
    emit_plan: &mut Emit,
    emit_callback_time: &mut Duration,
    max_proposals_per_window: usize,
    max_proposals_per_removal: usize,
    max_proposals: usize,
    generated_for_window: &mut usize,
    generated_for_removal: &mut usize,
) -> bool
where
    Emit: FnMut(&RewritePlan) -> bool,
{
    if *generated_total >= max_proposals
        || *generated_for_window >= max_proposals_per_window
        || *generated_for_removal >= max_proposals_per_removal
    {
        return false;
    }

    if depth == options_by_move.len() {
        let Some(plan) = build_rewrite_plan(
            window_start,
            window_size,
            removed_index,
            endpoint_assignment,
        ) else {
            return true;
        };
        if seen.insert(plan.clone()) {
            *generated_total += 1;
            *generated_for_window += 1;
            *generated_for_removal += 1;
            let emit_started = Instant::now();
            let should_continue = emit_plan(&plan);
            *emit_callback_time += emit_started.elapsed();
            if !should_continue {
                return false;
            }
        }
        return true;
    }

    let (move_index, options) = &options_by_move[depth];
    for &destination in options {
        endpoint_assignment[*move_index] = Some(destination);
        if !enumerate_window_assignments(
            window_start,
            window_size,
            removed_index,
            options_by_move,
            depth + 1,
            endpoint_assignment,
            seen,
            generated_total,
            emit_plan,
            emit_callback_time,
            max_proposals_per_window,
            max_proposals_per_removal,
            max_proposals,
            generated_for_window,
            generated_for_removal,
        ) {
            endpoint_assignment[*move_index] = None;
            return false;
        }
        if *generated_total >= max_proposals
            || *generated_for_window >= max_proposals_per_window
            || *generated_for_removal >= max_proposals_per_removal
        {
            endpoint_assignment[*move_index] = None;
            return true;
        }
    }
    endpoint_assignment[*move_index] = None;
    true
}

fn choose_contiguous_windows(path_len: usize, window_size: usize) -> impl Iterator<Item = usize> {
    let last_start = path_len.saturating_sub(window_size);
    0..=last_start
}

pub(crate) fn for_each_k_minus_one_rewrite_plan<Emit>(
    paths: &[BoxMovePath],
    walkable_cells: &[Coord],
    max_invented_destinations_per_move: usize,
    max_proposals: usize,
    max_proposals_per_window: usize,
    max_proposals_per_removal: usize,
    mut emit: Emit,
) -> RewritePlanEnumerationStats
where
    Emit: FnMut(&RewritePlan) -> bool,
{
    let started = Instant::now();
    let mut emit_callback_time = Duration::ZERO;
    let mut seen = HashSet::new();
    if paths.len() < WINDOW_SIZES[0]
        || max_proposals == 0
        || max_proposals_per_window == 0
        || max_proposals_per_removal == 0
    {
        return RewritePlanEnumerationStats::default();
    }

    let mut generated_total = 0usize;
    let mut endpoint_assignment = vec![None; paths.len()];
    for window_size in WINDOW_SIZES {
        if paths.len() < window_size {
            continue;
        }
        for window_start in choose_contiguous_windows(paths.len(), window_size) {
            let window_end = window_start + window_size;
            let window_endpoints = (window_start..window_end)
                .filter_map(|move_index| path_end(&paths[move_index]).map(|end| (move_index, end)))
                .collect::<Vec<_>>();
            if window_endpoints.len() != window_size {
                continue;
            }
            let mut generated_for_window = 0usize;

            for removed_index in window_start..window_end {
                if generated_for_window >= max_proposals_per_window {
                    break;
                }
                let mut generated_for_removal = 0usize;
                let invented_destinations_by_move = invented_destinations_for_removal(
                    paths,
                    walkable_cells,
                    &window_endpoints,
                    removed_index,
                    max_invented_destinations_per_move,
                );
                let mut options_by_move = Vec::with_capacity(window_size.saturating_sub(1));
                let mut valid = true;
                for move_index in window_start..window_end {
                    if move_index == removed_index {
                        continue;
                    }
                    let invented_destinations = invented_destinations_by_move
                        .iter()
                        .find_map(|(index, invented)| {
                            (*index == move_index).then_some(invented.as_slice())
                        })
                        .unwrap_or(&[]);
                    let options = build_endpoint_options(
                        paths,
                        move_index,
                        removed_index,
                        invented_destinations,
                    );
                    if options.is_empty() {
                        valid = false;
                        break;
                    }
                    options_by_move.push((move_index, options));
                }
                if !valid {
                    continue;
                }

                if !enumerate_window_assignments(
                    window_start,
                    window_size,
                    removed_index,
                    &options_by_move,
                    0,
                    &mut endpoint_assignment,
                    &mut seen,
                    &mut generated_total,
                    &mut emit,
                    &mut emit_callback_time,
                    max_proposals_per_window,
                    max_proposals_per_removal,
                    max_proposals,
                    &mut generated_for_window,
                    &mut generated_for_removal,
                ) {
                    return RewritePlanEnumerationStats {
                        emitted_plans: generated_total,
                        total_time: started.elapsed(),
                        emit_callback_time,
                    };
                }
                if generated_total >= max_proposals {
                    return RewritePlanEnumerationStats {
                        emitted_plans: generated_total,
                        total_time: started.elapsed(),
                        emit_callback_time,
                    };
                }
            }
        }
    }

    RewritePlanEnumerationStats {
        emitted_plans: generated_total,
        total_time: started.elapsed(),
        emit_callback_time,
    }
}

#[cfg(test)]
pub(crate) fn generate_k_minus_one_rewrite_plans(
    paths: &[BoxMovePath],
    walkable_cells: &[Coord],
    max_invented_destinations_per_move: usize,
    max_proposals: usize,
    max_proposals_per_window: usize,
    max_proposals_per_removal: usize,
) -> Vec<RewritePlan> {
    let mut plans = Vec::new();
    let _ = for_each_k_minus_one_rewrite_plan(
        paths,
        walkable_cells,
        max_invented_destinations_per_move,
        max_proposals,
        max_proposals_per_window,
        max_proposals_per_removal,
        |plan| {
            plans.push(plan.clone());
            true
        },
    );
    plans
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE, DEFAULT_MAX_REWRITE_PROPOSALS,
        DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL, DEFAULT_MAX_REWRITE_PROPOSALS_PER_WINDOW,
        apply_rewrite_plan, generate_k_minus_one_rewrite_plans,
    };
    use std::collections::HashSet;

    #[test]
    fn generates_triplet_rewrite_with_removed_continuation() {
        let paths = vec![
            vec![(0, 0), (1, 0)],
            vec![(5, 0), (4, 0)],
            vec![(1, 0), (3, 0)],
        ];
        let walkable_cells = vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0)];
        let expected = vec![vec![(0, 0), (4, 0)], vec![(5, 0), (3, 0)]];

        let plans = generate_k_minus_one_rewrite_plans(
            &paths,
            &walkable_cells,
            DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE,
            DEFAULT_MAX_REWRITE_PROPOSALS,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_WINDOW,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL,
        );

        assert!(
            plans.iter().any(|plan| {
                apply_rewrite_plan(&paths, plan)
                    .as_ref()
                    .is_some_and(|proposal| *proposal == expected)
            }),
            "expected 3->2 rewrite not found"
        );
    }

    #[test]
    fn can_include_invented_destination_cells() {
        let paths = vec![
            vec![(0, 0), (1, 0)],
            vec![(3, 0), (2, 0)],
            vec![(6, 0), (5, 0)],
        ];
        let walkable_cells = vec![
            (0, 0),
            (1, 0),
            (2, 0),
            (3, 0),
            (4, 0),
            (5, 0),
            (6, 0),
            (1, 1),
            (2, 1),
        ];

        let plans = generate_k_minus_one_rewrite_plans(
            &paths,
            &walkable_cells,
            DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE,
            DEFAULT_MAX_REWRITE_PROPOSALS,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_WINDOW,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL,
        );

        assert!(
            plans.iter().any(|plan| {
                apply_rewrite_plan(&paths, plan)
                    .as_ref()
                    .is_some_and(|proposal| {
                        proposal
                            .iter()
                            .any(|path| path.last().copied() == Some((1, 1)))
                    })
            }),
            "expected at least one proposal with an invented destination"
        );
    }

    #[test]
    fn allows_remove_only_rewrite_when_survivors_are_unchanged() {
        let paths = vec![
            vec![(0, 0), (1, 0)],
            vec![(1, 0), (2, 0)],
            vec![(5, 0), (4, 0)],
        ];
        let walkable_cells = (0..=6).map(|x| (x, 0)).collect::<Vec<_>>();
        let expected = vec![vec![(0, 0), (1, 0)], vec![(5, 0), (4, 0)]];

        let plans = generate_k_minus_one_rewrite_plans(
            &paths,
            &walkable_cells,
            DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE,
            DEFAULT_MAX_REWRITE_PROPOSALS,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_WINDOW,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL,
        );

        assert!(
            plans.iter().any(|plan| {
                apply_rewrite_plan(&paths, plan)
                    .as_ref()
                    .is_some_and(|proposal| *proposal == expected)
            }),
            "expected remove-only rewrite with unchanged survivor endpoints"
        );
    }

    #[test]
    fn plan_dedup_is_applied() {
        let paths = vec![
            vec![(0, 0), (1, 0)],
            vec![(3, 0), (2, 0)],
            vec![(6, 0), (5, 0)],
            vec![(9, 0), (8, 0)],
        ];
        let walkable_cells = (0..=10).map(|x| (x, 0)).collect::<Vec<_>>();

        let plans = generate_k_minus_one_rewrite_plans(
            &paths,
            &walkable_cells,
            DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE,
            DEFAULT_MAX_REWRITE_PROPOSALS,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_WINDOW,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL,
        );
        let unique = plans.iter().cloned().collect::<HashSet<_>>();
        assert_eq!(plans.len(), unique.len());
    }

    #[test]
    fn proposal_generation_respects_global_cap() {
        let paths = vec![
            vec![(0, 0), (1, 0)],
            vec![(3, 0), (2, 0)],
            vec![(6, 0), (5, 0)],
            vec![(9, 0), (8, 0)],
            vec![(12, 0), (11, 0)],
        ];
        let walkable_cells = (0..=12).map(|x| (x, 0)).collect::<Vec<_>>();

        let proposals = generate_k_minus_one_rewrite_plans(
            &paths,
            &walkable_cells,
            DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE,
            5,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_WINDOW,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL,
        );
        assert_eq!(proposals.len(), 5);
    }

    #[test]
    fn proposal_generation_respects_per_window_cap() {
        let paths = vec![
            vec![(0, 0), (1, 0)],
            vec![(3, 0), (2, 0)],
            vec![(6, 0), (5, 0)],
            vec![(9, 0), (8, 0)],
            vec![(12, 0), (11, 0)],
        ];
        let walkable_cells = (0..=12).map(|x| (x, 0)).collect::<Vec<_>>();

        let proposals = generate_k_minus_one_rewrite_plans(
            &paths,
            &walkable_cells,
            DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE,
            usize::MAX,
            1,
            DEFAULT_MAX_REWRITE_PROPOSALS_PER_REMOVAL,
        );
        assert!(proposals.len() <= 9);
    }

    #[test]
    fn proposal_generation_respects_per_removal_cap() {
        let paths = vec![
            vec![(0, 0), (1, 0)],
            vec![(3, 0), (2, 0)],
            vec![(6, 0), (5, 0)],
        ];
        let walkable_cells = (0..=8).map(|x| (x, 0)).collect::<Vec<_>>();

        let proposals = generate_k_minus_one_rewrite_plans(
            &paths,
            &walkable_cells,
            DEFAULT_MAX_INVENTED_DESTINATIONS_PER_MOVE,
            usize::MAX,
            usize::MAX,
            1,
        );
        assert!(proposals.len() <= 3);
    }
}
