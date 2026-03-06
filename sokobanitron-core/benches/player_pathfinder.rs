use criterion::{Criterion, criterion_group, criterion_main};
use sokobanitron_core::pathfinder::{PlayerPathfinder, Position};
use std::hint::black_box;

fn parse_player_pathfinder_with_endpoints(
    ascii_map: &str,
) -> (PlayerPathfinder, Position, Position) {
    let lines = ascii_map.lines().collect::<Vec<_>>();
    let width = lines.iter().map(|line| line.len()).max().unwrap_or(0);

    let mut from = None;
    let mut to = None;

    let rows = lines
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            row.chars()
                .chain(std::iter::repeat('#'))
                .take(width)
                .enumerate()
                .map(|(col_index, ch)| match ch {
                    '@' => {
                        from = Some(Position::new(row_index, col_index));
                        true
                    }
                    'x' => {
                        to = Some(Position::new(row_index, col_index));
                        true
                    }
                    '#' | '$' => false,
                    ' ' | '.' | '*' | '+' => true,
                    _ => panic!("unsupported character: {ch}"),
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    (
        PlayerPathfinder::from_rows(rows),
        from.expect("map must contain '@'"),
        to.expect("map must contain 'x'"),
    )
}

fn bench_player_pathfinder(c: &mut Criterion) {
    let ascii_map = "\
       ######\n\
    ####    ###\n\
    #x   ##   #\n\
 #### ###..## ####\n\
 #  #$$     #    #\n\
##  $ $ #...   # #\n\
#  #  $ ##### #  #\n\
#    ####  #   @##\n\
##   #     #   ##\n\
 #####     #####";

    let (mut pathfinder, from, to) = parse_player_pathfinder_with_endpoints(ascii_map);

    for _ in 0..1_000 {
        black_box(pathfinder.can_find_path(from, to, None));
    }

    c.bench_function("player_pathfinder/can_find_path_baseline", |b| {
        b.iter(|| {
            black_box(pathfinder.can_find_path(from, to, None));
        })
    });
}

criterion_group!(benches, bench_player_pathfinder);
criterion_main!(benches);
