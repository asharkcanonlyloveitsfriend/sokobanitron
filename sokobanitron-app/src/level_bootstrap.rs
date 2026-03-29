use crate::persistence::{BootstrappedLevelStore, LevelPersistence, LevelSetCatalogEntry};
use sokobanitron_gameplay::{BoardView, GameplayController, OrientationPolicy};
use std::path::Path;

const DEFAULT_FALLBACK_LEVEL_LINES: [&str; 4] =
    ["    ###   ", " $$     #@", " $ #...   ", "   #######"];

pub struct InitialLevels {
    pub levels: Vec<String>,
    pub preview_boards: Vec<BoardView>,
    pub initial_level_index: usize,
    pub persisted_resume_level_index: Option<usize>,
    pub persistence: LevelPersistence,
    pub level_set_catalog: Vec<LevelSetCatalogEntry>,
    pub active_level_set_index: usize,
}

pub fn load_initial_levels_for_app(level_sets_root: &Path) -> InitialLevels {
    match LevelPersistence::bootstrap(level_sets_root, OrientationPolicy::RotateWideToPortrait) {
        Ok(bootstrapped) if !bootstrapped.levels.is_empty() => {
            initial_levels_from_bootstrap(bootstrapped)
        }
        Ok(_) => fallback_initial_levels(),
        Err(err) => {
            eprintln!(
                "warning: failed to initialize persistent level storage at {}: {err}",
                level_sets_root.display()
            );
            fallback_initial_levels()
        }
    }
}

fn fallback_level_ascii() -> String {
    DEFAULT_FALLBACK_LEVEL_LINES.join("\n")
}

fn fallback_initial_levels() -> InitialLevels {
    let levels = vec![fallback_level_ascii()];
    let preview_boards = levels
        .iter()
        .map(String::as_str)
        .map(build_preview_board)
        .collect();

    InitialLevels {
        levels,
        preview_boards,
        initial_level_index: 0,
        persisted_resume_level_index: None,
        persistence: LevelPersistence::default(),
        level_set_catalog: Vec::new(),
        active_level_set_index: 0,
    }
}

fn initial_levels_from_bootstrap(bootstrapped: BootstrappedLevelStore) -> InitialLevels {
    let preview_boards = build_preview_boards(&bootstrapped.levels);

    InitialLevels {
        levels: bootstrapped.levels,
        preview_boards,
        initial_level_index: bootstrapped.initial_level_index,
        persisted_resume_level_index: bootstrapped.persisted_resume_level_index,
        persistence: bootstrapped.persistence,
        level_set_catalog: bootstrapped.level_set_catalog,
        active_level_set_index: bootstrapped.active_level_set_index,
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
