use crate::engine::GameEngine;
use crate::level::parse_level_ascii;
use crate::presenter::{BoardView, GameBoardPresenter};
use sokobanitron_core::pathfinder::Position;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameplayKey {
    Escape,
    Backspace,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayEvent {
    SelectionChanged { selected_box: Option<(u32, u32)> },
    PlayerMoved { to_x: u32, to_y: u32 },
    BoxMoved { path: Vec<(u32, u32)> },
    BoxRemoved { to_x: u32, to_y: u32 },
    MoveRejected,
    UndoApplied,
    Restarted,
    LevelSolved { clean: bool },
}

pub struct GameplaySession {
    level_ascii: String,
    presenter: GameBoardPresenter,
    engine: GameEngine,
    selected_box: Option<(u32, u32)>,
    board: BoardView,
}

impl GameplaySession {
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
        let board =
            presenter.render_board(player_xy, &box_positions, None, engine.is_level_solved());

        Self {
            level_ascii,
            presenter,
            engine,
            selected_box: None,
            board,
        }
    }

    pub fn board(&self) -> &BoardView {
        &self.board
    }

    pub fn is_started(&self) -> bool {
        !self.engine.is_at_start()
    }

    pub fn is_clean_solution(&self) -> bool {
        self.engine.is_clean_solution()
    }

    pub fn click_cell_with_events(&mut self, x: u32, y: u32) -> Vec<GameplayEvent> {
        let mut events = Vec::new();
        if self.board.is_won() {
            return events;
        }
        let was_solved = self.board.is_won();

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
            events.push(GameplayEvent::SelectionChanged {
                selected_box: self.selected_box,
            });
            return events;
        }

        if let Some((from_x, from_y)) = self.selected_box {
            if self.board.tile(x, y) == crate::presenter::TileKind::Void {
                let removed = self.engine.push_box_into_void(
                    Position::new(from_y as usize, from_x as usize),
                    Position::new(y as usize, x as usize),
                );
                if removed {
                    self.selected_box = None;
                    self.sync_board();
                    events.push(GameplayEvent::BoxRemoved { to_x: x, to_y: y });
                    self.push_solved_event_if_needed(was_solved, &mut events);
                    return events;
                }
                self.selected_box = None;
                self.sync_board();
                events.push(GameplayEvent::MoveRejected);
                return events;
            }

            let moved_box = self.engine.move_box_to(
                Position::new(from_y as usize, from_x as usize),
                Position::new(y as usize, x as usize),
            );
            if let Some(path) = moved_box {
                self.selected_box = None;
                self.sync_board();
                events.push(GameplayEvent::BoxMoved {
                    path: path
                        .into_iter()
                        .map(|p| (p.col as u32, p.row as u32))
                        .collect(),
                });
                self.push_solved_event_if_needed(was_solved, &mut events);
                return events;
            }
            self.selected_box = None;
            self.sync_board();
            events.push(GameplayEvent::MoveRejected);
            return events;
        }

        if self
            .engine
            .move_player_to(Position::new(y as usize, x as usize))
        {
            self.sync_board();
            events.push(GameplayEvent::PlayerMoved { to_x: x, to_y: y });
            return events;
        }
        events.push(GameplayEvent::MoveRejected);
        events
    }

    pub fn on_key_with_events(&mut self, key: GameplayKey) -> Vec<GameplayEvent> {
        let mut events = Vec::new();
        if self.board.is_won() {
            if key == GameplayKey::Escape {
                self.restart();
                events.push(GameplayEvent::Restarted);
            }
            return events;
        }

        match key {
            GameplayKey::Backspace => {
                if self.engine.undo().is_some() {
                    self.selected_box = None;
                    self.sync_board();
                    events.push(GameplayEvent::UndoApplied);
                }
            }
            GameplayKey::Escape => {
                self.restart();
                events.push(GameplayEvent::Restarted);
            }
            GameplayKey::Other => {}
        }
        events
    }

    pub fn restart(&mut self) {
        self.engine = GameEngine::from_ascii(&self.level_ascii).expect("level ascii must parse");
        self.selected_box = None;
        self.sync_board();
    }

    fn push_solved_event_if_needed(&self, was_solved: bool, events: &mut Vec<GameplayEvent>) {
        if !was_solved && self.board.is_won() {
            events.push(GameplayEvent::LevelSolved {
                clean: self.engine.is_clean_solution(),
            });
        }
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
