mod blink;
mod box_move;
mod box_path_drawing;
mod box_vanish;
mod box_vanish_drawing;
mod entity_flash;

use self::blink::BlinkAnimation;
use self::box_move::box_move_animation_for_policy;
use self::box_vanish::box_vanish_animation_for_policy;
use self::entity_flash::entity_flash_animation_for_policy;
use crate::renderer::Renderer;
use crate::screen_requests::{
    GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenRequest,
};
use sokobanitron_gameplay::BoardCell;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const ANIMATION_TICK: Duration = Duration::from_millis(50);
const ANIMATION_TICK_BOUNDARY_TOLERANCE: Duration = Duration::from_millis(2);

pub(super) fn animation_tick_duration(ticks: u32) -> Duration {
    ANIMATION_TICK * ticks
}

fn animation_elapsed_for_timing(elapsed: Duration) -> Duration {
    elapsed.saturating_add(ANIMATION_TICK_BOUNDARY_TOLERANCE)
}

fn animation_deadline_reached(now: Instant, deadline: Instant) -> bool {
    now.checked_add(ANIMATION_TICK_BOUNDARY_TOLERANCE)
        .is_none_or(|now_with_tolerance| now_with_tolerance >= deadline)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameplayAnimationPolicy {
    #[default]
    Full,
    Limited,
}

pub(crate) trait GameplayAnimation {
    fn hides_player(&self) -> bool {
        false
    }

    /// Returns the smallest correct dirty-cell set for the animation's current state.
    ///
    /// The runner is responsible only for unioning dirty cells across animation transitions
    /// (previous step, current step, start, finish). Presentation state then merges that
    /// animation damage with baseline scene damage and queued effect damage.
    fn dirty_cells(&self) -> Vec<BoardCell> {
        Vec::new()
    }

    fn draw_under_entities(
        &self,
        _renderer: &mut Renderer,
        _frame: &mut [u8],
        _width: u32,
        _height: u32,
        _scene: &GameplayScreenRequest,
        _clip_cell: Option<BoardCell>,
    ) {
    }

    fn draw_over_entities(
        &self,
        _renderer: &mut Renderer,
        _frame: &mut [u8],
        _width: u32,
        _height: u32,
        _scene: &GameplayScreenRequest,
        _clip_cell: Option<BoardCell>,
    ) {
    }

    fn duration(&self) -> Duration;

    fn set_elapsed(&mut self, elapsed: Duration);

    fn advance_to_elapsed(&mut self, elapsed: Duration) -> Vec<BoardCell> {
        let previous_dirty = self.dirty_cells();
        self.set_elapsed(elapsed);
        let mut dirty = previous_dirty;
        dirty.extend(self.dirty_cells());
        normalize_cells(dirty)
    }
}

struct ActiveAnimation {
    animation: Box<dyn GameplayAnimation>,
    started_at: Instant,
    ends_at: Instant,
    initial_frame_drawn: bool,
    initial_frame_presented: bool,
}

#[derive(Default)]
pub(crate) struct GameplayAnimationRunner {
    queue: VecDeque<Box<dyn GameplayAnimation>>,
    active: Option<ActiveAnimation>,
}

impl GameplayAnimationRunner {
    pub(crate) fn enqueue_for_update(
        &mut self,
        previous_scene: Option<&GameplayScreenRequest>,
        update: &GameplayPresentationUpdate,
        policy: GameplayAnimationPolicy,
        now: Instant,
    ) -> bool {
        // Build the semantic animation order first, then enqueue that sequence as-is.
        let animations = ordered_animations_for_update(previous_scene, update, policy);
        let animation_enqueued = !animations.is_empty();
        for animation in animations {
            self.enqueue(animation, now);
        }
        animation_enqueued
    }

    pub(crate) fn clear(&mut self) {
        self.active = None;
        self.queue.clear();
    }

    pub(crate) fn clear_damage(&mut self) -> Vec<BoardCell> {
        let dirty = self.current_dirty_cells();
        self.clear();
        dirty
    }

    pub(crate) fn mark_initial_frame_presented_at(&mut self, now: Instant) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        if active.initial_frame_presented || !active.initial_frame_drawn {
            return;
        }
        let duration = active.animation.duration();
        active.started_at = now;
        active.ends_at = now + duration;
        active.initial_frame_presented = true;
    }

    pub(crate) fn advance_to_with_damage(&mut self, now: Instant) -> Vec<BoardCell> {
        let mut dirty = Vec::new();
        loop {
            if self.active.is_none() {
                if self.queue.is_empty() {
                    return normalize_cells(dirty);
                }
                self.start_next(now);
                return self.draw_initial_frame();
            }

            if self
                .active
                .as_ref()
                .is_some_and(|active| !active.initial_frame_drawn)
            {
                return self.draw_initial_frame();
            }

            let active = self
                .active
                .as_ref()
                .expect("active animation checked above");
            let started_at = active.started_at;
            let ends_at = active.ends_at;
            let duration = active.animation.duration();
            let raw_elapsed = now.saturating_duration_since(started_at);
            let finished = animation_deadline_reached(now, ends_at);
            let elapsed = if finished {
                duration
            } else {
                animation_elapsed_for_timing(raw_elapsed).min(duration)
            };

            {
                let active = self
                    .active
                    .as_mut()
                    .expect("active animation checked above");
                dirty.extend(active.animation.advance_to_elapsed(elapsed));
            }

            if !finished {
                return normalize_cells(dirty);
            }

            self.finish_active(now);
            dirty.extend(self.draw_initial_frame());
        }
    }

    pub(crate) fn has_active_animation(&self) -> bool {
        self.active.is_some() || !self.queue.is_empty()
    }

    pub(crate) fn hides_player(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|active| active.animation.hides_player())
    }

    pub(crate) fn draw_under_entities(
        &self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        scene: &GameplayScreenRequest,
        clip_cell: Option<BoardCell>,
    ) {
        if let Some(active) = self.active.as_ref() {
            active
                .animation
                .draw_under_entities(renderer, frame, width, height, scene, clip_cell);
        }
    }

    pub(crate) fn draw_over_entities(
        &self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        scene: &GameplayScreenRequest,
        clip_cell: Option<BoardCell>,
    ) {
        if let Some(active) = self.active.as_ref() {
            active
                .animation
                .draw_over_entities(renderer, frame, width, height, scene, clip_cell);
        }
    }

    pub(crate) fn current_dirty_cells(&self) -> Vec<BoardCell> {
        self.active
            .as_ref()
            // Active animations report only their current local dirty footprint.
            .map(|active| normalize_cells(active.animation.dirty_cells()))
            .unwrap_or_default()
    }

    fn enqueue(&mut self, animation: Box<dyn GameplayAnimation>, now: Instant) {
        self.queue.push_back(animation);
        if self.active.is_none() {
            self.start_next(now);
        }
    }

    fn start_next(&mut self, now: Instant) {
        let Some(mut animation) = self.queue.pop_front() else {
            self.active = None;
            return;
        };
        animation.set_elapsed(Duration::ZERO);
        let duration = animation.duration();
        let ends_at = now + duration;
        self.active = Some(ActiveAnimation {
            animation,
            started_at: now,
            ends_at,
            initial_frame_drawn: false,
            initial_frame_presented: false,
        });
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.ends_at <= now)
        {
            self.finish_active(now);
        }
    }

    fn finish_active(&mut self, now: Instant) {
        self.active = None;
        if !self.queue.is_empty() {
            self.start_next(now);
        }
    }

    fn draw_initial_frame(&mut self) -> Vec<BoardCell> {
        let Some(active) = self.active.as_mut() else {
            return Vec::new();
        };
        active.initial_frame_drawn = true;
        normalize_cells(self.current_dirty_cells())
    }
}

