use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use sokobanitron_core::pathfinder::BoxPathfinder;
use sokobanitron_core::pathfinder::Position;
use std::hint::black_box;

struct Case {
    name: &'static str,
    ascii_map: &'static str,
}

fn parse_box_pathfinder_with_target(ascii_map: &str) -> (BoxPathfinder, Position) {
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

    (BoxPathfinder::new(grid, box_pos, player), to)
}

fn bench_box_pathfinder(c: &mut Criterion) {
    let cases = [
        Case {
            name: "l5",
            ascii_map: include_str!("fixtures/box_pathfinder/l5.txt"),
        },
        Case {
            name: "micro94",
            ascii_map: include_str!("fixtures/box_pathfinder/micro94.txt"),
        },
        Case {
            name: "misc19",
            ascii_map: include_str!("fixtures/box_pathfinder/misc19.txt"),
        },
        Case {
            name: "misc22",
            ascii_map: include_str!("fixtures/box_pathfinder/misc22.txt"),
        },
        Case {
            name: "sas27",
            ascii_map: include_str!("fixtures/box_pathfinder/sas27.txt"),
        },
    ];

    let mut group = c.benchmark_group("box_pathfinder/find_box_path");

    for case in cases {
        let (mut pathfinder, to) = parse_box_pathfinder_with_target(case.ascii_map);

        let baseline = pathfinder
            .find_box_path(to)
            .unwrap_or_else(|| panic!("case {} must produce a path", case.name));
        black_box(&baseline);

        for _ in 0..1_000 {
            black_box(pathfinder.find_box_path(to));
        }

        group.bench_with_input(
            BenchmarkId::from_parameter(case.name),
            &case.name,
            |b, _| {
                b.iter(|| {
                    black_box(pathfinder.find_box_path(to));
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_box_pathfinder);
criterion_main!(benches);
