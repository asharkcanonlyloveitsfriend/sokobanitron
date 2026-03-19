use crate::level::parse_level_ascii;
use crate::presenter::{BoardView, GameBoardPresenter};
use sokobanitron_core::pathfinder::Position;
use sokobanitron_game_engine_jni::engine::GameEngine;
use std::collections::HashSet;

const DEFAULT_LEVEL_LINES: [&str; 4] = [
    "    ###   ",
    " $$     #@",
    " $ #...   ",
    "   #######",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameplayKey {
    Escape,
    Backspace,
    Other,
}

pub struct GameplaySession {
    level_ascii: String,
    presenter: GameBoardPresenter,
    engine: GameEngine,
    selected_box: Option<(u32, u32)>,
    pending_box_trail: Option<Vec<(u32, u32)>>,
    board: BoardView,
}

impl GameplaySession {
    pub fn new_default_level() -> Self {
        let level_ascii = DEFAULT_LEVEL_LINES.join("\n");
        Self::from_level_ascii(level_ascii)
    }

    pub fn from_level_ascii(level_ascii: String) -> Self {
        let parsed = parse_level_ascii(&level_ascii);
        let presenter = GameBoardPresenter::new(parsed);
        let engine = GameEngine::from_ascii(&level_ascii).expect("level ascii must parse");

        let player = engine.player();
        let player_xy = Some((player.col as u32, player.row as u32));
        let box_positions: HashSet<(u32, u32)> = engine
            .boxes()
            .iter()
            .map(|pos| (pos.col as u32, pos.row as u32))
            .collect();
        let board = presenter.render_board(
            player_xy,
            &box_positions,
            None,
            engine.is_level_solved(),
        );

        Self {
            level_ascii,
            presenter,
            engine,
            selected_box: None,
            pending_box_trail: None,
            board,
        }
    }

    pub fn board(&self) -> &BoardView {
        &self.board
    }

    pub fn is_started(&self) -> bool {
        !self.engine.is_at_start()
    }

    pub fn take_pending_box_trail(&mut self) -> Option<Vec<(u32, u32)>> {
        self.pending_box_trail.take()
    }

    pub fn click_cell(&mut self, x: u32, y: u32) {
        if self.board.is_won() {
            return;
        }

        let clicked_has_box = self
            .engine
            .boxes()
            .iter()
            .any(|pos| pos.col == x as usize && pos.row == y as usize);

        if clicked_has_box {
            self.selected_box = if self.selected_box == Some((x, y)) {
                None
            } else {
                Some((x, y))
            };
            self.sync_board();
            return;
        }

        if let Some((from_x, from_y)) = self.selected_box {
            let moved_box = self.engine.move_box_to(
                Position::new(from_y as usize, from_x as usize),
                Position::new(y as usize, x as usize),
            );
            if let Some(path) = moved_box {
                self.selected_box = None;
                self.pending_box_trail = Some(
                    path.into_iter()
                        .map(|p| (p.col as u32, p.row as u32))
                        .collect(),
                );
            }
            self.sync_board();
            return;
        }

        if self.engine.move_player_to(Position::new(y as usize, x as usize)) {
            self.sync_board();
        }
    }

    pub fn on_key(&mut self, key: GameplayKey) {
        if self.board.is_won() {
            if key == GameplayKey::Escape {
                self.restart();
            }
            return;
        }

        match key {
            GameplayKey::Backspace => {
                if self.engine.undo().is_some() {
                    self.selected_box = None;
                    self.pending_box_trail = None;
                    self.sync_board();
                }
            }
            GameplayKey::Escape => self.restart(),
            GameplayKey::Other => {}
        }
    }

    pub fn restart(&mut self) {
        self.engine = GameEngine::from_ascii(&self.level_ascii).expect("level ascii must parse");
        self.selected_box = None;
        self.pending_box_trail = None;
        self.sync_board();
    }

    fn sync_board(&mut self) {
        let player = self.engine.player();
        let player_xy = Some((player.col as u32, player.row as u32));
        let box_positions: HashSet<(u32, u32)> = self
            .engine
            .boxes()
            .iter()
            .map(|pos| (pos.col as u32, pos.row as u32))
            .collect();
        self.board = self.presenter.render_board(
            player_xy,
            &box_positions,
            self.selected_box,
            self.engine.is_level_solved(),
        );
    }
}
