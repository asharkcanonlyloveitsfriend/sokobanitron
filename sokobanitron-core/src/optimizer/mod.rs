mod model;
mod passes;
mod replay_reverse;

pub use model::{BoxMovePath, Coord, ReverseOptimizationInput};

use passes::{jump_ahead_proposals, normalize_paths_in_place, optimize_adjacent_merge_in_place};
use replay_reverse::replay_reverse_solution;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SolutionScore {
    discrete_moves: usize,
    total_path_steps: usize,
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

pub fn optimize_box_move_paths_in_place(paths: &mut Vec<BoxMovePath>) {
    optimize_adjacent_merge_in_place(paths);
}

pub fn optimize_reverse_solution_in_place(
    input: &ReverseOptimizationInput,
    paths: &mut Vec<BoxMovePath>,
) -> bool {
    let before = paths.clone();
    normalize_paths_in_place(paths);
    optimize_adjacent_merge_in_place(paths);

    loop {
        let current_score = score(paths);
        let proposals = jump_ahead_proposals(paths);
        let mut best_candidate: Option<(Vec<BoxMovePath>, SolutionScore)> = None;

        for proposal in proposals {
            let Some(mut realized) = replay_reverse_solution(input, &proposal) else {
                continue;
            };
            normalize_paths_in_place(&mut realized);
            optimize_adjacent_merge_in_place(&mut realized);
            let candidate_score = score(&realized);
            if candidate_score >= current_score {
                continue;
            }
            let should_replace = match &best_candidate {
                None => true,
                Some((_, best_score)) => candidate_score < *best_score,
            };
            if should_replace {
                best_candidate = Some((realized, candidate_score));
            }
        }

        if let Some((candidate, _)) = best_candidate {
            *paths = candidate;
        } else {
            break;
        }
    }

    *paths != before
}

#[cfg(test)]
mod tests {
    use super::{
        ReverseOptimizationInput, optimize_box_move_paths_in_place,
        optimize_reverse_solution_in_place,
    };

    #[test]
    fn merges_contiguous_adjacent_paths() {
        let mut paths = vec![vec![(0, 0), (1, 0)], vec![(1, 0), (2, 0)]];
        optimize_box_move_paths_in_place(&mut paths);
        assert_eq!(paths, vec![vec![(0, 0), (1, 0), (2, 0)]]);
    }

    #[test]
    fn jump_ahead_reduces_interleaved_solution() {
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

        let changed = optimize_reverse_solution_in_place(&input, &mut paths);
        assert!(changed);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].first().copied(), Some((2, 2)));
        assert_eq!(paths[0].last().copied(), Some((2, 5)));
        assert_eq!(paths[1].first().copied(), Some((4, 2)));
        assert_eq!(paths[1].last().copied(), Some((4, 5)));
    }
}
