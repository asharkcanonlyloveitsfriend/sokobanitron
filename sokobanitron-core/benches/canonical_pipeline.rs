use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rusqlite::Connection;
use sokobanitron_core::canonical_hash;
use std::hint::black_box;
use std::path::PathBuf;

#[cfg(feature = "stage-profile")]
use sokobanitron_core::stage_profile;

fn load_all_grids() -> Vec<String> {
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
            Ok(grid)
        })
        .expect("Failed to query rows");

    rows.map(|r| r.expect("Row decode failed")).collect()
}

fn bench_canonical_pipeline(c: &mut Criterion) {
    // Load once outside the timing loop
    let grids = load_all_grids();

    let mut group = c.benchmark_group("canonical");

    group.bench_function(BenchmarkId::new("pipeline", grids.len()), |b| {
        b.iter(|| {
            #[cfg(feature = "stage-profile")]
            stage_profile::reset();

            for grid in &grids {
                let hash = canonical_hash(grid).expect("canonical_hash returned error");
                black_box(hash);
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

criterion_group!(benches, bench_canonical_pipeline);
criterion_main!(benches);
