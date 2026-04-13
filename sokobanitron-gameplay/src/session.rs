use crate::board_cell::BoardCell;
use crate::engine::GameEngine;
use crate::level::parse_level_ascii;
use crate::presenter::{BoardView, GameBoardPresenter, TileKind};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameplayKey {
    Escape,
    Backspace,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayEvent {
    SelectionChanged { selected_box: Option<BoardCell> },
    PlayerMoved { to: BoardCell },
    BoxMoved { path: Vec<BoardCell> },
    BoxRemoved { to: BoardCell },
    BoxMoveRejected,
    UndoApplied,
    Restarted,
    LevelSolved { clean: bool },
}

pub struct GameplaySession {
    level_ascii: String,
    presenter: GameBoardPresenter,
    engine: GameEngine,
    selected_box: Option<BoardCell>,
    board: BoardView,
}

impl GameplaySession {
    pub fn from_level_ascii(level_ascii: String) -> Self {
        let parsed = parse_level_ascii(&level_ascii);
        let presenter = GameBoardPresenter::new(parsed);
        let engine = GameEngine::from_ascii(&level_ascii).expect("level ascii must parse");

        let player = Some(engine.player());
        let box_positions: HashSet<BoardCell> = engine.boxes().collect();
        let board = presenter.render_board(player, &box_positions, None, engine.is_level_solved());

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

    pub fn can_restart(&self) -> bool {
        self.engine.can_restart()
    }

    pub fn can_undo(&self) -> bool {
        self.engine.can_undo()
    }

    pub fn is_clean_solution(&self) -> bool {
        self.engine.is_clean_solution()
    }

    pub fn box_move_history(&self) -> Vec<Vec<(usize, usize)>> {
        self.engine
            .box_move_history_cells()
            .into_iter()
            .map(|path| {
                path.into_iter()
                    .map(|cell| (cell.y as usize, cell.x as usize))
                    .collect()
            })
            .collect()
    }

    pub fn last_box_move_destination(&self) -> Option<BoardCell> {
        self.engine.last_box_move_destination()
    }

    pub fn click_cell_with_events(&mut self, clicked_cell: BoardCell) -> Vec<GameplayEvent> {
        let mut events = Vec::new();
        if self.board.is_solved() {
            return events;
        }
        let was_solved = self.board.is_solved();

        if self.engine.has_box(clicked_cell) {
            self.selected_box = if self.selected_box == Some(clicked_cell) {
                None
            } else {
                Some(clicked_cell)
            };
            self.sync_board();
            events.push(GameplayEvent::SelectionChanged {
                selected_box: self.selected_box,
            });
            return events;
        }

        if let Some(from_cell) = self.selected_box.take() {
            let outcome = if self.board.tile(clicked_cell) == TileKind::Void {
                self.engine
                    .push_box_into_void(from_cell, clicked_cell)
                    .then_some(GameplayEvent::BoxRemoved { to: clicked_cell })
            } else {
                self.engine
                    .move_box_to(from_cell, clicked_cell)
                    .map(|path| GameplayEvent::BoxMoved { path })
            };

            self.sync_board();

            if let Some(event) = outcome {
                events.push(event);
                self.push_solved_event_if_needed(was_solved, &mut events);
            } else {
                events.push(GameplayEvent::BoxMoveRejected);
            }
            return events;
        }

        if self.engine.player() == clicked_cell {
            return events;
        }

        if self.engine.move_player_to(clicked_cell) {
            self.sync_board();
            events.push(GameplayEvent::PlayerMoved { to: clicked_cell });
            return events;
        }
        events
    }

    pub fn on_key_with_events(&mut self, key: GameplayKey) -> Vec<GameplayEvent> {
        let mut events = Vec::new();
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
        if !was_solved && self.board.is_solved() {
            events.push(GameplayEvent::LevelSolved {
                clean: self.engine.is_clean_solution(),
            });
        }
    }

    fn sync_board(&mut self) {
        let player = Some(self.engine.player());
        let box_positions: HashSet<BoardCell> = self.engine.boxes().collect();
        self.board = self.presenter.render_board(
            player,
            &box_positions,
            self.selected_box,
            self.engine.is_level_solved(),
        );
    }
}