#[derive(Default)]
struct OrderedAnimations {
    animations: Vec<Box<dyn GameplayAnimation>>,
}

impl OrderedAnimations {
    fn push_optional(&mut self, animation: Option<Box<dyn GameplayAnimation>>) {
        if let Some(animation) = animation {
            self.animations.push(animation);
        }
    }

    fn push_blink(&mut self, player_position: Option<BoardCell>) {
        if let Some(player_position) = player_position {
            self.animations
                .push(Box::new(BlinkAnimation::new(player_position)));
        }
    }

    fn into_vec(self) -> Vec<Box<dyn GameplayAnimation>> {
        self.animations
    }
}

fn ordered_animations_for_update(
    previous_scene: Option<&GameplayScreenRequest>,
    update: &GameplayPresentationUpdate,
    policy: GameplayAnimationPolicy,
) -> Vec<Box<dyn GameplayAnimation>> {
    let mut ordered = OrderedAnimations::default();
    enqueue_state_change_flash(&mut ordered, previous_scene, update, policy);
    enqueue_cause_animations(&mut ordered, previous_scene, update, policy);
    enqueue_dirty_solution_blink(&mut ordered, previous_scene, update);
    ordered.into_vec()
}

fn enqueue_state_change_flash(
    ordered: &mut OrderedAnimations,
    previous_scene: Option<&GameplayScreenRequest>,
    update: &GameplayPresentationUpdate,
    policy: GameplayAnimationPolicy,
) {
    if is_state_change_flash_cause(&update.cause) {
        ordered.push_optional(entity_flash_animation_for_policy(
            policy,
            previous_scene,
            update,
        ));
    }
}

