use super::GameplayAnimation;
use super::box_vanish_drawing::draw_limited_vanishing_box_at;
use crate::gameplay_animation::GameplayAnimationPolicy;
use crate::renderer::Renderer;
use crate::screen_requests::GameplayScreenRequest;
use sokobanitron_gameplay::BoardCell;

#[derive(Debug, Clone, Copy, PartialEq)]
struct BoxVanishPhase {
    scale: f32,
    ticks: u32,
}

const FULL_BOX_VANISH_PHASES: [BoxVanishPhase; 7] = [
    BoxVanishPhase {
        scale: 1.0,
        ticks: 4,
    },
    BoxVanishPhase {
        scale: 0.75,
        ticks: 3,
    },
    BoxVanishPhase {
        scale: 0.5,
        ticks: 3,
    },
    BoxVanishPhase {
        scale: 0.3,
        ticks: 2,
    },
    BoxVanishPhase {
        scale: 0.18,
        ticks: 2,
    },
    BoxVanishPhase {
        scale: 0.14,
        ticks: 1,
    },
    BoxVanishPhase {
        scale: 0.1,
        ticks: 1,
    },
];

const LIMITED_BOX_VANISH_PHASES: [BoxVanishPhase; 10] = [
    BoxVanishPhase {
        scale: 1.0,
        ticks: 3,
    },
    BoxVanishPhase {
        scale: 0.89,
        ticks: 1,
    },
    BoxVanishPhase {
        scale: 0.84,
        ticks: 1,
    },
    BoxVanishPhase {
        scale: 0.77,
        ticks: 1,
    },
    BoxVanishPhase {
        scale: 0.69,
        ticks: 1,
    },
    BoxVanishPhase {
        scale: 0.60,
        ticks: 1,
    },
    BoxVanishPhase {
        scale: 0.50,
        ticks: 1,
    },
    BoxVanishPhase {
        scale: 0.40,
        ticks: 1,
    },
    BoxVanishPhase {
        scale: 0.25,
        ticks: 1,
    },
    BoxVanishPhase {
        scale: 0.1,
        ticks: 1,
    },
];

pub(super) fn box_vanish_animation_for_policy(
    policy: GameplayAnimationPolicy,
    position: BoardCell,
) -> Option<Box<dyn GameplayAnimation>> {
    Some(Box::new(BoxVanishAnimation::new(position, policy)))
}

pub(super) struct BoxVanishAnimation {
    position: BoardCell,
    phase_index: usize,
    policy: GameplayAnimationPolicy,
}

impl BoxVanishAnimation {
    fn new(position: BoardCell, policy: GameplayAnimationPolicy) -> Self {
        Self {
            position,
            phase_index: 0,
            policy,
        }
    }

    fn phases(&self) -> &'static [BoxVanishPhase] {
        match self.policy {
            GameplayAnimationPolicy::Full => &FULL_BOX_VANISH_PHASES,
            GameplayAnimationPolicy::Limited => &LIMITED_BOX_VANISH_PHASES,
        }
    }
}

impl GameplayAnimation for BoxVanishAnimation {
    fn dirty_cells(&self) -> Vec<BoardCell> {
        if self.phases().get(self.phase_index).is_some() {
            vec![self.position]
        } else {
            Vec::new()
        }
    }

    fn draw_over_entities(
        &self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        scene: &GameplayScreenRequest,
        clip_cell: Option<BoardCell>,
    ) {
        if clip_cell.is_some_and(|cell| cell != self.position) {
            return;
        }
        let Some(phase) = self.phases().get(self.phase_index) else {
            return;
        };
        match self.policy {
            GameplayAnimationPolicy::Full => renderer.draw_vanishing_box_at(
                frame,
                width,
                height,
                &scene.viewport,
                self.position,
                phase.scale,
            ),
            GameplayAnimationPolicy::Limited => draw_limited_vanishing_box_at(
                renderer,
                frame,
                width,
                height,
                scene,
                self.position,
                phase.scale,
            ),
        }
    }

    fn ticks_until_next_step(&self) -> Option<u32> {
        self.phases().get(self.phase_index).map(|phase| phase.ticks)
    }

    fn step(&mut self) {
        self.phase_index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::LIMITED_BOX_VANISH_PHASES;

    #[test]
    fn limited_box_vanish_uses_ten_visible_phases() {
        assert_eq!(LIMITED_BOX_VANISH_PHASES.len(), 10);
        assert_eq!(
            LIMITED_BOX_VANISH_PHASES
                .iter()
                .map(|phase| phase.ticks)
                .sum::<u32>(),
            12
        );
        assert_eq!(LIMITED_BOX_VANISH_PHASES[0].scale, 1.0);
        assert_eq!(LIMITED_BOX_VANISH_PHASES[0].ticks, 3);
        assert!(
            LIMITED_BOX_VANISH_PHASES
                .windows(2)
                .all(|window| window[0].scale > window[1].scale)
        );
    }
}
