use super::{GameplayAnimation, animation_tick_duration};
use crate::gameplay_animation::GameplayAnimationPolicy;
use crate::renderer::{Renderer, blit_premultiplied_gray_alpha, premultiply_straight_gray};
use crate::screen_requests::{
    GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenRequest,
};
use sokobanitron_gameplay::BoardCell;
use std::time::Duration;

const FLASH_DARK_TICKS: u32 = 1;
const FLASH_LIGHT_TICKS: u32 = 1;

pub(super) struct EntityFlashAnimation {
    targets: FlashTargets,
    hide_player: bool,
    phase: EntityFlashPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntityFlashPhase {
    FlashDark,
    FlashLight,
    Complete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FlashTargets {
    player_position: BoardCell,
    box_positions: Vec<BoardCell>,
}

pub(super) fn flash_targets_from_scenes(
    previous: &GameplayScreenRequest,
    current: &GameplayScreenRequest,
) -> Option<FlashTargets> {
    let player_position = previous.board.player()?;
    let player_changed = Some(player_position) != current.board.player();
    let box_positions = removed_box_positions(previous, current);
    if !player_changed && box_positions.is_empty() {
        return None;
    }
    Some(FlashTargets {
        player_position,
        box_positions,
    })
}

pub(super) fn flash_color(phase: EntityFlashPhase, renderer: &Renderer) -> Option<u8> {
    match phase {
        EntityFlashPhase::FlashDark => Some(renderer.theme.gray_13),
        EntityFlashPhase::FlashLight => Some(renderer.theme.gray_1),
        EntityFlashPhase::Complete => None,
    }
}

pub(super) fn entity_flash_duration() -> Duration {
    animation_tick_duration(FLASH_DARK_TICKS + FLASH_LIGHT_TICKS)
}

pub(super) fn entity_flash_phase_for_elapsed(elapsed: Duration) -> EntityFlashPhase {
    if elapsed < animation_tick_duration(FLASH_DARK_TICKS) {
        EntityFlashPhase::FlashDark
    } else if elapsed < entity_flash_duration() {
        EntityFlashPhase::FlashLight
    } else {
        EntityFlashPhase::Complete
    }
}

pub(super) fn flash_dirty_cells(targets: &FlashTargets) -> Vec<BoardCell> {
    let mut dirty = targets.box_positions.clone();
    dirty.push(targets.player_position);
    dirty.sort_by_key(|cell| (cell.y, cell.x));
    dirty.dedup();
    dirty
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_flashing_entities(
    targets: &FlashTargets,
    renderer: &mut Renderer,
    frame: &mut [u8],
    width: u32,
    height: u32,
    scene: &GameplayScreenRequest,
    clip_cell: Option<BoardCell>,
    color: u8,
) {
    if clip_cell.is_none_or(|cell| cell == targets.player_position) {
        draw_tinted_player(
            renderer,
            frame,
            width,
            height,
            scene,
            targets.player_position,
            color,
        );
    }
    for &position in &targets.box_positions {
        if clip_cell.is_none_or(|cell| cell == position) {
            draw_tinted_box(renderer, frame, width, height, scene, position, color);
        }
    }
}

pub(super) fn entity_flash_animation_for_policy(
    policy: GameplayAnimationPolicy,
    previous_scene: Option<&GameplayScreenRequest>,
    update: &GameplayPresentationUpdate,
) -> Option<Box<dyn GameplayAnimation>> {
    if policy != GameplayAnimationPolicy::Full {
        return None;
    }

    let previous_scene = previous_scene?;
    let targets = flash_targets_from_scenes(previous_scene, &update.scene)?;
    let animation = EntityFlashAnimation {
        targets,
        hide_player: matches!(update.cause, GameplayPresentationCause::BoxMoved { .. }),
        phase: EntityFlashPhase::FlashDark,
    };
    Some(Box::new(animation))
}

impl GameplayAnimation for EntityFlashAnimation {
    fn hides_player(&self) -> bool {
        self.hide_player
    }

    fn dirty_cells(&self) -> Vec<BoardCell> {
        match self.phase {
            EntityFlashPhase::FlashDark | EntityFlashPhase::FlashLight => {
                flash_dirty_cells(&self.targets)
            }
            EntityFlashPhase::Complete => Vec::new(),
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
        let Some(color) = flash_color(self.phase, renderer) else {
            return;
        };
        draw_flashing_entities(
            &self.targets,
            renderer,
            frame,
            width,
            height,
            scene,
            clip_cell,
            color,
        );
    }

    fn duration(&self) -> Duration {
        entity_flash_duration()
    }

    fn set_elapsed(&mut self, elapsed: Duration) {
        self.phase = entity_flash_phase_for_elapsed(elapsed);
    }

    fn advance_to_elapsed(&mut self, elapsed: Duration) -> Vec<BoardCell> {
        let previous_phase = self.phase;
        self.set_elapsed(elapsed);
        if previous_phase == self.phase {
            Vec::new()
        } else {
            flash_dirty_cells(&self.targets)
        }
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
    color: u8,
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
    color: u8,
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
/// `bitmap` is `[premultiplied_gray, alpha]` pairs. The tint color is straight grayscale.
fn tint_premultiplied_gray_alpha(bitmap: &[u8], color: u8) -> Vec<u8> {
    let mut tinted = Vec::with_capacity(bitmap.len());
    for pixel in bitmap.chunks_exact(2) {
        tinted.push(premultiply_straight_gray(color, pixel[1]));
        tinted.push(pixel[1]);
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
        let tinted = tint_premultiplied_gray_alpha(&bitmap, 200);
        let mut frame = vec![100];

        blit_premultiplied_gray_alpha(&mut frame, 1, 1, &tinted, 1, 1, 0, 0);

        assert_eq!(tinted, vec![100, 128]);
        assert_eq!(frame, vec![149]);
    }
}