fn enqueue_cause_animations(
    ordered: &mut OrderedAnimations,
    previous_scene: Option<&GameplayScreenRequest>,
    update: &GameplayPresentationUpdate,
    policy: GameplayAnimationPolicy,
) {
    match &update.cause {
        GameplayPresentationCause::BoxMoved { .. } => {
            ordered.push_optional(box_move_animation_for_policy(
                policy,
                previous_scene,
                update,
            ));
        }
        GameplayPresentationCause::BoxRemoved { to } => {
            enqueue_box_removed_animations(ordered, update, policy, *to);
        }
        GameplayPresentationCause::BoxMoveRejected => {
            ordered.push_blink(update.scene.board.player());
        }
        _ => {}
    }
}

fn enqueue_box_removed_animations(
    ordered: &mut OrderedAnimations,
    update: &GameplayPresentationUpdate,
    policy: GameplayAnimationPolicy,
    position: BoardCell,
) {
    ordered.push_optional(box_vanish_animation_for_policy(policy, position));
    ordered.push_blink(update.scene.board.player());
}

fn enqueue_dirty_solution_blink(
    ordered: &mut OrderedAnimations,
    previous_scene: Option<&GameplayScreenRequest>,
    update: &GameplayPresentationUpdate,
) {
    if matches!(update.cause, GameplayPresentationCause::BoxRemoved { .. }) {
        return;
    }
    let Some(previous_scene) = previous_scene else {
        return;
    };
    if !previous_scene.board.is_solved() && update.scene.board.is_dirty_solution() {
        ordered.push_blink(update.scene.board.player());
    }
}

fn is_state_change_flash_cause(cause: &GameplayPresentationCause) -> bool {
    matches!(
        cause,
        GameplayPresentationCause::PlayerMoved { .. }
            | GameplayPresentationCause::BoxRemoved { .. }
            | GameplayPresentationCause::UndoApplied
            | GameplayPresentationCause::Restarted
    )
}

fn normalize_cells(mut cells: Vec<BoardCell>) -> Vec<BoardCell> {
    cells.sort_by_key(|cell| (cell.y, cell.x));
    cells.dedup();
    cells
}

