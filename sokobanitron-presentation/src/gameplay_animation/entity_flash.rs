use super::GameplayAnimation;
use crate::renderer::{Renderer, blit_rgba};
use crate::screen_requests::GameplayScreenRequest;

const FLASH_DARK_COLOR: [u8; 4] = [142, 142, 142, 255];
const FLASH_LIGHT_COLOR: [u8; 4] = [242, 242, 242, 255];

pub(super) struct EntityFlashAnimation {
    player_position: (u32, u32),
    box_positions: Vec<(u32, u32)>,
    phase: EntityFlashPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityFlashPhase {
    FlashDark,
    FlashLight,
    Complete,
}

impl EntityFlashAnimation {
    pub(super) fn from_scenes(
        previous: &GameplayScreenRequest,
        current: &GameplayScreenRequest,
    ) -> Option<Self> {
        let player_position = previous.board.player()?;
        let player_changed = Some(player_position) != current.board.player();
        let box_positions = removed_box_positions(previous, current);
        if !player_changed && box_positions.is_empty() {
            return None;
        }
        Some(Self {
            player_position,
            box_positions,
            phase: EntityFlashPhase::FlashDark,
        })
    }

    fn flash_color(&self) -> Option<[u8; 4]> {
        match self.phase {
            EntityFlashPhase::FlashDark => Some(FLASH_DARK_COLOR),
            EntityFlashPhase::FlashLight => Some(FLASH_LIGHT_COLOR),
            EntityFlashPhase::Complete => None,
        }
    }
}

impl GameplayAnimation for EntityFlashAnimation {
    fn hides_player(&self) -> bool {
        true
    }

    fn draw_over_entities(
        &self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        scene: &GameplayScreenRequest,
    ) {
        let Some(color) = self.flash_color() else {
            return;
        };
        draw_tinted_player(
            renderer,
            frame,
            width,
            height,
            scene,
            self.player_position,
            color,
        );
        for &position in &self.box_positions {
            draw_tinted_box(renderer, frame, width, height, scene, position, color);
        }
    }

    fn ticks_until_next_step(&self) -> Option<u32> {
        match self.phase {
            EntityFlashPhase::FlashDark => Some(1),
            EntityFlashPhase::FlashLight => Some(1),
            EntityFlashPhase::Complete => None,
        }
    }

    fn step(&mut self) {
        self.phase = match self.phase {
            EntityFlashPhase::FlashDark => EntityFlashPhase::FlashLight,
            EntityFlashPhase::FlashLight => EntityFlashPhase::Complete,
            EntityFlashPhase::Complete => EntityFlashPhase::Complete,
        };
    }
}

fn removed_box_positions(
    previous: &GameplayScreenRequest,
    current: &GameplayScreenRequest,
) -> Vec<(u32, u32)> {
    let mut removed = Vec::new();
    for y in 0..previous.board.height() {
        for x in 0..previous.board.width() {
            let current_has_box = x < current.board.width()
                && y < current.board.height()
                && current.board.has_box(x, y);
            if previous.board.has_box(x, y) && !current_has_box {
                removed.push((x, y));
            }
        }
    }
    removed
}

fn draw_tinted_player(
    renderer: &mut Renderer,
    frame: &mut [u8],
    width: u32,
    height: u32,
    scene: &GameplayScreenRequest,
    position: (u32, u32),
    color: [u8; 4],
) {
    let Some((player_x, player_y, icon_size)) =
        renderer.player_sprite_rect_at(&scene.viewport, position)
    else {
        return;
    };
    let icon = renderer.standard_player_bitmap(icon_size);
    let tinted = tint_premultiplied_rgba(icon, color);
    blit_rgba(
        frame, width, height, &tinted, icon_size, icon_size, player_x, player_y,
    );
}

fn draw_tinted_box(
    renderer: &mut Renderer,
    frame: &mut [u8],
    width: u32,
    height: u32,
    scene: &GameplayScreenRequest,
    position: (u32, u32),
    color: [u8; 4],
) {
    let Some((box_x, box_y, icon_size)) = renderer.box_sprite_rect_at(&scene.viewport, position)
    else {
        return;
    };
    let icon = renderer.standard_box_bitmap(icon_size);
    let tinted = tint_premultiplied_rgba(icon, color);
    blit_rgba(
        frame, width, height, &tinted, icon_size, icon_size, box_x, box_y,
    );
}

fn tint_premultiplied_rgba(bitmap: &[u8], color: [u8; 4]) -> Vec<u8> {
    let mut tinted = Vec::with_capacity(bitmap.len());
    for pixel in bitmap.chunks_exact(4) {
        let alpha = (u16::from(pixel[3]) * u16::from(color[3]) / 255) as u8;
        tinted.push((u16::from(color[0]) * u16::from(alpha) / 255) as u8);
        tinted.push((u16::from(color[1]) * u16::from(alpha) / 255) as u8);
        tinted.push((u16::from(color[2]) * u16::from(alpha) / 255) as u8);
        tinted.push(alpha);
    }
    tinted
}
