use super::slc::parse_slc_xml;
use super::solution::{
    should_replace_solution, solution_history_to_json, solution_score_from_json,
};
use super::{LevelPersistence, LevelSetKind};
use rusqlite::Connection;
use sokobanitron_gameplay::OrientationPolicy;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let unique = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
    let dir =
        std::env::temp_dir().join(format!("sokobanitron-persistence-{name}-{nanos}-{unique}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

#[test]
fn parses_slc_title_and_levels() {
    let parsed = parse_slc_xml(
        r#"
            <SokobanLevels>
              <Title>Test Set</Title>
              <LevelCollection>
                <Level Id="1">
                  <L>###</L>
                  <L>#@.</L>
                </Level>
                <Level Id="2">
                  <L>####</L>
                </Level>
              </LevelCollection>
            </SokobanLevels>
            "#,
    )
    .expect("parse slc");

    assert_eq!(parsed.title, "Test Set");
    assert_eq!(parsed.levels.len(), 2);
    assert_eq!(parsed.levels[0].grid, "###\n#@.");
    assert_eq!(parsed.levels[1].grid, "####");
}

#[test]
fn bootstrap_imports_slc_and_tracks_resume_progress() {
    let root = temp_dir("bootstrap");
    write_slc(
        &root,
        "test_set.slc",
        "Test Set",
        &["#####\n#@$.#\n#####", "#####\n#@ $.#\n#####"],
    );

    let mut bootstrapped =
        LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("bootstrap");
    assert_eq!(bootstrapped.levels.len(), 2);
    assert_eq!(bootstrapped.initial_level_index, 0);
    assert_eq!(bootstrapped.persisted_resume_level_index, None);
    assert_eq!(bootstrapped.active_level_set_index, Some(0));

    bootstrapped
        .persistence
        .persist_resume_level(1)
        .expect("persist resume");

    let reloaded = LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("reload");
    assert_eq!(reloaded.persisted_resume_level_index, Some(1));
    assert_eq!(reloaded.initial_level_index, 1);
    assert_eq!(reloaded.active_level_set_index, Some(0));

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn record_completion_updates_catalog_count_for_active_set() {
    let root = temp_dir("catalog-count");
    write_slc(
        &root,
        "count_test.slc",
        "Count Test",
        &["#####\n#@$.#\n#####", "#####\n#@ $.#\n#####"],
    );

    let mut bootstrapped =
        LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("bootstrap");
    assert_eq!(bootstrapped.level_set_catalog.len(), 1);
    assert_eq!(bootstrapped.level_set_catalog[0].completed_puzzle_count, 0);

    bootstrapped
        .persistence
        .record_completion(0, &[vec![(0, 0), (0, 1)]])
        .expect("record completion");

    let catalog = bootstrapped.persistence.level_set_catalog();
    assert_eq!(catalog[0].completed_puzzle_count, 1);
    assert_eq!(catalog[0].total_puzzle_count, 2);

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn better_solution_requires_fewer_pushes_then_shorter_distance() {
    let best = solution_history_to_json(&[vec![(0, 0), (0, 1)]]).expect("serialize best");
    let worse_more_pushes = solution_history_to_json(&[vec![(0, 0), (0, 1)], vec![(0, 1), (0, 2)]])
        .expect("serialize worse pushes");
    let worse_same_pushes_longer =
        solution_history_to_json(&[vec![(0, 0), (0, 1), (0, 2)]]).expect("serialize longer");

    assert!(should_replace_solution(Some(&worse_more_pushes), &best));
    assert!(should_replace_solution(
        Some(&worse_same_pushes_longer),
        &best
    ));
    assert!(!should_replace_solution(Some(&best), &best));
    assert_eq!(solution_score_from_json(&best), Some((1, 1)));
}

#[test]
fn save_created_puzzle_creates_user_set_in_empty_store() {
    let root = temp_dir("created-puzzle");

    let mut bootstrapped =
        LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("bootstrap");
    assert!(bootstrapped.levels.is_empty());
    assert!(bootstrapped.level_set_catalog.is_empty());
    assert_eq!(bootstrapped.persistence.active_level_set_index(), None);

    let saved = bootstrapped
        .persistence
        .save_created_puzzle("My Puzzles", "#####\n#@$.#\n#####", &[vec![(1, 2), (1, 3)]])
        .expect("save created puzzle");
    assert_eq!(saved.level_set_index, 0);
    assert_eq!(saved.level_index, 0);

    let catalog = bootstrapped.persistence.level_set_catalog();
    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog[0].kind, LevelSetKind::UserCreated);
    assert_eq!(catalog[0].title, "My Puzzles");
    assert_eq!(catalog[0].completed_puzzle_count, 0);
    assert_eq!(catalog[0].total_puzzle_count, 1);
    assert_eq!(bootstrapped.persistence.active_level_set_index(), Some(0));

    let reloaded = LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("reload");
    assert_eq!(reloaded.levels, vec!["#####\n#@$.#\n#####".to_string()]);
    assert_eq!(reloaded.level_set_catalog.len(), 1);
    assert_eq!(
        reloaded.level_set_catalog[0].kind,
        LevelSetKind::UserCreated
    );
    assert_eq!(reloaded.active_level_set_index, Some(0));
    assert_eq!(reloaded.persisted_resume_level_index, Some(0));

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn save_created_puzzle_appends_to_existing_user_set() {
    let root = temp_dir("append-created-puzzle");

    let mut bootstrapped =
        LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("bootstrap");

    bootstrapped
        .persistence
        .save_created_puzzle("My Puzzles", "#####\n#@$.#\n#####", &[vec![(1, 2), (1, 3)]])
        .expect("save first puzzle");
    let second = bootstrapped
        .persistence
        .save_created_puzzle(
            "My Puzzles",
            "######\n#@ $.#\n######",
            &[vec![(1, 3), (1, 4)]],
        )
        .expect("save second puzzle");

    assert_eq!(second.level_set_index, 0);
    assert_eq!(second.level_index, 1);

    let reloaded = LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("reload");
    assert_eq!(reloaded.levels.len(), 2);
    assert_eq!(reloaded.levels[0], "#####\n#@$.#\n#####");
    assert_eq!(reloaded.levels[1], "######\n#@ $.#\n######");

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn bootstrap_empty_store_returns_empty_levels_without_fallback() {
    let root = temp_dir("empty-bootstrap");

    let bootstrapped =
        LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("bootstrap");

    assert!(bootstrapped.levels.is_empty());
    assert!(bootstrapped.level_set_catalog.is_empty());
    assert_eq!(bootstrapped.initial_level_index, 0);
    assert_eq!(bootstrapped.persisted_resume_level_index, None);
    assert_eq!(bootstrapped.active_level_set_index, None);
    assert_eq!(bootstrapped.persistence.active_level_set_index(), None);

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn bootstrap_with_non_empty_catalog_and_stored_active_set_loads_that_set() {
    let root = temp_dir("stored-active-set");
    write_slc(&root, "alpha.slc", "Alpha", &["#####\n#@$.#\n#####"]);
    write_slc(
        &root,
        "beta.slc",
        "Beta",
        &["#######\n#@  $.#\n#######", "#######\n#@ $. #\n#######"],
    );

    let mut bootstrapped =
        LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("bootstrap");
    bootstrapped
        .persistence
        .switch_to_level_set(1)
        .expect("switch level set")
        .expect("loaded level set");
    bootstrapped
        .persistence
        .persist_resume_level(0)
        .expect("persist active set");

    let reloaded = LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("reload");
    assert_eq!(reloaded.active_level_set_index, Some(1));
    assert_eq!(reloaded.level_set_catalog[1].title, "Beta");
    assert_eq!(
        reloaded.levels,
        vec!["@  $.".to_string(), "@ $.".to_string()]
    );

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn bootstrap_with_non_empty_catalog_and_no_stored_active_set_uses_first_set() {
    let root = temp_dir("first-active-set");
    write_slc(&root, "alpha.slc", "Alpha", &["#####\n#@$.#\n#####"]);
    write_slc(&root, "beta.slc", "Beta", &["#######\n#@  $.#\n#######"]);

    let bootstrapped =
        LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("bootstrap");

    assert_eq!(bootstrapped.active_level_set_index, Some(0));
    assert_eq!(bootstrapped.level_set_catalog[0].title, "Alpha");
    assert_eq!(bootstrapped.levels, vec!["@$.".to_string()]);

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn bootstrap_with_invalid_stored_active_set_falls_back_to_first_valid_set() {
    let root = temp_dir("invalid-active-set");
    write_slc(&root, "alpha.slc", "Alpha", &["#####\n#@$.#\n#####"]);
    write_slc(&root, "beta.slc", "Beta", &["#######\n#@  $.#\n#######"]);

    let _ = LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("bootstrap");
    let db_path = root.join("sokobanitron.db");
    let conn = Connection::open(&db_path).expect("open db");
    conn.execute_batch("PRAGMA foreign_keys = OFF;")
        .expect("disable foreign keys");
    conn.execute(
        "
        INSERT INTO level_set_progress (level_set_id, resume_level_id, updated_at)
        VALUES (999, 999, '2099-01-01 00:00:00')
        ",
        [],
    )
    .expect("insert invalid progress");
    drop(conn);

    let reloaded = LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("reload");
    assert_eq!(reloaded.active_level_set_index, Some(0));
    assert_eq!(reloaded.level_set_catalog[0].title, "Alpha");
    assert_eq!(reloaded.levels, vec!["@$.".to_string()]);

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn bootstrap_migrates_v1_schema_to_reference_solution_schema() {
    let root = temp_dir("migrate-v1");
    let db_path = root.join("sokobanitron.db");
    let conn = Connection::open(&db_path).expect("open db");
    conn.execute_batch(
        "
        PRAGMA user_version = 1;

        CREATE TABLE level_sets (
            id INTEGER PRIMARY KEY,
            title TEXT NOT NULL
        );

        CREATE TABLE puzzles (
            id INTEGER PRIMARY KEY,
            grid TEXT NOT NULL,
            last_completed_at TEXT,
            rating INTEGER NOT NULL DEFAULT 0 CHECK (rating IN (-1, 0, 1)),
            is_starred INTEGER NOT NULL DEFAULT 0 CHECK (is_starred IN (0, 1)),
            user_solution TEXT
        );

        CREATE TABLE levels (
            id INTEGER PRIMARY KEY,
            level_set_id INTEGER NOT NULL REFERENCES level_sets(id) ON DELETE CASCADE,
            ordinal INTEGER NOT NULL,
            puzzle_id INTEGER NOT NULL REFERENCES puzzles(id) ON DELETE CASCADE,
            UNIQUE(level_set_id, ordinal)
        );

        CREATE INDEX levels_level_set_id_idx ON levels(level_set_id);
        CREATE INDEX levels_puzzle_id_idx ON levels(puzzle_id);

        CREATE TABLE level_set_progress (
            level_set_id INTEGER PRIMARY KEY REFERENCES level_sets(id) ON DELETE CASCADE,
            resume_level_id INTEGER NOT NULL REFERENCES levels(id) ON DELETE CASCADE,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX level_set_progress_updated_at_idx
            ON level_set_progress(updated_at DESC);
        ",
    )
    .expect("seed v1 schema");
    drop(conn);

    let _ = LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("bootstrap");

    let conn = Connection::open(&db_path).expect("reopen db");
    let version = conn
        .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
        .expect("schema version");
    assert_eq!(version, 2);
    assert!(table_has_column(&conn, "level_sets", "kind"));
    assert!(table_has_column(&conn, "puzzles", "reference_solution"));

    fs::remove_dir_all(root).expect("cleanup");
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&pragma).expect("prepare table info");
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("query table info")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect columns");
    columns.iter().any(|name| name == column)
}

fn write_slc(root: &std::path::Path, filename: &str, title: &str, levels: &[&str]) {
    let inbox = root.join("to_import");
    fs::create_dir_all(&inbox).expect("create inbox");
    let level_xml = levels
        .iter()
        .enumerate()
        .map(|(index, grid)| {
            let lines = grid
                .lines()
                .map(|line| format!("    <L>{line}</L>"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("  <Level Id=\"{}\">\n{}\n  </Level>", index + 1, lines)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let xml = format!(
        "<SokobanLevels>\n  <Title>{title}</Title>\n  <LevelCollection>\n{level_xml}\n  </LevelCollection>\n</SokobanLevels>\n"
    );
    fs::write(inbox.join(filename), xml).expect("write slc");
}
