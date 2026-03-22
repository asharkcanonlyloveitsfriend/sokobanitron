use crate::optimizer::model::BoxMovePath;
use std::collections::HashSet;

#[inline]
fn path_start(path: &BoxMovePath) -> Option<(i32, i32)> {
    path.first().copied()
}

#[inline]
fn path_end(path: &BoxMovePath) -> Option<(i32, i32)> {
    path.last().copied()
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

pub(crate) fn jump_ahead_proposals(paths: &[BoxMovePath]) -> Vec<Vec<BoxMovePath>> {
    let mut proposals = Vec::new();
    if paths.len() < 3 {
        return proposals;
    }

    for i in 0..paths.len() {
        let Some(start_i) = path_start(&paths[i]) else {
            continue;
        };
        let Some(mut current_end) = path_end(&paths[i]) else {
            continue;
        };

        let mut chain_indices = Vec::new();
        for (j, path) in paths.iter().enumerate().skip(i + 1) {
            if path_start(path) == Some(current_end) {
                chain_indices.push(j);
                if let Some(end) = path_end(path) {
                    current_end = end;
                }
            }
        }

        for k in 0..chain_indices.len() {
            let target_idx = chain_indices[k];
            let Some(target_end) = path_end(&paths[target_idx]) else {
                continue;
            };
            if target_end == start_i {
                continue;
            }

            let mut remove = HashSet::new();
            for idx in chain_indices.iter().take(k + 1) {
                remove.insert(*idx);
            }

            let mut proposal = Vec::with_capacity(paths.len() - remove.len());
            for (idx, path) in paths.iter().enumerate() {
                if idx == i {
                    proposal.push(vec![start_i, target_end]);
                } else if !remove.contains(&idx) {
                    proposal.push(path.clone());
                }
            }
            proposals.push(proposal);
        }
    }

    proposals
}
