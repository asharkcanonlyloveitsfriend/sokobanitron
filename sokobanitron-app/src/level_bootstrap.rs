use crate::persistence::{BootstrappedLevelStore, LevelPersistence, LevelSetCatalogEntry};
use sokobanitron_gameplay::{BoardView, GameplayController, OrientationPolicy};
use std::io;
use std::path::Path;

pub struct InitialLevels {
    pub levels: Vec<String>,
    pub preview_boards: Vec<BoardView>,
    pub initial_level_index: usize,
    pub persisted_resume_level_index: Option<usize>,
    pub persistence: LevelPersistence,
    pub level_set_catalog: Vec<LevelSetCatalogEntry>,
    pub active_level_set_index: usize,
}

pub fn load_initial_levels_for_app(level_sets_root: &Path) -> io::Result<InitialLevels> {
    let bootstrapped =
        LevelPersistence::bootstrap(level_sets_root, OrientationPolicy::RotateWideToPortrait)?;
    if bootstrapped.levels.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no levels available in {}; import an .slc file into to_import",
                level_sets_root.display()
            ),
        ));
    }

    Ok(initial_levels_from_bootstrap(bootstrapped))
}

fn initial_levels_from_bootstrap(bootstrapped: BootstrappedLevelStore) -> InitialLevels {
    debug_assert!(
        bootstrapped.levels.is_empty() == bootstrapped.active_level_set_index.is_none(),
        "bootstrapped active level set index should be present iff levels are present"
    );
    let active_level_set_index = bootstrapped
        .active_level_set_index
        .expect("playable bootstrap result must include an active level set index");
    let preview_boards = build_preview_boards(&bootstrapped.levels);

    InitialLevels {
        levels: bootstrapped.levels,
        preview_boards,
        initial_level_index: bootstrapped.initial_level_index,
        persisted_resume_level_index: bootstrapped.persisted_resume_level_index,
        persistence: bootstrapped.persistence,
        level_set_catalog: bootstrapped.level_set_catalog,
        active_level_set_index,
    }
}

pub fn build_preview_boards(levels: &[String]) -> Vec<BoardView> {
    levels
        .iter()
        .map(String::as_str)
        .map(build_preview_board)
        .collect()
}

fn build_preview_board(level_ascii: &str) -> BoardView {
    GameplayController::new(vec![level_ascii.to_string()], None)
        .board()
        .clone()
}

#[cfg(test)]
mod tests {
    use super::load_initial_levels_for_app;
    use std::fs;
    use std::io;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn empty_store_bootstraps_without_fallback_content() {
        let root = temp_dir("empty-store");

        let err = match load_initial_levels_for_app(&root) {
            Ok(_) => panic!("empty store should fail"),
            Err(err) => err,
        };

        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("no levels available"));

        fs::remove_dir_all(root).expect("cleanup");
    }

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_nanos();
        let unique = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "sokobanitron-level-bootstrap-{name}-{nanos}-{unique}"
        ));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
