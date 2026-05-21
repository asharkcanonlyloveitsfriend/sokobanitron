//! Shared gameplay presentation state.
//!
//! This module owns the currently displayed gameplay scene at the presentation layer. It stores
//! the latest gameplay scene and delegates drawing to the shared gameplay renderer.

mod damage;

use crate::gameplay_animation::{GameplayAnimationPolicy, GameplayAnimationRunner};
use crate::layout::ScreenRect;
use crate::renderer::Renderer;
use crate::screen_refresh_flash::ScreenRefreshFlash;
use crate::screen_requests::{GameplayPresentationUpdate, GameplayScreenRequest};
use sokobanitron_gameplay::BoardCell;
use std::time::Instant;

use self::damage::{
    add_optional_cell, gameplay_damage, merge_damage, normalize_cells, restart_damage,
};

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
    animation_runner: GameplayAnimationRunner,
    screen_refresh_flash: Option<ScreenRefreshFlash>,
    pending_screen_refresh_flash: bool,
    gameplay_frame_obscured_by_overlay: bool,
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
            animation_runner: GameplayAnimationRunner::default(),
            screen_refresh_flash: None,
            pending_screen_refresh_flash: false,
            gameplay_frame_obscured_by_overlay: false,
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
        let previous_scene = self.current_scene.clone();
        let previous_scene_ref = previous_scene.as_ref();
        let start_screen_refresh_flash = if !suspend_presentation_effects
            && matches!(self.animation_policy, GameplayAnimationPolicy::Full)
        {
            update_starts_screen_refresh_flash(previous_scene_ref, &update)
        } else {
            false
        };
        let scene_unchanged = previous_scene_ref == Some(&update.scene);
        let mut damage = gameplay_damage(previous_scene_ref, &update.scene);
        if !suspend_presentation_effects {
            if scene_unchanged {
                damage = merge_damage(damage, self.advance_ready_presentation_at(now));
            } else {
                damage = merge_damage(
                    damage,
                    self.clear_animation_damage(update.scene.board.player()),
                );
            }
        }
        damage = merge_damage(damage, restart_damage(previous_scene_ref, &update));
        self.current_scene = Some(update.scene.clone());
        if start_screen_refresh_flash {
            self.restart_screen_refresh_flash();
            return GameplayPresentationResult::new(GameplayDamage::Full, true);
        }
        self.screen_refresh_flash = None;
        self.pending_screen_refresh_flash = false;
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
        self.gameplay_frame_obscured_by_overlay = false;
        self.clear_transient_presentation();
    }

    pub fn mark_gameplay_frame_obscured_by_overlay(&mut self) {
        self.gameplay_frame_obscured_by_overlay = true;
    }

    pub fn take_gameplay_frame_obscured_by_overlay(&mut self) -> bool {
        std::mem::take(&mut self.gameplay_frame_obscured_by_overlay)
    }

    pub fn clear_transient_presentation(&mut self) {
        self.animation_runner.clear();
        self.screen_refresh_flash = None;
        self.pending_screen_refresh_flash = false;
    }

    pub fn has_pending_presentation(&self) -> bool {
        self.pending_screen_refresh_flash
            || self.screen_refresh_flash.is_some()
            || self.animation_runner.has_active_animation()
    }

    pub fn mark_pending_frame_presented_at(&mut self, now: Instant) {
        if let Some(screen_refresh_flash) = self.screen_refresh_flash.as_mut() {
            screen_refresh_flash.mark_initial_frame_presented_at(now);
        }
        self.animation_runner.mark_initial_frame_presented_at(now);
    }

    pub fn has_active_screen_refresh_flash(&self) -> bool {
        self.pending_screen_refresh_flash || self.screen_refresh_flash.is_some()
    }

    pub fn dismiss_screen_refresh_flash(&mut self) -> bool {
        let had_pending = std::mem::take(&mut self.pending_screen_refresh_flash);
        let had_active = self.screen_refresh_flash.take().is_some();
        had_pending || had_active
    }

    pub fn queue_screen_refresh_flash(&mut self) {
        if !matches!(self.animation_policy, GameplayAnimationPolicy::Full) {
            return;
        }
        if !self.pending_screen_refresh_flash && self.screen_refresh_flash.is_none() {
            self.restart_screen_refresh_flash();
        }
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
        self.start_pending_screen_refresh_flash(renderer, frame.len(), width, height, now);
        if let Some(screen_refresh_flash) = self.screen_refresh_flash.as_mut() {
            if screen_refresh_flash.draw_and_step(frame, now) {
                self.screen_refresh_flash = None;
            }
            return;
        }
        let _ = self.advance_ready_presentation_at(now);
        let scene = self
            .current_scene
            .as_ref()
            .expect("current scene checked before presentation advance");
        renderer.draw_gameplay_scene_with_animation(
            frame,
            width,
            height,
            scene,
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
        if self.pending_screen_refresh_flash {
            return GameplayPresentationResult::new(GameplayDamage::Full, true);
        }
        if let Some(screen_refresh_flash) = self.screen_refresh_flash.as_ref() {
            let changed = screen_refresh_flash.is_ready_to_draw(now);
            return GameplayPresentationResult::new(
                if changed {
                    GameplayDamage::Full
                } else {
                    GameplayDamage::Cells(Vec::new())
                },
                self.has_pending_presentation(),
            );
        }
        let dirty = self.advance_ready_presentation_at(now);
        self.presentation_result(GameplayDamage::Cells(dirty))
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

    fn restart_screen_refresh_flash(&mut self) {
        self.animation_runner.clear();
        self.screen_refresh_flash = None;
        self.pending_screen_refresh_flash = true;
    }

    fn start_pending_screen_refresh_flash(
        &mut self,
        renderer: &mut Renderer,
        frame_len: usize,
        width: u32,
        height: u32,
        now: Instant,
    ) {
        if !std::mem::take(&mut self.pending_screen_refresh_flash) {
            return;
        }
        let Some(scene) = self.current_scene.as_ref() else {
            return;
        };
        let mut target_frame = vec![0; frame_len];
        renderer.draw_gameplay_scene_with_animation(
            &mut target_frame,
            width,
            height,
            scene,
            &GameplayAnimationRunner::default(),
        );
        self.screen_refresh_flash = Some(ScreenRefreshFlash::new(target_frame, now));
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

fn update_starts_screen_refresh_flash(
    previous_scene: Option<&GameplayScreenRequest>,
    update: &GameplayPresentationUpdate,
) -> bool {
    let Some(previous_scene) = previous_scene else {
        return false;
    };
    if !matches!(
        update.cause,
        crate::screen_requests::GameplayPresentationCause::LevelTransition
    ) {
        return false;
    }
    if previous_scene.mode != update.scene.mode {
        return false;
    }
    true
}

#[cfg(test)]
mod tests;