#[cfg(test)]
mod tests {
    use super::{GameplayAnimationPolicy, GameplayAnimationRunner};
    use crate::layout::fit_board_viewport_for_controls;
    use crate::screen_requests::{
        GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenMode,
        GameplayScreenRequest,
    };
    use sokobanitron_gameplay::{BoardCell, BoardSolveState, BoardView, TileKind};
    use std::time::{Duration, Instant};

    fn update_with_cause(cause: GameplayPresentationCause) -> GameplayPresentationUpdate {
        update_with_state(cause, Some(BoardCell::new(0, 0)), vec![false; 8])
    }

    fn update_with_state(
        cause: GameplayPresentationCause,
        player: Option<BoardCell>,
        boxes: Vec<bool>,
    ) -> GameplayPresentationUpdate {
        update_with_solve_state(cause, player, boxes, BoardSolveState::Unsolved)
    }

    fn update_with_solve_state(
        cause: GameplayPresentationCause,
        player: Option<BoardCell>,
        boxes: Vec<bool>,
        solve_state: BoardSolveState,
    ) -> GameplayPresentationUpdate {
        let board = BoardView::new(
            4,
            2,
            vec![
                TileKind::Floor,
                TileKind::Floor,
                TileKind::Floor,
                TileKind::Floor,
                TileKind::Floor,
                TileKind::Floor,
                TileKind::Floor,
                TileKind::Floor,
            ],
            boxes,
            player,
            None,
            solve_state,
        );
        GameplayPresentationUpdate {
            scene: GameplayScreenRequest {
                viewport: fit_board_viewport_for_controls(96, 96, &board),
                board,
                level_number: 1,
                mode: GameplayScreenMode::Normal,
                sleeping_player: false,
            },
            cause,
        }
    }

    fn draw_and_mark_initial_frame(runner: &mut GameplayAnimationRunner, now: Instant) {
        let _ = runner.advance_to_with_damage(now);
        runner.mark_initial_frame_presented_at(now);
    }

