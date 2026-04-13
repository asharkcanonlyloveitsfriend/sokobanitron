mod blink;
mod box_path;
mod box_vanish;
mod entity_flash;

use self::blink::BlinkAnimation;
use self::box_path::BoxPathAnimation;
use self::box_vanish::BoxVanishAnimation;
use self::entity_flash::EntityFlashAnimation;
use crate::renderer::Renderer;
use crate::screen_requests::{
    GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenRequest,
};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const ANIMATION_TICK: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameplayPresentationConfig {
    pub enable_box_path_animation: bool,
    pub enable_box_vanish_animation: bool,
    pub enable_entity_flash_animation: bool,
}

impl GameplayPresentationConfig {
    pub const fn blink_only() -> Self {
        Self {
            enable_box_path_animation: false,
            enable_box_vanish_animation: false,
            enable_entity_flash_animation: false,
        }
    }
}

impl Default for GameplayPresentationConfig {
    fn default() -> Self {
        Self {
            enable_box_path_animation: true,
            enable_box_vanish_animation: true,
            enable_entity_flash_animation: true,
        }
    }
}

pub(crate) trait GameplayAnimation {
    fn hides_player(&self) -> bool {
        false
    }

    fn draw_under_entities(
        &self,
        _renderer: &mut Renderer,
        _frame: &mut [u8],
        _width: u32,
        _height: u32,
        _scene: &GameplayScreenRequest,
    ) {
    }

    fn draw_over_entities(
        &self,
        _renderer: &mut Renderer,
        _frame: &mut [u8],
        _width: u32,
        _height: u32,
        _scene: &GameplayScreenRequest,
    ) {
    }

    fn ticks_until_next_step(&self) -> Option<u32>;

    fn step(&mut self);
}

