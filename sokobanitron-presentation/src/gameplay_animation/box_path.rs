use super::GameplayAnimation;
use crate::renderer::composite_straight_rgba_over_gray;
use crate::screen_requests::GameplayScreenRequest;
use sokobanitron_gameplay::BoardCell;

const SPEED_SCALE: f32 = 1.3;
const SPEED_EXPONENT: f32 = 0.5;
const PATH_COLOR: [u8; 4] = [211, 211, 211, 255];

pub(super) struct BoxPathAnimation {
    path: Vec<BoardCell>,
    path_progress_segments: f32,
    speed_per_tick: f32,
    total_segments: usize,
}

impl BoxPathAnimation {
    pub(super) fn new(path: Vec<BoardCell>) -> Self {
        let total_segments = path.len().saturating_sub(1);
        let total = total_segments as f32;
        Self {
            path,
            path_progress_segments: 0.0,
            speed_per_tick: SPEED_SCALE * total.powf(SPEED_EXPONENT),
            total_segments,
        }
    }
}

impl GameplayAnimation for BoxPathAnimation {
    fn hides_player(&self) -> bool {
        true
    }

    fn draw_under_entities(
        &self,
        _renderer: &mut crate::renderer::Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        scene: &GameplayScreenRequest,
    ) {
        if self.total_segments == 0 {
            return;
        }
        let consumed = self.path_progress_segments.min(self.total_segments as f32);
        let start_segment = consumed
            .floor()
            .clamp(0.0, (self.total_segments.saturating_sub(1)) as f32)
            as usize;
        let start_fraction = consumed - start_segment as f32;
        let points: Vec<(f32, f32)> = self
            .path
            .iter()
            .map(|&cell| {
                let (cell_x, cell_y, cell_w, cell_h) = scene.viewport.cell_to_screen_rect(cell);
                (
                    cell_x as f32 + cell_w as f32 / 2.0,
                    cell_y as f32 + cell_h as f32 / 2.0,
                )
            })
            .collect();
        if points.len() < 2 {
            return;
        }

        let (sx, sy) = points[start_segment];
        let (ex, ey) = points[start_segment + 1];
        let start_x = sx + (ex - sx) * start_fraction;
        let start_y = sy + (ey - sy) * start_fraction;

        // Use a deterministic grayscale-native stroke instead of the previous tiny-skia RGBA
        // path. This is a simplified round-brush approximation, not an antialiased vector stroke.
        // That tradeoff is acceptable for the transient box path cue and avoids reintroducing an
        // RGBA render target.
        let line_width = (scene.viewport.cell_size as f32 * 0.2).max(1.0);
        let mut previous = (start_x, start_y);
        for &next in &points[(start_segment + 1)..] {
            draw_thick_line(frame, width, height, previous, next, line_width, PATH_COLOR);
            previous = next;
        }
    }

    fn ticks_until_next_step(&self) -> Option<u32> {
        if self.path_progress_segments >= self.total_segments as f32 {
            None
        } else {
            Some(1)
        }
    }

    fn step(&mut self) {
        self.path_progress_segments += self.speed_per_tick;
    }
}

fn draw_thick_line(
    frame: &mut [u8],
    width: u32,
    height: u32,
    start: (f32, f32),
    end: (f32, f32),
    thickness: f32,
    color: [u8; 4],
) {
    let radius = thickness / 2.0;
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let distance = (dx * dx + dy * dy).sqrt();
    let steps = distance.ceil().max(1.0) as u32;
    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        draw_filled_circle(
            frame,
            width,
            height,
            start.0 + dx * t,
            start.1 + dy * t,
            radius,
            color,
        );
    }
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
    use super::draw_filled_circle;

    #[test]
    fn filled_circle_composites_alpha_into_gray_frame() {
        let mut frame = vec![100];

        draw_filled_circle(&mut frame, 1, 1, 0.5, 0.5, 1.0, [200, 200, 200, 128]);

        assert_eq!(frame, vec![149]);
    }
}