    #[test]
    fn blink_is_enqueued_for_box_move_rejected() {
        let now = Instant::now();
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            None,
            &update_with_cause(GameplayPresentationCause::BoxMoveRejected),
            GameplayAnimationPolicy::Full,
            now,
        ));
        draw_and_mark_initial_frame(&mut runner, now);

        assert!(runner.has_active_animation());
        assert!(!runner.hides_player());
        let _ = runner.advance_to_with_damage(now + Duration::from_millis(400));
        assert!(runner.has_active_animation());
    }

    #[test]
    fn limited_box_move_skips_short_paths() {
        let now = Instant::now();
        let path = vec![
            BoardCell::new(0, 0),
            BoardCell::new(1, 0),
            BoardCell::new(2, 0),
        ];
        let update = update_with_cause(GameplayPresentationCause::BoxMoved { path });
        let mut limited_runner = GameplayAnimationRunner::default();

        assert!(!limited_runner.enqueue_for_update(
            None,
            &update,
            GameplayAnimationPolicy::Limited,
            now,
        ));

        assert!(!limited_runner.has_active_animation());
    }

    #[test]
    fn limited_box_move_uses_sampled_frames_without_hiding_player() {
        let now = Instant::now();
        let path = vec![
            BoardCell::new(0, 0),
            BoardCell::new(1, 0),
            BoardCell::new(2, 0),
            BoardCell::new(3, 0),
            BoardCell::new(4, 0),
            BoardCell::new(5, 0),
            BoardCell::new(6, 0),
            BoardCell::new(7, 0),
        ];
        let update = update_with_cause(GameplayPresentationCause::BoxMoved { path });
        let mut limited_runner = GameplayAnimationRunner::default();

        assert!(limited_runner.enqueue_for_update(
            None,
            &update,
            GameplayAnimationPolicy::Limited,
            now,
        ));
        draw_and_mark_initial_frame(&mut limited_runner, now);

        assert!(limited_runner.has_active_animation());
        assert!(!limited_runner.hides_player());
        assert_eq!(
            limited_runner.current_dirty_cells(),
            vec![BoardCell::new(1, 0), BoardCell::new(2, 0)]
        );
        let _ = limited_runner.advance_to_with_damage(now + Duration::from_millis(50));
        assert_eq!(
            limited_runner.current_dirty_cells(),
            vec![
                BoardCell::new(3, 0),
                BoardCell::new(4, 0),
                BoardCell::new(5, 0)
            ]
        );
        let final_damage = limited_runner.advance_to_with_damage(now + Duration::from_millis(100));
        assert_eq!(
            final_damage,
            vec![
                BoardCell::new(3, 0),
                BoardCell::new(4, 0),
                BoardCell::new(5, 0)
            ]
        );
        assert!(!limited_runner.has_active_animation());
    }

    #[test]
    fn entity_flash_is_enqueued_from_previous_scene_for_state_changes() {
        let now = Instant::now();
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some(BoardCell::new(0, 0)),
            vec![false, true, false, false, false, false, false, false],
        );
        let update = update_with_state(
            GameplayPresentationCause::PlayerMoved {
                to: BoardCell::new(1, 0),
            },
            Some(BoardCell::new(1, 0)),
            vec![false, true, false, false, false, false, false, false],
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayAnimationPolicy::Full,
            now,
        ));
        draw_and_mark_initial_frame(&mut runner, now);

        assert!(runner.has_active_animation());
        assert!(!runner.hides_player());
    }

    #[test]
    fn animation_runner_advances_near_tick_boundary() {
        let now = Instant::now();
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some(BoardCell::new(0, 0)),
            vec![false; 8],
        );
        let update = update_with_state(
            GameplayPresentationCause::PlayerMoved {
                to: BoardCell::new(1, 0),
            },
            Some(BoardCell::new(1, 0)),
            vec![false; 8],
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayAnimationPolicy::Full,
            now,
        ));
        draw_and_mark_initial_frame(&mut runner, now);

        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(47)),
            Vec::<BoardCell>::new()
        );
        assert!(runner.has_active_animation());
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(49)),
            vec![BoardCell::new(0, 0)]
        );
        assert!(runner.has_active_animation());
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(99)),
            vec![BoardCell::new(0, 0)]
        );
        assert!(!runner.has_active_animation());
    }

    #[test]
    fn animation_runner_can_align_initial_frame_to_present_time() {
        let triggered_at = Instant::now();
        let presented_at = triggered_at + Duration::from_millis(40);
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some(BoardCell::new(0, 0)),
            vec![false; 8],
        );
        let update = update_with_state(
            GameplayPresentationCause::PlayerMoved {
                to: BoardCell::new(1, 0),
            },
            Some(BoardCell::new(1, 0)),
            vec![false; 8],
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayAnimationPolicy::Full,
            triggered_at,
        ));
        let _ = runner.advance_to_with_damage(presented_at);
        runner.mark_initial_frame_presented_at(presented_at);

        assert_eq!(
            runner.advance_to_with_damage(presented_at + Duration::from_millis(47)),
            Vec::<BoardCell>::new()
        );
        assert!(runner.has_active_animation());
        assert_eq!(
            runner.advance_to_with_damage(presented_at + Duration::from_millis(49)),
            vec![BoardCell::new(0, 0)]
        );
        assert!(runner.has_active_animation());
    }

    #[test]
    fn animation_runner_ignores_initial_present_mark_before_first_draw() {
        let triggered_at = Instant::now();
        let presented_at = triggered_at + Duration::from_millis(40);
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some(BoardCell::new(0, 0)),
            vec![false; 8],
        );
        let update = update_with_state(
            GameplayPresentationCause::PlayerMoved {
                to: BoardCell::new(1, 0),
            },
            Some(BoardCell::new(1, 0)),
            vec![false; 8],
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayAnimationPolicy::Full,
            triggered_at,
        ));
        runner.mark_initial_frame_presented_at(triggered_at);
        let _ = runner.advance_to_with_damage(presented_at);
        runner.mark_initial_frame_presented_at(presented_at);

        assert_eq!(
            runner.advance_to_with_damage(presented_at + Duration::from_millis(47)),
            Vec::<BoardCell>::new()
        );
        assert!(runner.has_active_animation());
    }

    #[test]
    fn full_box_move_hides_player_through_cleanup_phase() {
        let now = Instant::now();
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some(BoardCell::new(1, 0)),
            vec![false, false, true, false, false, false, false, false],
        );
        let update = update_with_state(
            GameplayPresentationCause::BoxMoved {
                path: vec![
                    BoardCell::new(2, 0),
                    BoardCell::new(3, 0),
                    BoardCell::new(3, 1),
                ],
            },
            Some(BoardCell::new(2, 0)),
            vec![false, false, false, true, false, false, false, false],
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayAnimationPolicy::Full,
            now,
        ));
        draw_and_mark_initial_frame(&mut runner, now);

        assert!(runner.has_active_animation());
        assert!(runner.hides_player());
        let _ = runner.advance_to_with_damage(now + Duration::from_millis(50));
        assert!(runner.has_active_animation());
        assert!(runner.hides_player());
        let _ = runner.advance_to_with_damage(now + Duration::from_millis(100));
        assert!(!runner.has_active_animation());
    }

    #[test]
    fn dirty_solve_blink_waits_until_box_move_finishes() {
        let now = Instant::now();
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some(BoardCell::new(1, 0)),
            vec![false, false, true, false, false, false, false, false],
        );
        let update = update_with_solve_state(
            GameplayPresentationCause::BoxMoved {
                path: vec![
                    BoardCell::new(2, 0),
                    BoardCell::new(3, 0),
                    BoardCell::new(3, 1),
                ],
            },
            Some(BoardCell::new(2, 0)),
            vec![false, false, false, true, false, false, false, false],
            BoardSolveState::SolvedDirty,
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayAnimationPolicy::Full,
            now,
        ));
        draw_and_mark_initial_frame(&mut runner, now);

        assert!(runner.has_active_animation());
        assert!(runner.hides_player());
        let _ = runner.advance_to_with_damage(now + Duration::from_millis(100));
        assert!(runner.has_active_animation());
        assert!(!runner.hides_player());
        assert_eq!(runner.current_dirty_cells(), Vec::<BoardCell>::new());

        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(500)),
            vec![BoardCell::new(2, 0)]
        );
        assert!(runner.has_active_animation());
    }

    #[test]
    fn full_box_move_flash_draws_full_path_for_both_flash_ticks() {
        let now = Instant::now();
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some(BoardCell::new(1, 0)),
            vec![false, false, true, false, false, false, false, false],
        );
        let update = update_with_state(
            GameplayPresentationCause::BoxMoved {
                path: vec![
                    BoardCell::new(2, 0),
                    BoardCell::new(3, 0),
                    BoardCell::new(3, 1),
                ],
            },
            Some(BoardCell::new(2, 0)),
            vec![false, false, false, true, false, false, false, false],
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayAnimationPolicy::Full,
            now,
        ));
        draw_and_mark_initial_frame(&mut runner, now);

        assert_eq!(
            runner.current_dirty_cells(),
            vec![
                BoardCell::new(1, 0),
                BoardCell::new(2, 0),
                BoardCell::new(3, 0),
                BoardCell::new(3, 1)
            ]
        );

        let damage = runner.advance_to_with_damage(now + Duration::from_millis(50));
        assert_eq!(damage, vec![BoardCell::new(1, 0), BoardCell::new(2, 0)]);
        assert_eq!(
            runner.current_dirty_cells(),
            vec![
                BoardCell::new(1, 0),
                BoardCell::new(2, 0),
                BoardCell::new(3, 0),
                BoardCell::new(3, 1)
            ]
        );

        let _ = runner.advance_to_with_damage(now + Duration::from_millis(100));
        assert_eq!(runner.current_dirty_cells(), Vec::<BoardCell>::new());
    }

    #[test]
    fn full_box_move_flash_light_clear_damage_includes_path() {
        let now = Instant::now();
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some(BoardCell::new(1, 0)),
            vec![false, false, true, false, false, false, false, false],
        );
        let update = update_with_state(
            GameplayPresentationCause::BoxMoved {
                path: vec![
                    BoardCell::new(2, 0),
                    BoardCell::new(3, 0),
                    BoardCell::new(3, 1),
                ],
            },
            Some(BoardCell::new(2, 0)),
            vec![false, false, false, true, false, false, false, false],
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayAnimationPolicy::Full,
            now,
        ));
        draw_and_mark_initial_frame(&mut runner, now);
        let _ = runner.advance_to_with_damage(now + Duration::from_millis(50));

        assert_eq!(
            runner.clear_damage(),
            vec![
                BoardCell::new(1, 0),
                BoardCell::new(2, 0),
                BoardCell::new(3, 0),
                BoardCell::new(3, 1)
            ]
        );
        assert!(!runner.has_active_animation());
    }

    #[test]
    fn short_full_box_move_does_not_draw_path_during_flash() {
        let now = Instant::now();
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some(BoardCell::new(1, 0)),
            vec![false, false, true, false, false, false, false, false],
        );
        let update = update_with_state(
            GameplayPresentationCause::BoxMoved {
                path: vec![BoardCell::new(2, 0), BoardCell::new(3, 0)],
            },
            Some(BoardCell::new(2, 0)),
            vec![false, false, false, true, false, false, false, false],
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayAnimationPolicy::Full,
            now,
        ));
        draw_and_mark_initial_frame(&mut runner, now);

        assert_eq!(
            runner.current_dirty_cells(),
            vec![BoardCell::new(1, 0), BoardCell::new(2, 0)]
        );
        let final_damage = runner.advance_to_with_damage(now + Duration::from_millis(100));
        assert_eq!(
            final_damage,
            vec![BoardCell::new(1, 0), BoardCell::new(2, 0)]
        );
        assert!(!runner.has_active_animation());
    }

    #[test]
    fn box_removed_uses_limited_vanish_before_blink() {
        let now = Instant::now();
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some(BoardCell::new(0, 0)),
            vec![false, true, false, false, false, false, false, false],
        );
        let update = update_with_state(
            GameplayPresentationCause::BoxRemoved {
                to: BoardCell::new(2, 0),
            },
            Some(BoardCell::new(0, 0)),
            vec![false; 8],
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayAnimationPolicy::Limited,
            now,
        ));
        draw_and_mark_initial_frame(&mut runner, now);

        assert!(runner.has_active_animation());
        assert!(!runner.hides_player());
        assert_eq!(runner.current_dirty_cells(), vec![BoardCell::new(2, 0)]);
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(50)),
            Vec::<BoardCell>::new()
        );
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(100)),
            Vec::<BoardCell>::new()
        );
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(150)),
            vec![BoardCell::new(2, 0)]
        );
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(200)),
            vec![BoardCell::new(2, 0)]
        );
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(250)),
            vec![BoardCell::new(2, 0)]
        );
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(300)),
            vec![BoardCell::new(2, 0)]
        );
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(350)),
            vec![BoardCell::new(2, 0)]
        );
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(400)),
            vec![BoardCell::new(2, 0)]
        );
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(450)),
            vec![BoardCell::new(2, 0)]
        );
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(500)),
            vec![BoardCell::new(2, 0)]
        );
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(550)),
            vec![BoardCell::new(2, 0)]
        );
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(600)),
            vec![BoardCell::new(2, 0)]
        );
        assert!(runner.has_active_animation());
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(650)),
            Vec::<BoardCell>::new()
        );
        assert!(runner.has_active_animation());
        assert_eq!(
            runner.advance_to_with_damage(now + Duration::from_millis(1000)),
            vec![BoardCell::new(0, 0)]
        );
    }
}
