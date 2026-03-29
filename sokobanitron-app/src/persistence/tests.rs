use super::LevelPersistence;
use super::slc::parse_slc_xml;
use super::solution::{
    should_replace_solution, solution_history_to_json, solution_score_from_json,
};
use sokobanitron_gameplay::OrientationPolicy;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("sokobanitron-persistence-{name}-{nanos}"));
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
    let inbox = root.join("to_import");
    fs::create_dir_all(&inbox).expect("create inbox");
    fs::write(
        inbox.join("test_set.slc"),
        r#"
            <SokobanLevels>
              <Title>Test Set</Title>
              <LevelCollection>
                <Level Id="1">
                  <L>#####</L>
                  <L>#@$.#</L>
                  <L>#####</L>
                </Level>
                <Level Id="2">
                  <L>#####</L>
                  <L>#@ $.#</L>
                  <L>#####</L>
                </Level>
              </LevelCollection>
            </SokobanLevels>
            "#,
    )
    .expect("write slc");

    let mut bootstrapped =
        LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("bootstrap");
    assert_eq!(bootstrapped.levels.len(), 2);
    assert_eq!(bootstrapped.initial_level_index, 0);
    assert_eq!(bootstrapped.persisted_resume_level_index, None);

    bootstrapped
        .persistence
        .persist_resume_level(1)
        .expect("persist resume");

    let reloaded = LevelPersistence::bootstrap(&root, OrientationPolicy::Keep).expect("reload");
    assert_eq!(reloaded.persisted_resume_level_index, Some(1));
    assert_eq!(reloaded.initial_level_index, 1);

    fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn record_completion_updates_catalog_count_for_active_set() {
    let root = temp_dir("catalog-count");
    let inbox = root.join("to_import");
    fs::create_dir_all(&inbox).expect("create inbox");
    fs::write(
        inbox.join("test_set.slc"),
        r#"
            <SokobanLevels>
              <Title>Count Test</Title>
              <LevelCollection>
                <Level Id="1">
                  <L>#####</L>
                  <L>#@$.#</L>
                  <L>#####</L>
                </Level>
                <Level Id="2">
                  <L>#####</L>
                  <L>#@ $.#</L>
                  <L>#####</L>
                </Level>
              </LevelCollection>
            </SokobanLevels>
            "#,
    )
    .expect("write slc");

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
