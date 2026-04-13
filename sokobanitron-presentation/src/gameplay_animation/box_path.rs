use super::GameplayAnimation;
use crate::screen_requests::GameplayScreenRequest;
use resvg::tiny_skia::{LineCap, LineJoin, Paint, PathBuilder, PixmapMut, Stroke, Transform};
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

        let mut builder = PathBuilder::new();
        builder.move_to(start_x, start_y);
        for &(x, y) in &points[(start_segment + 1)..] {
            builder.line_to(x, y);
        }
        let Some(path) = builder.finish() else {
            return;
        };

        let mut paint = Paint::default();
        paint.set_color_rgba8(PATH_COLOR[0], PATH_COLOR[1], PATH_COLOR[2], PATH_COLOR[3]);
        paint.anti_alias = true;

        let stroke = Stroke {
            width: scene.viewport.cell_size as f32 * 0.2,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };
        let mut pixmap =
            PixmapMut::from_bytes(frame, width, height).expect("frame dimensions must be valid");
        pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
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
