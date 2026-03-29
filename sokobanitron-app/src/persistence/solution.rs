use std::io;

pub(crate) fn solution_history_to_json(
    solution_history: &[Vec<(usize, usize)>],
) -> io::Result<String> {
    serde_json::to_string(
        &solution_history
            .iter()
            .map(|path| {
                path.iter()
                    .map(|(row, col)| [*row, *col])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>(),
    )
    .map_err(|err| io::Error::other(format!("serialize solution history: {err}")))
}

pub(crate) fn should_replace_solution(existing_json: Option<&str>, new_json: &str) -> bool {
    let new_score = match solution_score_from_json(new_json) {
        Some(score) => score,
        None => return false,
    };
    let Some(existing_json) = existing_json else {
        return true;
    };
    let Some(existing_score) = solution_score_from_json(existing_json) else {
        return true;
    };
    new_score < existing_score
}

pub(crate) fn solution_score_from_json(json_text: &str) -> Option<(usize, usize)> {
    let paths = serde_json::from_str::<Vec<Vec<[usize; 2]>>>(json_text).ok()?;
    let push_count = paths.len();
    let total_push_distance = paths
        .iter()
        .map(|path| path.len().saturating_sub(1))
        .sum::<usize>();
    Some((push_count, total_push_distance))
}
