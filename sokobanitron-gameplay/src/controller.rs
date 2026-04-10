use crate::presenter::BoardView;
use crate::session::{GameplayEvent, GameplayKey, GameplaySession};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameplayControllerChanges {
    pub level_changed: Option<usize>,
    pub resume_level_changed: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayTapEffect {
    None,
    SelectionChanged { selected_box: Option<(u32, u32)> },
    PlayerMoved { to_x: u32, to_y: u32 },
    BoxMoved { path: Vec<(u32, u32)> },
    BoxRemoved { to_x: u32, to_y: u32 },
    MoveRejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayTapOutcome {
    pub changes: GameplayControllerChanges,
    pub effect: GameplayTapEffect,
    pub became_solved: bool,
    pub dirty_solution: bool,
    pub started_now: bool,
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
            resume_level_changed: None,
        }
    }

    pub fn advance_after_win(&mut self, target_level: usize) -> GameplayControllerChanges {
        let mut changes = self.jump_to_level(target_level);
        if let Some(index) = self.set_resume_level_to_current_if_needed() {
            changes.resume_level_changed = Some(index);
        }
        changes
    }

    pub fn click_cell_with_outcome(&mut self, x: u32, y: u32) -> GameplayTapOutcome {
        let was_started = self.session.is_started();
        let was_solved = self.session.board().is_solved();
        let session_events = self.session.click_cell_with_events(x, y);
        let effect = classify_tap_effect(&session_events);
        let started_now = !was_started && self.session.is_started();
        let is_solved = self.session.board().is_solved();
        let became_solved = !was_solved && is_solved;
        let dirty_solution = became_solved && !self.session.is_clean_solution();

        let mut changes = GameplayControllerChanges::default();
        if started_now && let Some(index) = self.set_resume_level_to_current_if_needed() {
            changes.resume_level_changed = Some(index);
        }

        GameplayTapOutcome {
            changes,
            effect,
            became_solved,
            dirty_solution,
            started_now,
        }
    }

    pub fn on_key_with_changes(&mut self, key: GameplayKey) -> GameplayControllerChanges {
        let _ = self.session.on_key_with_events(key);
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

    pub fn last_box_move_destination(&self) -> Option<(u32, u32)> {
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

fn classify_tap_effect(events: &[GameplayEvent]) -> GameplayTapEffect {
    if events
        .iter()
        .any(|event| matches!(event, GameplayEvent::MoveRejected))
    {
        return GameplayTapEffect::MoveRejected;
    }
    if let Some((to_x, to_y)) = events.iter().find_map(|event| match event {
        GameplayEvent::BoxRemoved { to_x, to_y } => Some((*to_x, *to_y)),
        _ => None,
    }) {
        return GameplayTapEffect::BoxRemoved { to_x, to_y };
    }
    if let Some(path) = events.iter().find_map(|event| match event {
        GameplayEvent::BoxMoved { path } => Some(path.clone()),
        _ => None,
    }) {
        return GameplayTapEffect::BoxMoved { path };
    }
    if let Some((to_x, to_y)) = events.iter().find_map(|event| match event {
        GameplayEvent::PlayerMoved { to_x, to_y } => Some((*to_x, *to_y)),
        _ => None,
    }) {
        return GameplayTapEffect::PlayerMoved { to_x, to_y };
    }
    if let Some(selected_box) = events.iter().find_map(|event| match event {
        GameplayEvent::SelectionChanged { selected_box } => Some(*selected_box),
        _ => None,
    }) {
        return GameplayTapEffect::SelectionChanged { selected_box };
    }
    GameplayTapEffect::None
}
