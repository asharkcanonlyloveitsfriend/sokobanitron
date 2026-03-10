use criterion::{BenchmarkId, Criterion, SamplingMode, criterion_group, criterion_main};
use rusqlite::Connection;
use sokobanitron_core::normalize_build_grid_then_prune_immovable_boxes_lines;
use std::hint::black_box;
use std::path::PathBuf;

#[cfg(feature = "stage-profile")]
use sokobanitron_core::stage_profile;

fn split_grid_lines(grid: &str) -> Vec<String> {
    grid.trim_matches('\n')
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect()
}

fn load_all_grids() -> Vec<Vec<String>> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let db_path: PathBuf = [manifest_dir, "benches", "fixtures", "puzzles.db"]
        .iter()
        .collect();

    let conn = Connection::open(&db_path).expect("Failed to open puzzles.db");

    let mut stmt = conn
        .prepare("SELECT grid FROM puzzles")
        .expect("Failed to prepare SELECT statement");

    let rows = stmt
        .query_map([], |row| {
            let grid: String = row.get(0)?;
            Ok(split_grid_lines(&grid))
        })
        .expect("Failed to query rows");

    rows.map(|r| r.expect("Row decode failed")).collect()
}

fn bench_normalize_build_grid_prune_immovable(c: &mut Criterion) {
    // Load and split once outside the timing loop.
    let grids = load_all_grids();

    let mut group = c.benchmark_group("normalize_slice");
    group.sample_size(50);
    group.sampling_mode(SamplingMode::Flat);
    group.bench_function(BenchmarkId::new("build_grid_prune_immovable", grids.len()), |b| {
        b.iter(|| {
            #[cfg(feature = "stage-profile")]
            stage_profile::reset();

            for grid_lines in &grids {
                let normalized = normalize_build_grid_then_prune_immovable_boxes_lines(grid_lines);
                black_box(normalized);
            }

            #[cfg(feature = "stage-profile")]
            {
                let report = stage_profile::report();
                black_box(&report);
            }
        })
    });
    group.finish();
}

criterion_group!(benches, bench_normalize_build_grid_prune_immovable);
criterion_main!(benches);
