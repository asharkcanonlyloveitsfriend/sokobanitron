use crate::renderer::EntityVisualStyle;
use crate::screen_requests::{GameplayPresentationCause, GameplayPresentationUpdate};
use sokobanitron_gameplay::BoardCell;
use std::time::Instant;

use super::GameplayPresentationState;
use super::damage::{add_optional_cell, normalize_cells};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum GameplayVisualEffect {
    #[default]
    None,
    PuzzleSolvedClean,
    PuzzleSolvedDirty,
}

impl GameplayVisualEffect {
    pub(super) fn entity_visual_style(self) -> EntityVisualStyle {
        match self {
            Self::None => EntityVisualStyle::Standard,
            Self::PuzzleSolvedClean => EntityVisualStyle::SolvedClean,
            Self::PuzzleSolvedDirty => EntityVisualStyle::SolvedDirty,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QueuedGameplayEffect {
    PuzzleSolved { clean: bool },
}

pub(super) fn queued_effect_for_update(
    update: &GameplayPresentationUpdate,
) -> Option<QueuedGameplayEffect> {
    match update.cause {
        GameplayPresentationCause::PuzzleSolved { clean } => {
            Some(QueuedGameplayEffect::PuzzleSolved { clean })
        }
        _ => None,
    }
}

impl GameplayPresentationState {
    pub(super) fn apply_ready_effects(&mut self, now: Instant) -> Vec<BoardCell> {
        let mut dirty = Vec::new();
        while !self.animation_runner.has_active_animation() {
            let Some(effect) = self.pending_effects.pop_front() else {
                break;
            };
            dirty.extend(self.apply_effect(effect, now));
        }
        normalize_cells(dirty)
    }

    fn apply_effect(&mut self, effect: QueuedGameplayEffect, now: Instant) -> Vec<BoardCell> {
        let scene = self
            .current_scene
            .as_ref()
            .expect("queued gameplay effect requires a current scene");
        match effect {
            QueuedGameplayEffect::PuzzleSolved { clean } => {
                self.visual_effect = if clean {
                    GameplayVisualEffect::PuzzleSolvedClean
                } else {
                    GameplayVisualEffect::PuzzleSolvedDirty
                };
                let mut dirty: Vec<BoardCell> = scene
                    .board
                    .cells()
                    .filter(|&cell| scene.board.has_box(cell))
                    .collect();
                if clean {
                    add_optional_cell(&mut dirty, scene.board.player());
                } else if let Some(player) = scene.board.player() {
                    self.animation_runner.enqueue_blink(player, now);
                }
                normalize_cells(dirty)
            }
        }
    }
}
