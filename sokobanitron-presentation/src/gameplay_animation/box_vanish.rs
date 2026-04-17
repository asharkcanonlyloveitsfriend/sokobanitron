use super::GameplayAnimation;
use crate::gameplay_animation::GameplayAnimationPolicy;
use crate::renderer::{Renderer, composite_straight_rgba_over_gray, fill_rect};
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

const LIMITED_BOX_VANISH_CORNER_RADIUS_DIVISOR: u32 = 5;
const STANDARD_BOX_BODY_SIZE_NUMERATOR: u32 = 72;
const STANDARD_BOX_BODY_SIZE_DENOMINATOR: u32 = 100;

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

fn draw_limited_vanishing_box_at(
    renderer: &Renderer,
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    scene: &GameplayScreenRequest,
    position: BoardCell,
    scale: f32,
) {
    if scale <= 0.0 {
        return;
    }
    let Some((box_x, box_y, icon_size)) = renderer.box_sprite_rect_at(&scene.viewport, position)
    else {
        return;
    };
    let body_size = standard_box_body_size(icon_size);
    let scaled_size = ((body_size as f32 * scale).round() as u32).max(1);
    let offset = ((icon_size as f32 - scaled_size as f32) / 2.0).round() as i32;
    draw_filled_rounded_rect(
        frame,
        frame_width,
        frame_height,
        box_x + offset,
        box_y + offset,
        scaled_size,
        scaled_size,
        renderer.theme.mid_3,
    );
}

fn standard_box_body_size(icon_size: u32) -> u32 {
    (icon_size * STANDARD_BOX_BODY_SIZE_NUMERATOR / STANDARD_BOX_BODY_SIZE_DENOMINATOR).max(1)
}

#[allow(clippy::too_many_arguments)]
fn draw_filled_rounded_rect(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: [u8; 4],
) {
    if w == 0 || h == 0 {
        return;
    }
    let radius = (w.min(h) / LIMITED_BOX_VANISH_CORNER_RADIUS_DIVISOR)
        .min(w / 2)
        .min(h / 2);
    if radius == 0 {
        fill_rect(frame, frame_width, frame_height, x, y, w, h, color);
        return;
    }

    fill_rect(
        frame,
        frame_width,
        frame_height,
        x + radius as i32,
        y,
        w.saturating_sub(radius.saturating_mul(2)),
        h,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y + radius as i32,
        w,
        h.saturating_sub(radius.saturating_mul(2)),
        color,
    );

    let radius = radius as f32;
    draw_filled_circle(
        frame,
        frame_width,
        frame_height,
        x as f32 + radius,
        y as f32 + radius,
        radius,
        color,
    );
    draw_filled_circle(
        frame,
        frame_width,
        frame_height,
        x as f32 + w as f32 - radius,
        y as f32 + radius,
        radius,
        color,
    );
    draw_filled_circle(
        frame,
        frame_width,
        frame_height,
        x as f32 + radius,
        y as f32 + h as f32 - radius,
        radius,
        color,
    );
    draw_filled_circle(
        frame,
        frame_width,
        frame_height,
        x as f32 + w as f32 - radius,
        y as f32 + h as f32 - radius,
        radius,
        color,
    );
}

fn draw_filled_circle(
    frame: &mut [u8],
    width: u32,
    height: u32,
    cx: f32,
    cy: f32,
    radius: f32,
    color: [u8; 4],
) {
    let min_x = (cx - radius).floor().max(0.0) as u32;
    let max_x = (cx + radius).ceil().min(width.saturating_sub(1) as f32) as u32;
    let min_y = (cy - radius).floor().max(0.0) as u32;
    let max_y = (cy + radius).ceil().min(height.saturating_sub(1) as f32) as u32;
    if min_x > max_x || min_y > max_y {
        return;
    }
    let radius_sq = radius * radius;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dist_sq = (px - cx) * (px - cx) + (py - cy) * (py - cy);
            if dist_sq <= radius_sq {
                let idx = (y * width + x) as usize;
                frame[idx] = composite_straight_rgba_over_gray(frame[idx], color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LIMITED_BOX_VANISH_PHASES, standard_box_body_size};

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

    #[test]
    fn limited_box_vanish_starts_from_standard_box_body_size() {
        assert_eq!(standard_box_body_size(100), 72);
        assert_eq!(standard_box_body_size(72), 51);
    }
}
