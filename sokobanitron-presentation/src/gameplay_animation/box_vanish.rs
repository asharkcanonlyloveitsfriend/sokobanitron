use super::GameplayAnimation;
use crate::renderer::Renderer;
use crate::screen_requests::GameplayScreenRequest;

#[derive(Debug, Clone, Copy, PartialEq)]
struct BoxVanishPhase {
    scale: f32,
    ticks: u32,
}

const BOX_VANISH_PHASES: [BoxVanishPhase; 7] = [
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

pub(super) struct BoxVanishAnimation {
    position: (u32, u32),
    phase_index: usize,
}

impl BoxVanishAnimation {
    pub(super) fn new(position: (u32, u32)) -> Self {
        Self {
            position,
            phase_index: 0,
        }
    }
}

impl GameplayAnimation for BoxVanishAnimation {
    fn draw_over_entities(
        &self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        scene: &GameplayScreenRequest,
    ) {
        let Some(phase) = BOX_VANISH_PHASES.get(self.phase_index) else {
            return;
        };
        renderer.draw_vanishing_box_at(
            frame,
            width,
            height,
            &scene.viewport,
            self.position,
            phase.scale,
        );
    }

    fn ticks_until_next_step(&self) -> Option<u32> {
        BOX_VANISH_PHASES
            .get(self.phase_index)
            .map(|phase| phase.ticks)
    }

    fn step(&mut self) {
        self.phase_index += 1;
    }
}
