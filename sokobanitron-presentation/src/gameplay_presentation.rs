//! Shared gameplay presentation state.
//!
//! This module owns the currently displayed gameplay scene at the presentation layer. It stores
//! the latest gameplay scene and delegates drawing to the shared gameplay renderer.

mod damage;
mod effects;

use crate::gameplay_animation::{GameplayAnimationPolicy, GameplayAnimationRunner};
use crate::layout::ScreenRect;
use crate::renderer::Renderer;
use crate::screen_requests::{GameplayPresentationUpdate, GameplayScreenRequest};
use sokobanitron_gameplay::BoardCell;
use std::collections::VecDeque;
use std::time::Instant;

use self::damage::{
    add_optional_cell, gameplay_board_state_changed, gameplay_damage, merge_damage,
    normalize_cells, restart_damage,
};
use self::effects::{GameplayVisualEffect, QueuedGameplayEffect, queued_effect_for_update};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayDamage {
    Full,
    Cells(Vec<BoardCell>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameplayPresentationResult {
    /// Damage the client should draw immediately for this presentation step.
    pub damage: GameplayDamage,
    /// Whether more timed presentation work remains after drawing this step.
    pub has_pending_presentation: bool,
}

impl GameplayPresentationResult {
    fn new(damage: GameplayDamage, has_pending_presentation: bool) -> Self {
        Self {
            damage,
            has_pending_presentation,
        }
    }
}

pub fn gameplay_damage_union_rect(
    scene: &GameplayScreenRequest,
    damage: &GameplayDamage,
    surface_width: u32,
    surface_height: u32,
) -> Option<ScreenRect> {
    damage::gameplay_damage_union_rect(scene, damage, surface_width, surface_height)
}

/// Shared gameplay presentation orchestrator.
///
/// Client redraw contract:
/// - call [`Self::replace_update_with_damage`] when app/gameplay produces a new scene update
/// - draw the returned [`GameplayPresentationResult::damage`] immediately
/// - if [`GameplayPresentationResult::has_pending_presentation`] is `true`, schedule another
///   gameplay redraw
/// - on that redraw, call [`Self::advance_presentation_with_damage`], draw the returned damage,
///   and repeat while pending presentation remains
///
/// [`Self::draw`] is the full-frame convenience path. Unlike [`Self::draw_damage`], it may
/// advance any ready presentation work before drawing the scene.
pub struct GameplayPresentationState {
    animation_policy: GameplayAnimationPolicy,
    current_scene: Option<GameplayScreenRequest>,
    pending_effects: VecDeque<QueuedGameplayEffect>,
    visual_effect: GameplayVisualEffect,
    animation_runner: GameplayAnimationRunner,
}

impl Default for GameplayPresentationState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameplayPresentationState {
    pub fn new() -> Self {
        Self::with_animation_policy(GameplayAnimationPolicy::Full)
    }

    pub fn with_animation_policy(animation_policy: GameplayAnimationPolicy) -> Self {
        Self {
            animation_policy,
            current_scene: None,
            pending_effects: VecDeque::new(),
            visual_effect: GameplayVisualEffect::default(),
            animation_runner: GameplayAnimationRunner::default(),
        }
    }

    pub fn replace_update(&mut self, update: GameplayPresentationUpdate) {
        let _ = self.replace_update_at(update, Instant::now());
    }

    pub fn replace_update_with_damage(
        &mut self,
        update: GameplayPresentationUpdate,
    ) -> GameplayPresentationResult {
        self.replace_update_at(update, Instant::now())
    }

    pub(crate) fn replace_update_at(
        &mut self,
        update: GameplayPresentationUpdate,
        now: Instant,
    ) -> GameplayPresentationResult {
        self.replace_update_at_internal(update, now, false)
    }

    fn replace_update_at_internal(
        &mut self,
        update: GameplayPresentationUpdate,
        now: Instant,
        suspend_presentation_effects: bool,
    ) -> GameplayPresentationResult {
        let queued_effect = queued_effect_for_update(&update);
        let previous_scene = self.current_scene.clone();
        let previous_scene_ref = previous_scene.as_ref();
        let scene_unchanged = previous_scene_ref == Some(&update.scene);
        let mut damage = gameplay_damage(previous_scene_ref, &update.scene);
        if suspend_presentation_effects {
            if !scene_unchanged && gameplay_board_state_changed(previous_scene_ref, &update.scene) {
                self.pending_effects.clear();
                self.visual_effect = GameplayVisualEffect::default();
            }
        } else if scene_unchanged {
            damage = merge_damage(damage, self.advance_ready_presentation_at(now));
        } else {
            damage = merge_damage(
                damage,
                self.clear_animation_damage(update.scene.board.player()),
            );
            if gameplay_board_state_changed(previous_scene_ref, &update.scene) {
                self.pending_effects.clear();
                self.visual_effect = GameplayVisualEffect::default();
            }
        }
        damage = merge_damage(damage, restart_damage(&update));
        self.current_scene = Some(update.scene.clone());
        if suspend_presentation_effects {
            return self.presentation_result(damage);
        }
        let was_hiding_player = self.animation_runner.hides_player();
        let animation_enqueued = self.animation_runner.enqueue_for_update(
            previous_scene_ref,
            &update,
            self.animation_policy,
            now,
        );
        if let Some(effect) = queued_effect {
            self.pending_effects.push_back(effect);
        }
        if animation_enqueued {
            damage = merge_damage(
                damage,
                self.animation_damage_with_hidden_player(
                    self.animation_runner.current_dirty_cells(),
                    update.scene.board.player(),
                    was_hiding_player,
                ),
            );
        }
        damage = merge_damage(damage, self.apply_ready_effects(now));
        self.presentation_result(damage)
    }

    #[cfg(test)]
    pub(crate) fn replace_update_without_presentation_effects_at(
        &mut self,
        update: GameplayPresentationUpdate,
        now: Instant,
    ) -> GameplayPresentationResult {
        self.replace_update_at_internal(update, now, true)
    }

    pub fn current_scene(&self) -> Option<&GameplayScreenRequest> {
        self.current_scene.as_ref()
    }

    pub fn clear(&mut self) {
        self.current_scene = None;
        self.pending_effects.clear();
        self.visual_effect = GameplayVisualEffect::default();
        self.animation_runner.clear();
    }

    pub fn has_pending_presentation(&self) -> bool {
        self.animation_runner.has_active_animation() || !self.pending_effects.is_empty()
    }

    pub fn advance_presentation_with_damage(&mut self) -> GameplayPresentationResult {
        self.advance_presentation_with_damage_at(Instant::now())
    }

    pub fn draw(&mut self, renderer: &mut Renderer, frame: &mut [u8], width: u32, height: u32) {
        self.draw_at(renderer, frame, width, height, Instant::now());
    }

    pub fn draw_damage(
        &mut self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        damage: &GameplayDamage,
    ) {
        match damage {
            GameplayDamage::Full => self.draw(renderer, frame, width, height),
            GameplayDamage::Cells(cells) => {
                let scene = self
                    .current_scene
                    .as_ref()
                    .expect("cell damage draw requires a current gameplay scene");
                renderer.draw_gameplay_scene_cells(
                    frame,
                    width,
                    height,
                    scene,
                    cells,
                    self.visual_effect.entity_visual_style(),
                    &self.animation_runner,
                );
            }
        }
    }

    pub(crate) fn draw_at(
        &mut self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        now: Instant,
    ) {
        if self.current_scene.is_none() {
            return;
        }
        let _ = self.advance_ready_presentation_at(now);
        let _ = self.apply_ready_effects(now);
        let scene = self
            .current_scene
            .as_ref()
            .expect("current scene checked before presentation advance");
        renderer.draw_gameplay_scene_with_style_and_animation(
            frame,
            width,
            height,
            scene,
            self.visual_effect.entity_visual_style(),
            &self.animation_runner,
        );
    }

    pub(crate) fn advance_presentation_with_damage_at(
        &mut self,
        now: Instant,
    ) -> GameplayPresentationResult {
        if self.current_scene.is_none() {
            return GameplayPresentationResult::new(GameplayDamage::Cells(Vec::new()), false);
        }
        let mut damage = GameplayDamage::Cells(self.advance_ready_presentation_at(now));
        damage = merge_damage(damage, self.apply_ready_effects(now));
        self.presentation_result(damage)
    }

    fn advance_ready_presentation_at(&mut self, now: Instant) -> Vec<BoardCell> {
        let Some(player) = self
            .current_scene
            .as_ref()
            .and_then(|scene| scene.board.player())
        else {
            return self.animation_runner.advance_to_with_damage(now);
        };
        let was_hiding_player = self.animation_runner.hides_player();
        let dirty = self.animation_runner.advance_to_with_damage(now);
        self.animation_damage_with_hidden_player(dirty, Some(player), was_hiding_player)
    }

    fn clear_animation_damage(&mut self, player: Option<BoardCell>) -> Vec<BoardCell> {
        let was_hiding_player = self.animation_runner.hides_player();
        let dirty = self.animation_runner.clear_damage();
        self.animation_damage_with_hidden_player(dirty, player, was_hiding_player)
    }

    fn presentation_result(&self, damage: GameplayDamage) -> GameplayPresentationResult {
        GameplayPresentationResult::new(damage, self.has_pending_presentation())
    }

    fn animation_damage_with_hidden_player(
        &self,
        mut dirty: Vec<BoardCell>,
        player: Option<BoardCell>,
        was_hiding_player: bool,
    ) -> Vec<BoardCell> {
        if was_hiding_player != self.animation_runner.hides_player() {
            add_optional_cell(&mut dirty, player);
        }
        normalize_cells(dirty)
    }
}

#[cfg(test)]
mod tests;