struct ActiveAnimation {
    animation: Box<dyn GameplayAnimation>,
    next_step_at: Option<Instant>,
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
        config: GameplayPresentationConfig,
        now: Instant,
    ) -> bool {
        let animations = animations_for_update(previous_scene, update, config);
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

    pub(crate) fn advance_to(&mut self, now: Instant) {
        loop {
            let Some(active) = self.active.as_mut() else {
                if self.queue.is_empty() {
                    return;
                }
                self.start_next(now);
                continue;
            };

            let Some(next_step_at) = active.next_step_at else {
                self.finish_active(now);
                continue;
            };
            if next_step_at > now {
                return;
            }

            active.animation.step();
            active.next_step_at = active.animation.ticks_until_next_step().map(|ticks| {
                now + Duration::from_millis(ANIMATION_TICK.as_millis() as u64 * u64::from(ticks))
            });

            if active.next_step_at.is_none() {
                self.finish_active(now);
            }
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
    ) {
        if let Some(active) = self.active.as_ref() {
            active
                .animation
                .draw_under_entities(renderer, frame, width, height, scene);
        }
    }

    pub(crate) fn draw_over_entities(
        &self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        scene: &GameplayScreenRequest,
    ) {
        if let Some(active) = self.active.as_ref() {
            active
                .animation
                .draw_over_entities(renderer, frame, width, height, scene);
        }
    }

    fn enqueue(&mut self, animation: Box<dyn GameplayAnimation>, now: Instant) {
        self.queue.push_back(animation);
        if self.active.is_none() {
            self.start_next(now);
        }
    }

    fn start_next(&mut self, now: Instant) {
        let Some(animation) = self.queue.pop_front() else {
            self.active = None;
            return;
        };
        let next_step_at = animation.ticks_until_next_step().map(|ticks| {
            now + Duration::from_millis(ANIMATION_TICK.as_millis() as u64 * u64::from(ticks))
        });
        self.active = Some(ActiveAnimation {
            animation,
            next_step_at,
        });
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.next_step_at.is_none())
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
}

fn animations_for_update(
    previous_scene: Option<&GameplayScreenRequest>,
    update: &GameplayPresentationUpdate,
    config: GameplayPresentationConfig,
) -> Vec<Box<dyn GameplayAnimation>> {
    let mut animations: Vec<Box<dyn GameplayAnimation>> = Vec::new();
    if config.enable_entity_flash_animation
        && is_state_change_flash_cause(&update.cause)
        && let Some(animation) = previous_scene.and_then(|previous_scene| {
            EntityFlashAnimation::from_scenes(previous_scene, &update.scene)
        })
    {
        animations.push(Box::new(animation) as Box<dyn GameplayAnimation>);
    }

    match &update.cause {
        GameplayPresentationCause::BoxMoved { path }
            if config.enable_box_path_animation && path.len() > 2 =>
        {
            animations.push(Box::new(BoxPathAnimation::new(path.clone())));
        }
        GameplayPresentationCause::BoxRemoved { to_x, to_y } => {
            if config.enable_box_vanish_animation {
                animations.push(Box::new(BoxVanishAnimation::new((*to_x, *to_y))));
            }
            if let Some(player_position) = update.scene.board.player() {
                animations.push(Box::new(BlinkAnimation::new(player_position)));
            }
        }
        GameplayPresentationCause::BoxMoveRejected => {
            if let Some(player_position) = update.scene.board.player() {
                animations.push(Box::new(BlinkAnimation::new(player_position)));
            }
        }
        _ => {}
    }
    animations
}

fn is_state_change_flash_cause(cause: &GameplayPresentationCause) -> bool {
    matches!(
        cause,
        GameplayPresentationCause::PlayerMoved { .. }
            | GameplayPresentationCause::BoxMoved { .. }
            | GameplayPresentationCause::BoxRemoved { .. }
            | GameplayPresentationCause::UndoApplied
            | GameplayPresentationCause::Restarted
    )
}

#[cfg(test)]
mod tests {
    use super::{GameplayAnimationRunner, GameplayPresentationConfig};
    use crate::layout::fit_board_viewport_for_controls;
    use crate::screen_requests::{
        GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenMode,
        GameplayScreenRequest,
    };
    use sokobanitron_gameplay::{BoardView, TileKind};
    use std::time::{Duration, Instant};

    fn update_with_cause(cause: GameplayPresentationCause) -> GameplayPresentationUpdate {
        update_with_state(cause, Some((0, 0)), vec![false; 8])
    }

    fn update_with_state(
        cause: GameplayPresentationCause,
        player: Option<(u32, u32)>,
        boxes: Vec<bool>,
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
            false,
        );
        GameplayPresentationUpdate {
            scene: GameplayScreenRequest {
                viewport: fit_board_viewport_for_controls(96, 96, &board),
                board,
                level_number: 1,
                mode: GameplayScreenMode::Normal,
            },
            cause,
        }
    }

    #[test]
    fn blink_is_enqueued_for_box_move_rejected() {
        let now = Instant::now();
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            None,
            &update_with_cause(GameplayPresentationCause::BoxMoveRejected),
            GameplayPresentationConfig::default(),
            now,
        ));

        assert!(runner.has_active_animation());
        assert!(!runner.hides_player());
        runner.advance_to(now + Duration::from_millis(400));
        assert!(runner.has_active_animation());
    }

    #[test]
    fn box_path_is_client_configurable() {
        let now = Instant::now();
        let path = vec![(0, 0), (1, 0), (2, 0)];
        let update = update_with_cause(GameplayPresentationCause::BoxMoved { path });
        let mut full_runner = GameplayAnimationRunner::default();
        let mut blink_only_runner = GameplayAnimationRunner::default();

        assert!(full_runner.enqueue_for_update(
            None,
            &update,
            GameplayPresentationConfig::default(),
            now
        ));
        assert!(!blink_only_runner.enqueue_for_update(
            None,
            &update,
            GameplayPresentationConfig::blink_only(),
            now,
        ));

        assert!(full_runner.has_active_animation());
        assert!(!blink_only_runner.has_active_animation());
    }

    #[test]
    fn entity_flash_is_enqueued_from_previous_scene_for_state_changes() {
        let now = Instant::now();
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some((0, 0)),
            vec![false, true, false, false, false, false, false, false],
        );
        let update = update_with_state(
            GameplayPresentationCause::PlayerMoved { to_x: 1, to_y: 0 },
            Some((1, 0)),
            vec![false, true, false, false, false, false, false, false],
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayPresentationConfig::default(),
            now,
        ));

        assert!(runner.has_active_animation());
        assert!(runner.hides_player());
    }

    #[test]
    fn box_removed_uses_blink_only_config_without_vanish_or_flash() {
        let now = Instant::now();
        let previous = update_with_state(
            GameplayPresentationCause::CurrentState,
            Some((0, 0)),
            vec![false, true, false, false, false, false, false, false],
        );
        let update = update_with_state(
            GameplayPresentationCause::BoxRemoved { to_x: 2, to_y: 0 },
            Some((0, 0)),
            vec![false; 8],
        );
        let mut runner = GameplayAnimationRunner::default();

        assert!(runner.enqueue_for_update(
            Some(&previous.scene),
            &update,
            GameplayPresentationConfig::blink_only(),
            now,
        ));

        assert!(runner.has_active_animation());
        assert!(!runner.hides_player());
    }
}
