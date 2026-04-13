use super::GameplayAnimation;
use crate::renderer::{Renderer, blit_rgba};
use crate::screen_requests::GameplayScreenRequest;

pub(super) struct BlinkAnimation {
    player_position: (u32, u32),
    phase: BlinkPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlinkPhase {
    Waiting,
    Blinking,
    Complete,
}

impl BlinkAnimation {
    pub(super) fn new(player_position: (u32, u32)) -> Self {
        Self {
            player_position,
            phase: BlinkPhase::Waiting,
        }
    }
}

impl GameplayAnimation for BlinkAnimation {
    fn draw_over_entities(
        &self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        scene: &GameplayScreenRequest,
    ) {
        if self.phase != BlinkPhase::Blinking {
            return;
        }
        let Some((player_x, player_y, icon_size)) =
            renderer.player_sprite_rect_at(&scene.viewport, self.player_position)
        else {
            return;
        };
        let icon = renderer.player_blink_overlay_bitmap(icon_size);
        blit_rgba(
            frame, width, height, icon, icon_size, icon_size, player_x, player_y,
        );
    }

    fn ticks_until_next_step(&self) -> Option<u32> {
        match self.phase {
            BlinkPhase::Waiting => Some(8),
            BlinkPhase::Blinking => Some(6),
            BlinkPhase::Complete => None,
        }
    }

    fn step(&mut self) {
        self.phase = match self.phase {
            BlinkPhase::Waiting => BlinkPhase::Blinking,
            BlinkPhase::Blinking => BlinkPhase::Complete,
            BlinkPhase::Complete => BlinkPhase::Complete,
        };
    }
}
