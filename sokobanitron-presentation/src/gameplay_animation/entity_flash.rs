use super::GameplayAnimation;
use crate::renderer::{
    Renderer, blit_premultiplied_gray_alpha, premultiply_straight_gray, rgba_to_gray,
};
use crate::screen_requests::GameplayScreenRequest;
use sokobanitron_gameplay::BoardCell;

const FLASH_DARK_COLOR: [u8; 4] = [142, 142, 142, 255];
const FLASH_LIGHT_COLOR: [u8; 4] = [242, 242, 242, 255];

pub(super) struct EntityFlashAnimation {
    player_position: BoardCell,
    box_positions: Vec<BoardCell>,
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
) -> Vec<BoardCell> {
    let mut removed = Vec::new();
    for cell in previous.board.cells() {
        let current_has_box = cell.x < current.board.width()
            && cell.y < current.board.height()
            && current.board.has_box(cell);
        if previous.board.has_box(cell) && !current_has_box {
            removed.push(cell);
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
    position: BoardCell,
    color: [u8; 4],
) {
    let Some((player_x, player_y, icon_size)) =
        renderer.player_sprite_rect_at(&scene.viewport, position)
    else {
        return;
    };
    let icon = renderer.standard_player_bitmap(icon_size);
    let tinted = tint_premultiplied_gray_alpha(icon, color);
    blit_premultiplied_gray_alpha(
        frame, width, height, &tinted, icon_size, icon_size, player_x, player_y,
    );
}

fn draw_tinted_box(
    renderer: &mut Renderer,
    frame: &mut [u8],
    width: u32,
    height: u32,
    scene: &GameplayScreenRequest,
    position: BoardCell,
    color: [u8; 4],
) {
    let Some((box_x, box_y, icon_size)) = renderer.box_sprite_rect_at(&scene.viewport, position)
    else {
        return;
    };
    let icon = renderer.standard_box_bitmap(icon_size);
    let tinted = tint_premultiplied_gray_alpha(icon, color);
    blit_premultiplied_gray_alpha(
        frame, width, height, &tinted, icon_size, icon_size, box_x, box_y,
    );
}

/// Recolors a premultiplied gray+alpha bitmap and returns the same premultiplied format.
///
/// `bitmap` is `[premultiplied_gray, alpha]` pairs. The tint color is straight RGBA; its alpha
/// modulates the bitmap alpha before the output gray channel is premultiplied.
fn tint_premultiplied_gray_alpha(bitmap: &[u8], color: [u8; 4]) -> Vec<u8> {
    let color_alpha = color[3];
    let color = rgba_to_gray(color);
    let mut tinted = Vec::with_capacity(bitmap.len());
    for pixel in bitmap.chunks_exact(2) {
        let alpha = (u16::from(pixel[1]) * u16::from(color_alpha) / 255) as u8;
        tinted.push(premultiply_straight_gray(color, alpha));
        tinted.push(alpha);
    }
    tinted
}

#[cfg(test)]
mod tests {
    use super::tint_premultiplied_gray_alpha;
    use crate::renderer::blit_premultiplied_gray_alpha;

    #[test]
    fn tint_premultiplied_gray_alpha_preserves_composited_alpha_behavior() {
        let bitmap = vec![200, 128];
        let tinted = tint_premultiplied_gray_alpha(&bitmap, [200, 200, 200, 128]);
        let mut frame = vec![100];

        blit_premultiplied_gray_alpha(&mut frame, 1, 1, &tinted, 1, 1, 0, 0);

        assert_eq!(tinted, vec![50, 64]);
        assert_eq!(frame, vec![124]);
    }
}
