use std::env;
use std::path::PathBuf;

use rusqlite::Connection;
use sokobanitron_core::canonical_hash;

#[test]
fn canonical_hash_matches_database() {
    // Resolve DB path relative to workspace root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let db_path: PathBuf = [manifest_dir, "benches", "fixtures", "puzzles.db"]
        .iter()
        .collect();

    assert!(db_path.exists(), "Database not found at {:?}", db_path);

    let conn = Connection::open(&db_path).expect("Failed to open puzzles.db");

    let mut stmt = conn
        .prepare("SELECT grid, canonical_hash FROM puzzles")
        .expect("Failed to prepare SELECT statement");

    let rows = stmt
        .query_map([], |row| {
            let grid: String = row.get(0)?;
            let expected_hash: String = row.get(1)?;
            Ok((grid, expected_hash))
        })
        .expect("Failed to query rows");

    for (index, row) in rows.enumerate() {
        let (grid, expected_hash) = row.expect("Row decode failed");

        let computed = canonical_hash(&grid).expect("canonical_hash returned error");

        assert_eq!(
            computed, expected_hash,
            "Mismatch at row {}\nExpected: {}\nComputed: {}",
            index, expected_hash, computed
        );
    }
}
