use crate::board_cell::BoardCell;
use crate::presenter::BoardView;
use crate::session::{
    GameplayKey, GameplayMoveDirection, GameplaySession, GameplaySessionTapOutcome,
    GameplayTapEffect, GameplayTapEvent,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameplayControllerChanges {
    pub level_changed: Option<usize>,
    pub resume_level_to_persist: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayTapOutcome {
    pub changes: GameplayControllerChanges,
    pub effect: GameplayTapEffect,
    pub event: GameplayTapEvent,
}

pub struct GameplayController {
    levels: Vec<String>,
    current_level: usize,
    resume_level: usize,
    resume_level_persisted: bool,
    session: GameplaySession,
}

impl GameplayController {
    pub fn new(levels: Vec<String>, resume_level: Option<usize>) -> Self {
        let start_level = resume_level.unwrap_or(0);
        Self::new_at_level(levels, start_level, resume_level)
    }

    pub fn new_at_level(
        levels: Vec<String>,
        start_level: usize,
        persisted_resume_level: Option<usize>,
    ) -> Self {
        assert!(!levels.is_empty(), "levels must not be empty");
        let current_level = Some(start_level)
            .filter(|idx| *idx < levels.len())
            .unwrap_or(0);
        let resume_level_persisted = persisted_resume_level.is_some();
        let resume_level = persisted_resume_level
            .filter(|idx| *idx < levels.len())
            .unwrap_or(current_level);
        let session = GameplaySession::from_level_ascii(levels[current_level].clone());
        Self {
            levels,
            current_level,
            resume_level,
            resume_level_persisted,
            session,
        }
    }

    pub fn board(&self) -> &BoardView {
        self.session.board()
    }

    pub fn current_level(&self) -> usize {
        self.current_level
    }

    pub fn resume_level(&self) -> usize {
        self.resume_level
    }

    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    pub fn can_restart(&self) -> bool {
        self.session.can_restart()
    }

    pub fn can_undo(&self) -> bool {
        self.session.can_undo()
    }

    pub fn peek_level(&self, delta: i32) -> Option<usize> {
        if self.levels.is_empty() {
            return None;
        }
        let len = self.levels.len() as i32;
        let next = (self.current_level as i32 + delta).rem_euclid(len);
        Some(next as usize)
    }

    pub fn jump_to_level(&mut self, index: usize) -> GameplayControllerChanges {
        if self.levels.is_empty() {
            return GameplayControllerChanges::default();
        }
        let clamped = index.min(self.levels.len().saturating_sub(1));
        self.current_level = clamped;
        self.session = GameplaySession::from_level_ascii(self.levels[self.current_level].clone());
        GameplayControllerChanges {
            level_changed: Some(self.current_level),
            resume_level_to_persist: None,
        }
    }

    pub fn advance_after_win(&mut self, target_level: usize) -> GameplayControllerChanges {
        let mut changes = self.jump_to_level(target_level);
        if let Some(index) = self.set_resume_level_to_current_if_needed() {
            changes.resume_level_to_persist = Some(index);
        }
        changes
    }

    pub fn click_cell_with_outcome(&mut self, cell: BoardCell) -> GameplayTapOutcome {
        let was_started = self.session.is_started();
        let session_outcome = self.session.click_cell(cell);
        self.session_outcome_with_changes(was_started, session_outcome)
    }

    pub fn move_direction_with_outcome(
        &mut self,
        direction: GameplayMoveDirection,
    ) -> GameplayTapOutcome {
        let was_started = self.session.is_started();
        let session_outcome = self.session.move_direction(direction);
        self.session_outcome_with_changes(was_started, session_outcome)
    }

    fn session_outcome_with_changes(
        &mut self,
        was_started: bool,
        session_outcome: GameplaySessionTapOutcome,
    ) -> GameplayTapOutcome {
        let puzzle_started = !was_started && self.session.is_started();

        let mut changes = GameplayControllerChanges::default();
        if puzzle_started {
            changes.resume_level_to_persist = self
                .set_resume_level_to_current_if_needed()
                .or(Some(self.current_level));
        }

        GameplayTapOutcome {
            changes,
            effect: session_outcome.effect,
            event: session_outcome.event,
        }
    }

    pub fn on_key_with_changes(&mut self, key: GameplayKey) -> GameplayControllerChanges {
        self.session.on_key(key);
        GameplayControllerChanges::default()
    }

    pub fn restart_with_changes(&mut self) -> GameplayControllerChanges {
        self.on_key_with_changes(GameplayKey::Escape)
    }

    pub fn undo_with_changes(&mut self) -> GameplayControllerChanges {
        self.on_key_with_changes(GameplayKey::Backspace)
    }

    pub fn solution_history(&self) -> Vec<Vec<(usize, usize)>> {
        self.session.box_move_history()
    }

    pub fn last_box_move_destination(&self) -> Option<BoardCell> {
        self.session.last_box_move_destination()
    }

    fn set_resume_level_to_current_if_needed(&mut self) -> Option<usize> {
        if self.resume_level_persisted && self.resume_level == self.current_level {
            return None;
        }
        self.resume_level = self.current_level;
        self.resume_level_persisted = true;
        Some(self.current_level)
    }
}

#[cfg(test)]
mod tests {
    use super::GameplayController;
    use crate::{BoardCell, GameplayMoveDirection, GameplayTapEffect, GameplayTapEvent};

    fn cell(x: u32, y: u32) -> BoardCell {
        BoardCell::new(x, y)
    }

    #[test]
    fn failed_player_move_is_a_noop_effect() {
        let level = "#####\n#@  #\n#####".to_string();
        let mut controller = GameplayController::new(vec![level], None);

        let outcome = controller.click_cell_with_outcome(cell(4, 1));

        assert_eq!(outcome.effect, GameplayTapEffect::None);
        assert_eq!(outcome.event, GameplayTapEvent::None);
    }

    #[test]
    fn invalid_selected_box_destination_is_box_move_rejected() {
        let level = "######\n#@$ ##\n######".to_string();
        let mut controller = GameplayController::new(vec![level], None);

        let select = controller.click_cell_with_outcome(cell(2, 1));
        assert_eq!(
            select.effect,
            GameplayTapEffect::SelectionChanged {
                selected_box: Some(cell(2, 1))
            }
        );

        let reject = controller.click_cell_with_outcome(cell(4, 1));

        assert_eq!(reject.effect, GameplayTapEffect::BoxMoveRejected);
        assert_eq!(reject.event, GameplayTapEvent::None);
    }

    #[test]
    fn first_meaningful_tap_updates_resume_level_without_emitting_event() {
        let level = "#######\n#@ $. #\n#######".to_string();
        let mut controller = GameplayController::new(vec![level], None);

        let outcome = controller.click_cell_with_outcome(cell(2, 1));

        assert_eq!(
            outcome.effect,
            GameplayTapEffect::PlayerMoved { to: cell(2, 1) }
        );
        assert_eq!(outcome.event, GameplayTapEvent::None);
        assert_eq!(outcome.changes.resume_level_to_persist, Some(0));
    }

    #[test]
    fn tap_that_starts_and_solves_only_emits_puzzle_solved_event() {
        let level = "#####\n#@$.#\n#####".to_string();
        let mut controller = GameplayController::new(vec![level], None);

        let select = controller.click_cell_with_outcome(cell(2, 1));
        let solved = controller.click_cell_with_outcome(cell(3, 1));

        assert_eq!(select.event, GameplayTapEvent::None);
        assert_eq!(
            solved.effect,
            GameplayTapEffect::BoxMoved {
                path: vec![cell(2, 1), cell(3, 1)]
            }
        );
        assert_eq!(solved.event, GameplayTapEvent::PuzzleSolved { clean: true });
    }

    #[test]
    fn directional_move_moves_player_one_cell() {
        let level = "######\n#@ $.#\n######".to_string();
        let mut controller = GameplayController::new(vec![level], None);

        let outcome = controller.move_direction_with_outcome(GameplayMoveDirection::Right);

        assert_eq!(
            outcome.effect,
            GameplayTapEffect::PlayerMoved { to: cell(2, 1) }
        );
        assert_eq!(controller.board().player(), Some(cell(2, 1)));
        assert_eq!(outcome.changes.resume_level_to_persist, Some(0));
    }

    #[test]
    fn directional_move_pushes_adjacent_box() {
        let level = "######\n# @$ #\n#    #\n######".to_string();
        let mut controller = GameplayController::new(vec![level], None);

        let outcome = controller.move_direction_with_outcome(GameplayMoveDirection::Right);

        assert_eq!(
            outcome.effect,
            GameplayTapEffect::BoxMoved {
                path: vec![cell(3, 1), cell(4, 1)]
            }
        );
        assert_eq!(controller.board().player(), Some(cell(3, 1)));
        assert!(controller.board().has_box(cell(4, 1)));
    }
}
