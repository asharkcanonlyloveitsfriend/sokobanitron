//! Shared gameplay presentation state.
//!
//! This module owns the currently displayed gameplay scene at the presentation layer. It stores
//! the latest gameplay scene and delegates drawing to the shared gameplay renderer.

use crate::gameplay_animation::{GameplayAnimationPolicy, GameplayAnimationRunner};
use crate::layout::ScreenRect;
use crate::renderer::{EntityVisualStyle, Renderer};
use crate::screen_requests::{
    GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenMode,
    GameplayScreenRequest,
};
use sokobanitron_gameplay::BoardCell;
use std::collections::VecDeque;
use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameplayDamage {
    Full,
    Cells(Vec<BoardCell>),
}

pub fn gameplay_damage_union_rect(
    scene: &GameplayScreenRequest,
    damage: &GameplayDamage,
    surface_width: u32,
    surface_height: u32,
) -> Option<ScreenRect> {
    match damage {
        GameplayDamage::Full => {
            if surface_width == 0 || surface_height == 0 {
                None
            } else {
                Some(ScreenRect {
                    x: 0,
                    y: 0,
                    w: surface_width,
                    h: surface_height,
                })
            }
        }
        GameplayDamage::Cells(cells) => {
            gameplay_cell_union_rect(scene, cells, surface_width, surface_height)
        }
    }
}

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
    ) -> GameplayDamage {
        self.replace_update_at(update, Instant::now())
    }

    pub(crate) fn replace_update_at(
        &mut self,
        update: GameplayPresentationUpdate,
        now: Instant,
    ) -> GameplayDamage {
        self.replace_update_at_internal(update, now, false)
    }

    fn replace_update_at_internal(
        &mut self,
        update: GameplayPresentationUpdate,
        now: Instant,
        suspend_presentation_effects: bool,
    ) -> GameplayDamage {
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
            return damage;
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
        damage
    }

    #[cfg(test)]
    pub(crate) fn replace_update_without_presentation_effects_at(
        &mut self,
        update: GameplayPresentationUpdate,
        now: Instant,
    ) -> GameplayDamage {
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

    pub fn has_active_animation(&self) -> bool {
        self.animation_runner.has_active_animation() || !self.pending_effects.is_empty()
    }

    pub fn advance_presentation_with_damage(&mut self) -> GameplayDamage {
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

    pub(crate) fn advance_presentation_with_damage_at(&mut self, now: Instant) -> GameplayDamage {
        if self.current_scene.is_none() {
            return GameplayDamage::Cells(Vec::new());
        }
        let mut damage = GameplayDamage::Cells(self.advance_ready_presentation_at(now));
        damage = merge_damage(damage, self.apply_ready_effects(now));
        damage
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

    fn apply_ready_effects(&mut self, now: Instant) -> Vec<BoardCell> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum GameplayVisualEffect {
    #[default]
    None,
    PuzzleSolvedClean,
    PuzzleSolvedDirty,
}

impl GameplayVisualEffect {
    fn entity_visual_style(self) -> EntityVisualStyle {
        match self {
            Self::None => EntityVisualStyle::Standard,
            Self::PuzzleSolvedClean => EntityVisualStyle::SolvedClean,
            Self::PuzzleSolvedDirty => EntityVisualStyle::SolvedDirty,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueuedGameplayEffect {
    PuzzleSolved { clean: bool },
}

fn gameplay_damage(
    previous: Option<&GameplayScreenRequest>,
    current: &GameplayScreenRequest,
) -> GameplayDamage {
    let Some(previous) = previous else {
        return GameplayDamage::Full;
    };

    if !gameplay_cell_damage_compatible(previous, current) {
        return GameplayDamage::Full;
    }
    // Pass one still redraws fully on level changes because gameplay chrome changes with the
    // level number, but that policy is not part of the core scene-compatibility invariant.
    if previous.level_number != current.level_number {
        return GameplayDamage::Full;
    }

    let mut dirty = Vec::new();

    if previous.board.player() != current.board.player() {
        add_optional_cell(&mut dirty, previous.board.player());
        add_optional_cell(&mut dirty, current.board.player());
    }

    for cell in current.board.cells() {
        if previous.board.has_box(cell) != current.board.has_box(cell) {
            dirty.push(cell);
        }
    }

    if previous.board.selected_box() != current.board.selected_box() {
        add_optional_cell(&mut dirty, previous.board.selected_box());
        add_optional_cell(&mut dirty, current.board.selected_box());
    }

    GameplayDamage::Cells(normalize_cells(dirty))
}

fn gameplay_cell_damage_compatible(
    previous: &GameplayScreenRequest,
    current: &GameplayScreenRequest,
) -> bool {
    // This compatibility check is intentionally about render structure only. Pass-one policy
    // fallbacks like level changes sit outside it.
    if previous.mode != GameplayScreenMode::Normal || current.mode != GameplayScreenMode::Normal {
        return false;
    }
    if previous.viewport != current.viewport {
        return false;
    }
    if previous.board.width() != current.board.width()
        || previous.board.height() != current.board.height()
    {
        return false;
    }
    for cell in current.board.cells() {
        if previous.board.tile(cell) != current.board.tile(cell) {
            return false;
        }
    }
    true
}

fn add_optional_cell(cells: &mut Vec<BoardCell>, cell: Option<BoardCell>) {
    if let Some(cell) = cell {
        cells.push(cell);
    }
}

fn gameplay_cell_union_rect(
    scene: &GameplayScreenRequest,
    cells: &[BoardCell],
    surface_width: u32,
    surface_height: u32,
) -> Option<ScreenRect> {
    let mut dirty = DamageRectUnion::default();
    for &cell in cells {
        let (x, y, w, h) = scene.viewport.cell_to_screen_rect(cell);
        dirty.add_rect(x, y, w, h, surface_width, surface_height);
    }
    dirty.finish()
}

#[derive(Default)]
struct DamageRectUnion {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    found: bool,
}

impl DamageRectUnion {
    fn add_rect(
        &mut self,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        surface_width: u32,
        surface_height: u32,
    ) {
        if w == 0 || h == 0 || surface_width == 0 || surface_height == 0 {
            return;
        }
        let left = x.max(0) as u32;
        let top = y.max(0) as u32;
        let right = (x + w as i32).clamp(0, surface_width as i32) as u32;
        let bottom = (y + h as i32).clamp(0, surface_height as i32) as u32;
        if left >= right || top >= bottom {
            return;
        }
        if self.found {
            self.left = self.left.min(left);
            self.top = self.top.min(top);
            self.right = self.right.max(right);
            self.bottom = self.bottom.max(bottom);
        } else {
            self.left = left;
            self.top = top;
            self.right = right;
            self.bottom = bottom;
            self.found = true;
        }
    }

    fn finish(self) -> Option<ScreenRect> {
        self.found.then_some(ScreenRect {
            x: self.left,
            y: self.top,
            w: self.right - self.left,
            h: self.bottom - self.top,
        })
    }
}

fn normalize_cells(mut cells: Vec<BoardCell>) -> Vec<BoardCell> {
    cells.sort_by_key(|cell| (cell.y, cell.x));
    cells.dedup();
    cells
}

fn merge_damage(mut current: GameplayDamage, mut more_cells: Vec<BoardCell>) -> GameplayDamage {
    if matches!(current, GameplayDamage::Full) {
        return current;
    }
    if more_cells.is_empty() {
        return current;
    }
    let GameplayDamage::Cells(ref mut cells) = current else {
        unreachable!("full damage returns early");
    };
    cells.append(&mut more_cells);
    *cells = normalize_cells(std::mem::take(cells));
    current
}

fn queued_effect_for_update(update: &GameplayPresentationUpdate) -> Option<QueuedGameplayEffect> {
    match update.cause {
        GameplayPresentationCause::PuzzleSolved { clean } => {
            Some(QueuedGameplayEffect::PuzzleSolved { clean })
        }
        _ => None,
    }
}

fn gameplay_board_state_changed(
    previous: Option<&GameplayScreenRequest>,
    current: &GameplayScreenRequest,
) -> bool {
    let Some(previous) = previous else {
        return true;
    };
    previous.board != current.board
}

fn restart_damage(update: &GameplayPresentationUpdate) -> Vec<BoardCell> {
    if !matches!(update.cause, GameplayPresentationCause::Restarted) {
        return Vec::new();
    }
    normalize_cells(update.scene.board.player().into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::{GameplayDamage, GameplayPresentationState};
    use crate::gameplay_animation::GameplayAnimationPolicy;
    use crate::layout::fit_board_viewport_for_controls;
    use crate::renderer::Renderer;
    use crate::screen_requests::{
        GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenMode,
        GameplayScreenRequest,
    };
    use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};
    use std::time::{Duration, Instant};

    fn gameplay_scene(level_number: usize) -> GameplayPresentationUpdate {
        gameplay_scene_with_player(level_number, Some(BoardCell::new(1, 1)))
    }

    fn cell(x: u32, y: u32) -> BoardCell {
        BoardCell::new(x, y)
    }

    fn gameplay_scene_with_player(
        level_number: usize,
        player: Option<BoardCell>,
    ) -> GameplayPresentationUpdate {
        let board = BoardView::new(
            3,
            3,
            vec![
                TileKind::Void,
                TileKind::Floor,
                TileKind::Void,
                TileKind::Floor,
                TileKind::Floor,
                TileKind::Floor,
                TileKind::Void,
                TileKind::Goal,
                TileKind::Void,
            ],
            vec![false; 9],
            player,
            None,
            false,
        );
        GameplayPresentationUpdate {
            scene: GameplayScreenRequest {
                viewport: fit_board_viewport_for_controls(64, 64, &board),
                board,
                level_number,
                mode: GameplayScreenMode::Normal,
            },
            cause: GameplayPresentationCause::CurrentState,
        }
    }

    fn update_from_board(
        board: BoardView,
        cause: GameplayPresentationCause,
    ) -> GameplayPresentationUpdate {
        GameplayPresentationUpdate {
            scene: GameplayScreenRequest {
                viewport: fit_board_viewport_for_controls(96, 64, &board),
                board,
                level_number: 1,
                mode: GameplayScreenMode::Normal,
            },
            cause,
        }
    }

    fn floor_board(
        width: u32,
        height: u32,
        boxes: Vec<BoardCell>,
        player: Option<BoardCell>,
        selected_box: Option<BoardCell>,
        solved: bool,
    ) -> BoardView {
        let len = (width * height) as usize;
        let mut box_flags = vec![false; len];
        for cell in boxes {
            box_flags[(cell.y * width + cell.x) as usize] = true;
        }
        BoardView::new(
            width,
            height,
            vec![TileKind::Floor; len],
            box_flags,
            player,
            selected_box,
            solved,
        )
    }

    #[test]
    fn replace_update_stores_current_scene() {
        let mut state = GameplayPresentationState::new();
        let first = gameplay_scene(1);
        let second = gameplay_scene(2);

        state.replace_update(first);
        state.replace_update(second.clone());

        assert_eq!(state.current_scene(), Some(&second.scene));
    }

    #[test]
    fn gameplay_damage_for_box_move_is_normalized_dirty_cells() {
        let previous = update_from_board(
            floor_board(
                5,
                3,
                vec![cell(2, 1)],
                Some(cell(1, 1)),
                Some(cell(2, 1)),
                false,
            ),
            GameplayPresentationCause::CurrentState,
        );
        let current = update_from_board(
            floor_board(5, 3, vec![cell(3, 1)], Some(cell(1, 1)), None, false),
            GameplayPresentationCause::BoxMoved {
                path: vec![cell(2, 1), cell(3, 1)],
            },
        );
        let mut state = GameplayPresentationState::new();
        let now = Instant::now();

        let _ = state.replace_update_without_presentation_effects_at(previous, now);
        let damage = state.replace_update_without_presentation_effects_at(current, now);

        assert_eq!(damage, GameplayDamage::Cells(vec![cell(2, 1), cell(3, 1)]));
    }

    #[test]
    fn limited_box_path_damage_includes_sampled_interior_cells_only() {
        let previous = update_from_board(
            floor_board(7, 3, vec![cell(2, 1)], Some(cell(1, 1)), None, false),
            GameplayPresentationCause::CurrentState,
        );
        let current = update_from_board(
            floor_board(7, 3, vec![cell(6, 1)], Some(cell(5, 1)), None, false),
            GameplayPresentationCause::BoxMoved {
                path: vec![cell(2, 1), cell(3, 1), cell(4, 1), cell(5, 1), cell(6, 1)],
            },
        );
        let mut state =
            GameplayPresentationState::with_animation_policy(GameplayAnimationPolicy::Limited);

        state.replace_update(previous);
        let damage = state.replace_update_with_damage(current);

        assert_eq!(
            damage,
            GameplayDamage::Cells(vec![
                cell(1, 1),
                cell(2, 1),
                cell(3, 1),
                cell(4, 1),
                cell(5, 1),
                cell(6, 1)
            ])
        );
    }

    #[test]
    fn clean_puzzle_solved_effect_dirties_boxes_and_player() {
        let previous = update_from_board(
            floor_board(
                5,
                3,
                vec![cell(1, 1), cell(2, 1)],
                Some(cell(0, 1)),
                None,
                false,
            ),
            GameplayPresentationCause::CurrentState,
        );
        let current = update_from_board(
            floor_board(
                5,
                3,
                vec![cell(1, 1), cell(2, 1)],
                Some(cell(0, 1)),
                None,
                true,
            ),
            GameplayPresentationCause::PuzzleSolved { clean: true },
        );
        let mut state = GameplayPresentationState::new();

        state.replace_update(previous);
        let damage = state.replace_update_with_damage(current);

        assert_eq!(
            damage,
            GameplayDamage::Cells(vec![cell(0, 1), cell(1, 1), cell(2, 1)])
        );
    }

    #[test]
    fn dirty_puzzle_solved_effect_dirties_boxes_then_blink_dirties_player() {
        let previous = update_from_board(
            floor_board(
                5,
                3,
                vec![cell(1, 1), cell(2, 1)],
                Some(cell(0, 1)),
                None,
                false,
            ),
            GameplayPresentationCause::CurrentState,
        );
        let current = update_from_board(
            floor_board(
                5,
                3,
                vec![cell(1, 1), cell(2, 1)],
                Some(cell(0, 1)),
                None,
                true,
            ),
            GameplayPresentationCause::PuzzleSolved { clean: false },
        );
        let start = Instant::now();
        let mut state = GameplayPresentationState::new();

        state.replace_update_at(previous, start);
        let damage = state.replace_update_at(current, start);

        assert_eq!(damage, GameplayDamage::Cells(vec![cell(1, 1), cell(2, 1)]));
        assert!(state.has_active_animation());

        let blink_damage =
            state.advance_presentation_with_damage_at(start + Duration::from_millis(400));

        assert_eq!(blink_damage, GameplayDamage::Cells(vec![cell(0, 1)]));
    }

    #[test]
    fn solved_board_flag_without_puzzle_solved_effect_does_not_dirty_entities() {
        let previous = update_from_board(
            floor_board(
                5,
                3,
                vec![cell(1, 1), cell(2, 1)],
                Some(cell(0, 1)),
                None,
                false,
            ),
            GameplayPresentationCause::CurrentState,
        );
        let current = update_from_board(
            floor_board(
                5,
                3,
                vec![cell(1, 1), cell(2, 1)],
                Some(cell(0, 1)),
                None,
                true,
            ),
            GameplayPresentationCause::CurrentState,
        );
        let mut state = GameplayPresentationState::new();
        let now = Instant::now();

        let _ = state.replace_update_without_presentation_effects_at(previous, now);
        let damage = state.replace_update_without_presentation_effects_at(current, now);

        assert_eq!(damage, GameplayDamage::Cells(Vec::new()));
    }

    #[test]
    fn puzzle_solved_effect_waits_for_full_policy_box_move_animation() {
        let previous = update_from_board(
            floor_board(5, 3, vec![cell(2, 1)], Some(cell(1, 1)), None, false),
            GameplayPresentationCause::CurrentState,
        );
        let current = update_from_board(
            floor_board(5, 3, vec![cell(4, 1)], Some(cell(3, 1)), None, true),
            GameplayPresentationCause::BoxMoved {
                path: vec![cell(2, 1), cell(3, 1), cell(4, 1)],
            },
        );
        let solved = GameplayPresentationUpdate {
            cause: GameplayPresentationCause::PuzzleSolved { clean: true },
            ..current.clone()
        };
        let start = Instant::now();
        let mut state = GameplayPresentationState::new();

        let _ = state.replace_update_without_presentation_effects_at(previous, start);
        let _ = state.replace_update_without_presentation_effects_at(current.clone(), start);
        assert!(state.animation_runner.enqueue_for_update(
            None,
            &current,
            GameplayAnimationPolicy::Full,
            start,
        ));
        let solved_damage = state.replace_update_at(solved, start);

        assert_eq!(solved_damage, GameplayDamage::Cells(Vec::new()));
        assert!(state.has_active_animation());

        let first_animation_damage =
            state.advance_presentation_with_damage_at(start + Duration::from_millis(50));
        assert_eq!(
            first_animation_damage,
            GameplayDamage::Cells(vec![cell(2, 1), cell(3, 1), cell(4, 1)])
        );

        let solved_damage =
            state.advance_presentation_with_damage_at(start + Duration::from_millis(100));
        assert_eq!(
            solved_damage,
            GameplayDamage::Cells(vec![cell(2, 1), cell(3, 1), cell(4, 1)])
        );
        assert!(!state.has_active_animation());
    }

    #[test]
    fn restart_after_clean_solve_dirties_unchanged_player_cell() {
        let solved = update_from_board(
            floor_board(5, 3, vec![cell(2, 1)], Some(cell(0, 1)), None, true),
            GameplayPresentationCause::PuzzleSolved { clean: true },
        );
        let restarted = update_from_board(
            floor_board(5, 3, vec![cell(1, 1)], Some(cell(0, 1)), None, false),
            GameplayPresentationCause::Restarted,
        );
        let start = Instant::now();
        let mut state = GameplayPresentationState::new();
        let initial = update_from_board(
            floor_board(5, 3, vec![cell(1, 1)], Some(cell(0, 1)), None, false),
            GameplayPresentationCause::CurrentState,
        );

        state.replace_update_at(initial, start);
        let damage = state.replace_update_at(solved, start);
        assert_eq!(
            damage,
            GameplayDamage::Cells(vec![cell(0, 1), cell(1, 1), cell(2, 1)])
        );

        let damage = state.replace_update_at(restarted.clone(), start);
        assert_eq!(
            damage,
            GameplayDamage::Cells(vec![cell(0, 1), cell(1, 1), cell(2, 1)])
        );
        assert_eq!(restarted.scene.board.player(), Some(cell(0, 1)));
    }

    #[test]
    fn player_move_entity_flash_final_damage_restores_current_player_cell() {
        let previous = update_from_board(
            floor_board(5, 3, Vec::new(), Some(cell(1, 1)), None, false),
            GameplayPresentationCause::CurrentState,
        );
        let current = update_from_board(
            floor_board(5, 3, Vec::new(), Some(cell(2, 1)), None, false),
            GameplayPresentationCause::PlayerMoved { to: cell(2, 1) },
        );
        let start = Instant::now();
        let mut state = GameplayPresentationState::new();
        let mut partial_renderer = Renderer::new();
        let mut full_renderer = Renderer::new();
        let mut partial_frame = vec![0; 96 * 64];
        let mut full_frame = vec![0; 96 * 64];

        state.replace_update_at(previous, start);
        state.draw_at(&mut partial_renderer, &mut partial_frame, 96, 64, start);
        let damage = state.replace_update_at(current.clone(), start);
        state.draw_damage(&mut partial_renderer, &mut partial_frame, 96, 64, &damage);
        let damage = state.advance_presentation_with_damage_at(start + Duration::from_millis(50));
        state.draw_damage(&mut partial_renderer, &mut partial_frame, 96, 64, &damage);
        let damage = state.advance_presentation_with_damage_at(start + Duration::from_millis(100));
        state.draw_damage(&mut partial_renderer, &mut partial_frame, 96, 64, &damage);
        full_renderer.draw_gameplay_scene(&mut full_frame, 96, 64, &current.scene);

        assert_eq!(damage, GameplayDamage::Cells(vec![cell(1, 1)]));
        assert_eq!(partial_frame, full_frame);
        assert!(!state.has_active_animation());
    }

    #[test]
    fn box_move_entity_flash_damage_includes_hidden_current_player_cell() {
        let previous = update_from_board(
            floor_board(5, 3, vec![cell(2, 1)], Some(cell(1, 1)), None, false),
            GameplayPresentationCause::CurrentState,
        );
        let current = update_from_board(
            floor_board(5, 3, vec![cell(3, 1)], Some(cell(1, 1)), None, false),
            GameplayPresentationCause::BoxMoved {
                path: vec![cell(2, 1), cell(3, 1)],
            },
        );
        let mut state = GameplayPresentationState::new();

        state.replace_update(previous);
        let damage = state.replace_update_with_damage(current);

        assert_eq!(
            damage,
            GameplayDamage::Cells(vec![cell(1, 1), cell(2, 1), cell(3, 1)])
        );
    }

    #[test]
    fn level_change_falls_back_to_full_damage() {
        let first = gameplay_scene(1);
        let second = gameplay_scene(2);
        let mut state = GameplayPresentationState::new();
        let now = Instant::now();

        let _ = state.replace_update_without_presentation_effects_at(first, now);
        let damage = state.replace_update_without_presentation_effects_at(second, now);

        assert_eq!(damage, GameplayDamage::Full);
    }

    #[test]
    fn partial_cell_draw_matches_full_gameplay_render() {
        let previous = update_from_board(
            floor_board(
                5,
                3,
                vec![cell(2, 1)],
                Some(cell(1, 1)),
                Some(cell(2, 1)),
                false,
            ),
            GameplayPresentationCause::CurrentState,
        );
        let current = update_from_board(
            floor_board(5, 3, vec![cell(3, 1)], Some(cell(1, 1)), None, false),
            GameplayPresentationCause::BoxMoved {
                path: vec![cell(2, 1), cell(3, 1)],
            },
        );
        let mut state = GameplayPresentationState::new();
        let mut partial_renderer = Renderer::new();
        let mut full_renderer = Renderer::new();
        let mut partial_frame = vec![0; 96 * 64];
        let mut full_frame = vec![0; 96 * 64];
        let now = Instant::now();

        let _ = state.replace_update_without_presentation_effects_at(previous, now);
        state.draw(&mut partial_renderer, &mut partial_frame, 96, 64);
        let damage = state.replace_update_without_presentation_effects_at(current.clone(), now);
        state.draw_damage(&mut partial_renderer, &mut partial_frame, 96, 64, &damage);
        full_renderer.draw_gameplay_scene(&mut full_frame, 96, 64, &current.scene);

        assert_eq!(partial_frame, full_frame);
        assert_eq!(damage, GameplayDamage::Cells(vec![cell(2, 1), cell(3, 1)]));
    }

    #[test]
    fn partial_cell_draw_matches_full_gameplay_render_with_goal_tile() {
        let tiles = vec![
            TileKind::Void,
            TileKind::Goal,
            TileKind::Void,
            TileKind::Floor,
            TileKind::Floor,
            TileKind::Floor,
            TileKind::Void,
            TileKind::Floor,
            TileKind::Void,
        ];
        let previous = update_from_board(
            BoardView::new(
                3,
                3,
                tiles.clone(),
                vec![false, false, false, true, false, false, false, false, false],
                Some(cell(2, 1)),
                None,
                false,
            ),
            GameplayPresentationCause::CurrentState,
        );
        let current = update_from_board(
            BoardView::new(
                3,
                3,
                tiles,
                vec![false, true, false, false, false, false, false, false, false],
                Some(cell(1, 1)),
                None,
                false,
            ),
            GameplayPresentationCause::BoxMoved {
                path: vec![cell(0, 1), cell(1, 0)],
            },
        );
        let mut state = GameplayPresentationState::new();
        let mut partial_renderer = Renderer::new();
        let mut full_renderer = Renderer::new();
        let mut partial_frame = vec![0; 96 * 64];
        let mut full_frame = vec![0; 96 * 64];
        let now = Instant::now();

        let _ = state.replace_update_without_presentation_effects_at(previous, now);
        state.draw(&mut partial_renderer, &mut partial_frame, 96, 64);
        let damage = state.replace_update_without_presentation_effects_at(current.clone(), now);
        state.draw_damage(&mut partial_renderer, &mut partial_frame, 96, 64, &damage);
        full_renderer.draw_gameplay_scene(&mut full_frame, 96, 64, &current.scene);

        assert_eq!(partial_frame, full_frame);
        assert_eq!(
            damage,
            GameplayDamage::Cells(vec![cell(1, 0), cell(0, 1), cell(1, 1), cell(2, 1)])
        );
    }

    #[test]
    fn draw_renders_current_scene() {
        let mut state = GameplayPresentationState::new();
        state.replace_update(gameplay_scene(1));
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 64 * 64];

        state.draw(&mut renderer, &mut frame, 64, 64);

        assert!(frame.iter().any(|pixel| *pixel != 0));
    }

    #[test]
    fn draw_matches_shared_gameplay_renderer_behavior() {
        let update = gameplay_scene(1);
        let mut state = GameplayPresentationState::new();
        state.replace_update(update.clone());
        let mut state_renderer = Renderer::new();
        let mut direct_renderer = Renderer::new();
        let mut state_frame = vec![0; 64 * 64];
        let mut direct_frame = vec![0; 64 * 64];

        state.draw(&mut state_renderer, &mut state_frame, 64, 64);
        direct_renderer.draw_gameplay_scene(&mut direct_frame, 64, 64, &update.scene);

        assert_eq!(state_frame, direct_frame);
    }

    #[test]
    fn box_move_rejected_blink_animation_becomes_visible_after_wait() {
        let mut update = gameplay_scene(1);
        update.cause = GameplayPresentationCause::BoxMoveRejected;
        let mut state = GameplayPresentationState::new();
        let mut renderer = Renderer::new();
        let mut waiting_frame = vec![0; 64 * 64];
        let mut blinking_frame = vec![0; 64 * 64];
        let start = Instant::now();

        state.replace_update_at(update, start);
        state.draw_at(&mut renderer, &mut waiting_frame, 64, 64, start);
        state.draw_at(
            &mut renderer,
            &mut blinking_frame,
            64,
            64,
            start + Duration::from_millis(400),
        );

        assert_ne!(waiting_frame, blinking_frame);
        assert!(state.has_active_animation());
    }

    #[test]
    fn repeated_animated_update_replays_for_unchanged_scene() {
        let mut update = gameplay_scene(1);
        update.cause = GameplayPresentationCause::BoxMoveRejected;
        let mut state = GameplayPresentationState::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 64 * 64];
        let start = Instant::now();

        state.replace_update_at(update.clone(), start);
        state.draw_at(
            &mut renderer,
            &mut frame,
            64,
            64,
            start + Duration::from_millis(400),
        );
        state.draw_at(
            &mut renderer,
            &mut frame,
            64,
            64,
            start + Duration::from_millis(700),
        );
        assert!(!state.has_active_animation());

        state.replace_update_at(update, start + Duration::from_millis(800));

        assert!(state.has_active_animation());
    }

    #[test]
    fn scene_change_drops_pending_animation() {
        let mut rejected_update = gameplay_scene(1);
        rejected_update.cause = GameplayPresentationCause::BoxMoveRejected;
        let moved_update = gameplay_scene(2);
        let mut state = GameplayPresentationState::new();
        let start = Instant::now();

        state.replace_update_at(rejected_update, start);
        assert!(state.has_active_animation());
        state.replace_update_at(moved_update.clone(), start + Duration::from_millis(100));

        assert!(!state.has_active_animation());
        assert_eq!(state.current_scene(), Some(&moved_update.scene));
    }
}
