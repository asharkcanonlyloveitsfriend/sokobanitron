use sokobanitron_gameplay::{
    BoardView, GameplayController, OrientationPolicy, load_levels_from_default_locations,
};

const DEFAULT_FALLBACK_LEVEL_LINES: [&str; 4] =
    ["    ###   ", " $$     #@", " $ #...   ", "   #######"];

#[derive(Debug, Clone)]
pub struct InitialLevels {
    pub levels: Vec<String>,
    pub preview_boards: Vec<BoardView>,
}

pub fn load_initial_levels_for_app() -> InitialLevels {
    let levels = load_levels_from_default_locations(
        OrientationPolicy::RotateWideToPortrait,
        &fallback_level_ascii(),
    );
    let preview_boards = levels
        .iter()
        .map(String::as_str)
        .map(build_preview_board)
        .collect();

    InitialLevels {
        levels,
        preview_boards,
    }
}

fn fallback_level_ascii() -> String {
    DEFAULT_FALLBACK_LEVEL_LINES.join("\n")
}

fn build_preview_board(level_ascii: &str) -> BoardView {
    GameplayController::new(vec![level_ascii.to_string()], None)
        .board()
        .clone()
}
