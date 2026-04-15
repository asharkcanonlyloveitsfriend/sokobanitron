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
pub enum GameplayTapEffect {
    None,
    SelectionChanged { selected_box: Option<BoardCell> },
    PlayerMoved { to: BoardCell },
    BoxMoved { path: Vec<BoardCell> },
    BoxRemoved { to: BoardCell },
    BoxMoveRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameplayTapEvent {
    #[default]
    None,
    PuzzleSolved {
        clean: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GameplaySessionTapOutcome {
    pub effect: GameplayTapEffect,
    pub event: GameplayTapEvent,
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

    pub fn click_cell(&mut self, clicked_cell: BoardCell) -> GameplaySessionTapOutcome {
        if self.board.is_solved() {
            return GameplaySessionTapOutcome::default();
        }
        let was_solved = self.board.is_solved();

        if self.engine.has_box(clicked_cell) {
            self.selected_box = if self.selected_box == Some(clicked_cell) {
                None
            } else {
                Some(clicked_cell)
            };
            self.sync_board();
            return GameplaySessionTapOutcome::effect(GameplayTapEffect::SelectionChanged {
                selected_box: self.selected_box,
            });
        }

        if let Some(from_cell) = self.selected_box.take() {
            let effect = if self.board.tile(clicked_cell) == TileKind::Void {
                self.engine
                    .push_box_into_void(from_cell, clicked_cell)
                    .then_some(GameplayTapEffect::BoxRemoved { to: clicked_cell })
            } else {
                self.engine
                    .move_box_to(from_cell, clicked_cell)
                    .map(|path| GameplayTapEffect::BoxMoved { path })
            };

            self.sync_board();

            if let Some(effect) = effect {
                return GameplaySessionTapOutcome {
                    effect,
                    event: self.solved_event_if_needed(was_solved),
                };
            } else {
                return GameplaySessionTapOutcome::effect(GameplayTapEffect::BoxMoveRejected);
            }
        }

        if self.engine.player() == clicked_cell {
            return GameplaySessionTapOutcome::default();
        }

        if self.engine.move_player_to(clicked_cell) {
            self.sync_board();
            return GameplaySessionTapOutcome::effect(GameplayTapEffect::PlayerMoved {
                to: clicked_cell,
            });
        }
        GameplaySessionTapOutcome::default()
    }

    pub fn on_key(&mut self, key: GameplayKey) {
        match key {
            GameplayKey::Backspace => {
                if self.engine.undo().is_some() {
                    self.selected_box = None;
                    self.sync_board();
                }
            }
            GameplayKey::Escape => {
                self.restart();
            }
            GameplayKey::Other => {}
        }
    }

    pub fn restart(&mut self) {
        self.engine = GameEngine::from_ascii(&self.level_ascii).expect("level ascii must parse");
        self.selected_box = None;
        self.sync_board();
    }

    fn solved_event_if_needed(&self, was_solved: bool) -> GameplayTapEvent {
        if !was_solved && self.board.is_solved() {
            GameplayTapEvent::PuzzleSolved {
                clean: self.engine.is_clean_solution(),
            }
        } else {
            GameplayTapEvent::None
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

impl Default for GameplaySessionTapOutcome {
    fn default() -> Self {
        Self {
            effect: GameplayTapEffect::None,
            event: GameplayTapEvent::None,
        }
    }
}

impl GameplaySessionTapOutcome {
    fn effect(effect: GameplayTapEffect) -> Self {
        Self {
            effect,
            event: GameplayTapEvent::None,
        }
    }
}
