use crate::presenter::BoardView;
use crate::session::{GameplayEvent, GameplayKey, GameplaySession};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GameplayControllerChanges {
    pub level_changed: Option<usize>,
    pub last_attempted_level_changed: Option<usize>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayPresentMode {
    Full,
    FastPartial,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayTapPresentationStep {
    Render {
        box_trail: Option<Vec<(u32, u32)>>,
        draw_player: bool,
        show_win_overlay: bool,
        present_mode: GameplayPresentMode,
    },
    AnimatePlayerBlink,
    AnimateBoxVanish {
        to_x: u32,
        to_y: u32,
        show_win_overlay: bool,
    },
    AnimateBoxPathDisappear {
        path: Vec<(u32, u32)>,
        show_win_overlay: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayTapPresentationPlan {
    pub steps: Vec<GameplayTapPresentationStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxRemovedPresentation {
    VanishThenBlink,
    RenderThenBlink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxMovedTrailPresentation {
    FlashThenHide,
    AnimatePathDisappear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayTapPresentationStyle {
    pub box_removed_presentation: BoxRemovedPresentation,
    pub long_box_path_presentation: BoxMovedTrailPresentation,
    pub delayed_win_present_mode: GameplayPresentMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayTapOutcome {
    pub changes: GameplayControllerChanges,
    pub effect: GameplayTapEffect,
    pub delay_win_overlay: bool,
    pub dirty_win: bool,
    pub started_now: bool,
}

pub fn build_tap_presentation_plan(
    tap_outcome: &GameplayTapOutcome,
    show_box_path: bool,
    style: GameplayTapPresentationStyle,
) -> GameplayTapPresentationPlan {
    let mut steps = Vec::new();

    if matches!(tap_outcome.effect, GameplayTapEffect::MoveRejected) {
        steps.push(GameplayTapPresentationStep::AnimatePlayerBlink);
        return GameplayTapPresentationPlan { steps };
    }

    if let GameplayTapEffect::BoxRemoved { to_x, to_y } = tap_outcome.effect {
        match style.box_removed_presentation {
            BoxRemovedPresentation::VanishThenBlink => {
                steps.push(GameplayTapPresentationStep::AnimateBoxVanish {
                    to_x,
                    to_y,
                    show_win_overlay: !tap_outcome.delay_win_overlay,
                });
            }
            BoxRemovedPresentation::RenderThenBlink => {
                steps.push(GameplayTapPresentationStep::Render {
                    box_trail: None,
                    draw_player: true,
                    show_win_overlay: !tap_outcome.delay_win_overlay,
                    present_mode: GameplayPresentMode::Full,
                });
            }
        }
        steps.push(GameplayTapPresentationStep::AnimatePlayerBlink);
        return GameplayTapPresentationPlan { steps };
    }

    if show_box_path
        && let GameplayTapEffect::BoxMoved { path } = &tap_outcome.effect
        && path.len() > 2
    {
        match style.long_box_path_presentation {
            BoxMovedTrailPresentation::FlashThenHide => {
                steps.push(GameplayTapPresentationStep::Render {
                    box_trail: Some(path.clone()),
                    draw_player: false,
                    show_win_overlay: !tap_outcome.delay_win_overlay,
                    present_mode: GameplayPresentMode::Full,
                });
                if tap_outcome.dirty_win {
                    steps.push(GameplayTapPresentationStep::AnimatePlayerBlink);
                } else {
                    steps.push(GameplayTapPresentationStep::Render {
                        box_trail: None,
                        draw_player: true,
                        show_win_overlay: true,
                        present_mode: style.delayed_win_present_mode,
                    });
                }
            }
            BoxMovedTrailPresentation::AnimatePathDisappear => {
                steps.push(GameplayTapPresentationStep::AnimateBoxPathDisappear {
                    path: path.clone(),
                    show_win_overlay: !tap_outcome.delay_win_overlay,
                });
                if tap_outcome.dirty_win {
                    steps.push(GameplayTapPresentationStep::AnimatePlayerBlink);
                } else if tap_outcome.delay_win_overlay {
                    steps.push(GameplayTapPresentationStep::Render {
                        box_trail: None,
                        draw_player: true,
                        show_win_overlay: true,
                        present_mode: style.delayed_win_present_mode,
                    });
                }
            }
        }
        return GameplayTapPresentationPlan { steps };
    }

    if !matches!(tap_outcome.effect, GameplayTapEffect::None) {
        steps.push(GameplayTapPresentationStep::Render {
            box_trail: None,
            draw_player: true,
            show_win_overlay: !tap_outcome.delay_win_overlay,
            present_mode: GameplayPresentMode::Full,
        });
    }
    if tap_outcome.dirty_win {
        steps.push(GameplayTapPresentationStep::AnimatePlayerBlink);
    } else if tap_outcome.delay_win_overlay {
        steps.push(GameplayTapPresentationStep::Render {
            box_trail: None,
            draw_player: true,
            show_win_overlay: true,
            present_mode: style.delayed_win_present_mode,
        });
    }

    GameplayTapPresentationPlan { steps }
}

pub struct GameplayController {
    levels: Vec<String>,
    current_level: usize,
    last_attempted_level: Option<usize>,
    session: GameplaySession,
}

impl GameplayController {
    pub fn new(levels: Vec<String>, last_attempted_level: Option<usize>) -> Self {
        assert!(!levels.is_empty(), "levels must not be empty");
        let current_level = last_attempted_level
            .filter(|idx| *idx < levels.len())
            .unwrap_or(0);
        let session = GameplaySession::from_level_ascii(levels[current_level].clone());
        Self {
            levels,
            current_level,
            last_attempted_level,
            session,
        }
    }

    pub fn board(&self) -> &BoardView {
        self.session.board()
    }

    pub fn peek_level(&self, delta: i32) -> Option<usize> {
        if self.levels.is_empty() {
            return None;
        }
        let len = self.levels.len() as i32;
        let next = (self.current_level as i32 + delta).rem_euclid(len);
        Some(next as usize)
    }

    fn jump_to_level(&mut self, index: usize) -> GameplayControllerChanges {
        if self.levels.is_empty() {
            return GameplayControllerChanges::default();
        }
        let clamped = index.min(self.levels.len().saturating_sub(1));
        self.current_level = clamped;
        self.session = GameplaySession::from_level_ascii(self.levels[self.current_level].clone());
        GameplayControllerChanges {
            level_changed: Some(self.current_level),
            last_attempted_level_changed: None,
        }
    }

    pub fn advance_after_win(&mut self, target_level: usize) -> GameplayControllerChanges {
        let mut changes = self.jump_to_level(target_level);
        if let Some(index) = self.set_last_attempted_to_current_if_needed() {
            changes.last_attempted_level_changed = Some(index);
        }
        changes
    }

    pub fn click_cell_with_outcome(&mut self, x: u32, y: u32) -> GameplayTapOutcome {
        let was_started = self.session.is_started();
        let was_won = self.session.board().is_won();
        let session_events = self.session.click_cell_with_events(x, y);
        let effect = classify_tap_effect(&session_events);
        let started_now = !was_started && self.session.is_started();
        let is_won = self.session.board().is_won();
        let delay_win_overlay = !was_won && is_won;
        let dirty_win = delay_win_overlay && !self.session.is_clean_solution();

        let mut changes = GameplayControllerChanges::default();
        if started_now && let Some(index) = self.set_last_attempted_to_current_if_needed() {
            changes.last_attempted_level_changed = Some(index);
        }

        GameplayTapOutcome {
            changes,
            effect,
            delay_win_overlay,
            dirty_win,
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

    fn set_last_attempted_to_current_if_needed(&mut self) -> Option<usize> {
        if self.last_attempted_level == Some(self.current_level) {
            return None;
        }
        self.last_attempted_level = Some(self.current_level);
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

#[cfg(test)]
mod tests {
    use super::{
        BoxMovedTrailPresentation, BoxRemovedPresentation, GameplayControllerChanges,
        GameplayPresentMode, GameplayTapEffect, GameplayTapOutcome, GameplayTapPresentationStep,
        GameplayTapPresentationStyle, build_tap_presentation_plan,
    };

    const EINK_STYLE: GameplayTapPresentationStyle = GameplayTapPresentationStyle {
        box_removed_presentation: BoxRemovedPresentation::VanishThenBlink,
        long_box_path_presentation: BoxMovedTrailPresentation::FlashThenHide,
        delayed_win_present_mode: GameplayPresentMode::FastPartial,
    };

    const DESKTOP_STYLE: GameplayTapPresentationStyle = GameplayTapPresentationStyle {
        box_removed_presentation: BoxRemovedPresentation::RenderThenBlink,
        long_box_path_presentation: BoxMovedTrailPresentation::AnimatePathDisappear,
        delayed_win_present_mode: GameplayPresentMode::Full,
    };

    #[test]
    fn move_rejected_plan_blinks_only() {
        let outcome = GameplayTapOutcome {
            changes: GameplayControllerChanges {
                level_changed: None,
                last_attempted_level_changed: Some(3),
            },
            effect: GameplayTapEffect::MoveRejected,
            delay_win_overlay: false,
            dirty_win: false,
            started_now: false,
        };

        let plan = build_tap_presentation_plan(&outcome, true, EINK_STYLE);
        assert_eq!(
            plan.steps,
            vec![GameplayTapPresentationStep::AnimatePlayerBlink]
        );
    }

    #[test]
    fn long_box_path_plan_hides_player_then_finalizes() {
        let outcome = GameplayTapOutcome {
            changes: GameplayControllerChanges::default(),
            effect: GameplayTapEffect::BoxMoved {
                path: vec![(1, 1), (2, 1), (3, 1), (4, 1)],
            },
            delay_win_overlay: false,
            dirty_win: false,
            started_now: true,
        };

        let plan = build_tap_presentation_plan(&outcome, true, EINK_STYLE);
        assert_eq!(plan.steps.len(), 2);
        assert!(matches!(
            plan.steps[0],
            GameplayTapPresentationStep::Render {
                draw_player: false,
                present_mode: GameplayPresentMode::Full,
                ..
            }
        ));
        assert!(matches!(
            plan.steps[1],
            GameplayTapPresentationStep::Render {
                draw_player: true,
                present_mode: GameplayPresentMode::FastPartial,
                ..
            }
        ));
    }

    #[test]
    fn desktop_long_box_path_plan_uses_disappear_animation() {
        let outcome = GameplayTapOutcome {
            changes: GameplayControllerChanges::default(),
            effect: GameplayTapEffect::BoxMoved {
                path: vec![(1, 1), (2, 1), (3, 1), (4, 1)],
            },
            delay_win_overlay: false,
            dirty_win: false,
            started_now: true,
        };

        let plan = build_tap_presentation_plan(&outcome, true, DESKTOP_STYLE);
        assert_eq!(plan.steps.len(), 1);
        assert!(matches!(
            plan.steps[0],
            GameplayTapPresentationStep::AnimateBoxPathDisappear { .. }
        ));
    }

    #[test]
    fn desktop_long_box_path_plan_without_path_renders_immediately() {
        let outcome = GameplayTapOutcome {
            changes: GameplayControllerChanges::default(),
            effect: GameplayTapEffect::BoxMoved {
                path: vec![(1, 1), (2, 1), (3, 1), (4, 1)],
            },
            delay_win_overlay: false,
            dirty_win: false,
            started_now: true,
        };

        let plan = build_tap_presentation_plan(&outcome, false, DESKTOP_STYLE);
        assert_eq!(plan.steps.len(), 1);
        assert!(matches!(
            plan.steps[0],
            GameplayTapPresentationStep::Render {
                box_trail: None,
                draw_player: true,
                present_mode: GameplayPresentMode::Full,
                ..
            }
        ));
    }
}
