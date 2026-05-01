use super::{GameplayAnimation, animation_tick_duration};
use crate::renderer::{Renderer, blit_premultiplied_gray_alpha};
use crate::screen_requests::GameplayScreenRequest;
use sokobanitron_gameplay::BoardCell;
use std::time::Duration;

const WAIT_TICKS: u32 = 8;
const BLINK_TICKS: u32 = 6;

pub(super) struct BlinkAnimation {
    player_position: BoardCell,
    phase: BlinkPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlinkPhase {
    Waiting,
    Blinking,
    Complete,
}

impl BlinkAnimation {
    pub(super) fn new(player_position: BoardCell) -> Self {
        Self {
            player_position,
            phase: BlinkPhase::Waiting,
        }
    }

    fn phase_for_elapsed(elapsed: Duration) -> BlinkPhase {
        if elapsed < animation_tick_duration(WAIT_TICKS) {
            BlinkPhase::Waiting
        } else if elapsed < animation_tick_duration(WAIT_TICKS + BLINK_TICKS) {
            BlinkPhase::Blinking
        } else {
            BlinkPhase::Complete
        }
    }
}

impl GameplayAnimation for BlinkAnimation {
    fn dirty_cells(&self) -> Vec<BoardCell> {
        match self.phase {
            BlinkPhase::Blinking => vec![self.player_position],
            BlinkPhase::Waiting | BlinkPhase::Complete => Vec::new(),
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
        if self.phase != BlinkPhase::Blinking
            || clip_cell.is_some_and(|cell| cell != self.player_position)
        {
            return;
        }
        let Some((player_x, player_y, icon_size)) =
            renderer.player_sprite_rect_at(&scene.viewport, self.player_position)
        else {
            return;
        };
        let icon = renderer.player_blink_overlay_bitmap(icon_size);
        blit_premultiplied_gray_alpha(
            frame, width, height, icon, icon_size, icon_size, player_x, player_y,
        );
    }

    fn duration(&self) -> Duration {
        animation_tick_duration(WAIT_TICKS + BLINK_TICKS)
    }

    fn set_elapsed(&mut self, elapsed: Duration) {
        self.phase = Self::phase_for_elapsed(elapsed);
    }

    fn advance_to_elapsed(&mut self, elapsed: Duration) -> Vec<BoardCell> {
        let previous_phase = self.phase;
        let previous_dirty = self.dirty_cells();
        self.set_elapsed(elapsed);
        if previous_phase == self.phase {
            Vec::new()
        } else {
            let mut dirty = previous_dirty;
            dirty.extend(self.dirty_cells());
            dirty
        }
    }
}
