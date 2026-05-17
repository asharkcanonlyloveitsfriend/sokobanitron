use sokobanitron_core::pathfinder::BoxPathfinder;
use sokobanitron_core::pathfinder::Position;

#[test]
fn find_box_path_straight_line_with_player_access() {
    let ascii_map = "\
#######\n\
#@    #\n\
# $  x#\n\
#######";
    let (mut mover, to, _) = parse_box_mover_with_endpoints(ascii_map);

    let expected_path = vec![
        Position::new(2, 2),
        Position::new(2, 3),
        Position::new(2, 4),
        Position::new(2, 5),
    ];
    let path = mover.find_box_path(to);

    assert_eq!(Some(expected_path), path);
}

#[test]
fn find_box_path_not_straight_line() {
    let ascii_map = "\
#####\n\
#@  #\n\
# $ #\n\
#  x#\n\
#####";
    let (mut mover, to, _) = parse_box_mover_with_endpoints(ascii_map);

    let expected_path = vec![
        Position::new(2, 2),
        Position::new(3, 2),
        Position::new(3, 3),
    ];
    let path = mover.find_box_path(to);

    assert_eq!(Some(expected_path), path);
}

#[test]
fn find_box_path_complex_path() {
    let ascii_map = "\
###################\n\
# ###   ##        #\n\
# #  $#  #        #\n\
### # ## #   ######\n\
#   # ## ## ##    #\n\
# #              ##\n\
#    ####    @#  x#\n\
###################";
    let (mut mover, to, box_position) = parse_box_mover_with_endpoints(ascii_map);

    let path = mover
        .find_box_path(to)
        .expect("complex path should be found");

    assert!(!path.is_empty());
    assert_eq!(box_position, path[0]);
    assert_eq!(to, path[path.len() - 1]);
}

#[test]
fn find_box_path_blocked() {
    let ascii_map = "\
#####\n\
#   #\n\
###$#\n\
#  @#\n\
# #x#\n\
#####";
    let (mut mover, to, _) = parse_box_mover_with_endpoints(ascii_map);

    assert_eq!(None, mover.find_box_path(to));
}

#[test]
fn benchmark_fixture_cases_return_paths_from_box_to_target() {
    let cases = [
        (
            "l5",
            include_str!("../benches/fixtures/box_pathfinder/l5.txt"),
        ),
        (
            "micro94",
            include_str!("../benches/fixtures/box_pathfinder/micro94.txt"),
        ),
        (
            "misc19",
            include_str!("../benches/fixtures/box_pathfinder/misc19.txt"),
        ),
        (
            "misc22",
            include_str!("../benches/fixtures/box_pathfinder/misc22.txt"),
        ),
        (
            "sas27",
            include_str!("../benches/fixtures/box_pathfinder/sas27.txt"),
        ),
    ];

    for (name, ascii_map) in cases {
        let (mut pathfinder, to, box_pos) = parse_box_pathfinder_with_target(ascii_map);
        let path = pathfinder
            .find_box_path(to)
            .unwrap_or_else(|| panic!("benchmark case should resolve: {name}"));

        assert_eq!(
            path.first().copied(),
            Some(box_pos),
            "benchmark case should start at the box position: {name}"
        );
        assert_eq!(
            path.last().copied(),
            Some(to),
            "benchmark case should end at the target: {name}"
        );
    }
}

fn parse_box_mover_with_endpoints(ascii_map: &str) -> (BoxPathfinder, Position, Position) {
    let mut player = None;
    let mut to = None;
    let mut box_pos = None;

    let grid = ascii_map
        .lines()
        .enumerate()
        .map(|(row_index, row)| {
            row.chars()
                .enumerate()
                .map(|(col_index, ch)| match ch {
                    '@' => {
                        player = Some(Position::new(row_index, col_index));
                        true
                    }
                    'x' => {
                        to = Some(Position::new(row_index, col_index));
                        true
                    }
                    '$' => {
                        box_pos = Some(Position::new(row_index, col_index));
                        true
                    }
                    '#' => false,
                    ' ' => true,
                    _ => panic!("unsupported character: {ch}"),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let player = player.expect("map must contain '@'");
    let to = to.expect("map must contain 'x'");
    let box_pos = box_pos.expect("map must contain '$'");

    (BoxPathfinder::new(grid, box_pos, player), to, box_pos)
}

fn parse_box_pathfinder_with_target(ascii_map: &str) -> (BoxPathfinder, Position, Position) {
    let lines = ascii_map.lines().collect::<Vec<_>>();
    let width = lines.iter().map(|line| line.len()).max().unwrap_or(0);
    let mut player = None;
    let mut to = None;
    let mut box_pos = None;

    let grid = lines
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            row.chars()
                .chain(std::iter::repeat('#'))
                .take(width)
                .enumerate()
                .map(|(col_index, ch)| match ch {
                    '#' | '$' => false,
                    'b' => {
                        box_pos = Some(Position::new(row_index, col_index));
                        true
                    }
                    '@' => {
                        player = Some(Position::new(row_index, col_index));
                        true
                    }
                    'x' => {
                        to = Some(Position::new(row_index, col_index));
                        true
                    }
                    ' ' | '.' | '*' | '+' => true,
                    _ => panic!("unsupported character: {ch}"),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let player = player.expect("map must contain '@'");
    let to = to.expect("map must contain 'x'");
    let box_pos = box_pos.expect("map must contain 'b'");

    (BoxPathfinder::new(grid, box_pos, player), to, box_pos)
}
