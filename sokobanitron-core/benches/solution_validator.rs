use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rusqlite::Connection;
use sokobanitron_core::solution::{IndexedSolution, PreparedPuzzle, ValidationScratch};
use std::hint::black_box;
use std::path::PathBuf;

struct Puzzle {
    puzzle: PreparedPuzzle,
    solution: IndexedSolution,
    scratch: ValidationScratch,
}

fn load_all_puzzles() -> Vec<Puzzle> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let db_path: PathBuf = [manifest_dir, "benches", "fixtures", "normalized_puzzles.db"]
        .iter()
        .collect();

    let conn = Connection::open(&db_path).expect("Failed to open normalized_puzzles.db");

    let mut stmt = conn
        .prepare("SELECT grid, solution FROM puzzles")
        .expect("Failed to prepare SELECT statement");

    let rows = stmt
        .query_map([], |row| {
            let grid: String = row.get(0)?;
            let solution: String = row.get(1)?;
            Ok((grid, solution))
        })
        .expect("Failed to query rows");

    rows.map(|row| {
        let (grid, solution) = row.expect("Row decode failed");
        let grid = split_grid(&grid);
        let puzzle = PreparedPuzzle::from_normalized_lines(grid);
        let solution = parse_solution(&solution);
        let scratch = puzzle.scratch();
        Puzzle {
            puzzle,
            solution,
            scratch,
        }
    })
    .collect()
}

fn split_grid(grid: &str) -> Vec<String> {
    grid.trim_matches('\n')
        .lines()
        .map(|line| line.to_string())
        .collect()
}

fn parse_solution(solution: &str) -> IndexedSolution {
    serde_json::from_str(solution).expect("solution must be row-major index JSON paths")
}

fn bench_solution_validator(c: &mut Criterion) {
    let mut puzzles = load_all_puzzles();

    for puzzle in &mut puzzles {
        puzzle
            .puzzle
            .validate_indexed_solution_with_scratch(&puzzle.solution, &mut puzzle.scratch)
            .expect("normalized puzzle solution should validate");
    }

    let mut group = c.benchmark_group("solution_validator");

    group.bench_function(BenchmarkId::new("validate_all", puzzles.len()), |b| {
        b.iter(|| {
            for puzzle in &mut puzzles {
                let result = puzzle
                    .puzzle
                    .validate_indexed_solution_with_scratch(&puzzle.solution, &mut puzzle.scratch);
                result.expect("normalized puzzle solution should validate");
                black_box(());
            }
        })
    });

    group.finish();
}

criterion_group!(benches, bench_solution_validator);
criterion_main!(benches);
