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

    fn dirty_cells(&self) -> Vec<BoardCell> {
        if self.path_progress_segments >= self.total_segments as f32 {
            Vec::new()
        } else {
            normalize_cells(self.path.clone())
        }
    }

    fn draw_under_entities(
        &self,
        _renderer: &mut crate::renderer::Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        scene: &GameplayScreenRequest,
        clip_cell: Option<BoardCell>,
    ) {
        if self.total_segments == 0 {
            return;
        }
        let clip_rect = clip_cell.map(|cell| scene.viewport.cell_to_screen_rect(cell));
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
            draw_thick_line(
                frame, width, height, previous, next, line_width, PATH_COLOR, clip_rect,
            );
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

#[allow(clippy::too_many_arguments)]
fn draw_thick_line(
    frame: &mut [u8],
    width: u32,
    height: u32,
    start: (f32, f32),
    end: (f32, f32),
    thickness: f32,
    color: [u8; 4],
    clip_rect: Option<(i32, i32, u32, u32)>,
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
            clip_rect,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_filled_circle(
    frame: &mut [u8],
    width: u32,
    height: u32,
    cx: f32,
    cy: f32,
    radius: f32,
    color: [u8; 4],
    clip_rect: Option<(i32, i32, u32, u32)>,
) {
    let clip_left = clip_rect.map(|rect| rect.0.max(0) as u32).unwrap_or(0);
    let clip_top = clip_rect.map(|rect| rect.1.max(0) as u32).unwrap_or(0);
    let clip_right = clip_rect
        .map(|rect| (rect.0 + rect.2 as i32).clamp(0, width as i32) as u32)
        .unwrap_or(width);
    let clip_bottom = clip_rect
        .map(|rect| (rect.1 + rect.3 as i32).clamp(0, height as i32) as u32)
        .unwrap_or(height);
    if clip_left >= clip_right || clip_top >= clip_bottom {
        return;
    }
    let min_x = (cx - radius).floor().max(clip_left as f32) as u32;
    let max_x = (cx + radius)
        .ceil()
        .min(clip_right.saturating_sub(1) as f32) as u32;
    let min_y = (cy - radius).floor().max(clip_top as f32) as u32;
    let max_y = (cy + radius)
        .ceil()
        .min(clip_bottom.saturating_sub(1) as f32) as u32;
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

fn normalize_cells(mut cells: Vec<BoardCell>) -> Vec<BoardCell> {
    cells.sort_by_key(|cell| (cell.y, cell.x));
    cells.dedup();
    cells
}

#[cfg(test)]
mod tests {
    use super::draw_filled_circle;

    #[test]
    fn filled_circle_composites_alpha_into_gray_frame() {
        let mut frame = vec![100];

        draw_filled_circle(&mut frame, 1, 1, 0.5, 0.5, 1.0, [200, 200, 200, 128], None);

        assert_eq!(frame, vec![149]);
    }
}
