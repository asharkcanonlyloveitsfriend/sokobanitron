use super::GameplayAnimation;
use crate::renderer::{Renderer, blit_premultiplied_gray_alpha};
use crate::screen_requests::GameplayScreenRequest;
use sokobanitron_gameplay::BoardCell;

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
