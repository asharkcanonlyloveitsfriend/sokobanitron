use sokobanitron_core::pathfinder::{Pathfinder, Position};

#[test]
fn can_find_path_straight_line_clear() {
    let ascii_map = "\
#####\n\
#@ x#\n\
#   #\n\
#   #\n\
#####";
    let (mut pathfinder, from, to) = parse_pathfinder_with_endpoints(ascii_map);

    assert!(pathfinder.can_find_path(from, to, None));
}

#[test]
fn can_find_path_straight_line_blocked_by_wall() {
    let ascii_map = "\
#####\n\
#@#x#\n\
#   #\n\
#   #\n\
#####";
    let (mut pathfinder, from, to) = parse_pathfinder_with_endpoints(ascii_map);

    assert!(pathfinder.can_find_path(from, to, None));
}

#[test]
fn can_find_path_multi_turn() {
    let ascii_map = "\
#########\n\
#@     ##\n\
### #   #\n\
#x# ### #\n\
#     # #\n\
#########";
    let (mut pathfinder, from, to) = parse_pathfinder_with_endpoints(ascii_map);

    assert!(pathfinder.can_find_path(from, to, None));
}

#[test]
fn can_find_path_turn_corner() {
    let ascii_map = "\
#####\n\
#@  #\n\
# # #\n\
#  x#\n\
#####";
    let (mut pathfinder, from, to) = parse_pathfinder_with_endpoints(ascii_map);

    assert!(pathfinder.can_find_path(from, to, None));
}

#[test]
fn can_find_path_completely_blocked() {
    let ascii_map = "\
#######\n\
#@ # x#\n\
#######";
    let (mut pathfinder, from, to) = parse_pathfinder_with_endpoints(ascii_map);

    assert!(!pathfinder.can_find_path(from, to, None));
}

fn parse_pathfinder_with_endpoints(ascii_map: &str) -> (Pathfinder, Position, Position) {
    let mut from = None;
    let mut to = None;

    for (row_index, row) in ascii_map.lines().enumerate() {
        for (col_index, ch) in row.chars().enumerate() {
            match ch {
                '@' => from = Some(Position::new(row_index, col_index)),
                'x' => to = Some(Position::new(row_index, col_index)),
                _ => {}
            }
        }
    }

    let from = from.expect("map must contain '@'");
    let to = to.expect("map must contain 'x'");

    (create_pathfinder_from_ascii(ascii_map), from, to)
}

fn create_pathfinder_from_ascii(ascii_map: &str) -> Pathfinder {
    let rows = ascii_map
        .lines()
        .map(|row| {
            row.chars()
                .map(|ch| match ch {
                    '#' | '$' => false,
                    ' ' | '@' | 'x' => true,
                    _ => panic!("unsupported character: {ch}"),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    Pathfinder::from_rows(rows)
}
