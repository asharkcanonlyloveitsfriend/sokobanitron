pub(crate) fn validate_characters(lines: &[String]) {
    for (row, line) in lines.iter().enumerate() {
        for (col, ch) in line.bytes().enumerate() {
            match ch {
                b'#' | b'.' | b'@' | b'$' | b'*' | b'+' | b' ' => {}
                _ => panic!(
                    "invalid Sokoban character '{}' at ({},{})",
                    ch as char, row, col
                ),
            }
        }
    }
}

pub(crate) fn assert_single_player(lines: &[String]) {
    let player_count = lines
        .iter()
        .flat_map(|l| l.bytes())
        .filter(|&b| b == b'@' || b == b'+')
        .count();

    assert!(player_count == 1, "puzzle must contain exactly one player");
}

pub(crate) fn assert_has_goal(lines: &[String]) {
    let goal_count = lines
        .iter()
        .flat_map(|l| l.bytes())
        .filter(|&b| b == b'.' || b == b'+' || b == b'*')
        .count();

    assert!(goal_count > 0, "puzzle must contain at least one goal");
}
