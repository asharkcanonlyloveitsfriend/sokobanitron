use super::GameplayAnimation;
use super::box_path_drawing::{
    FULL_BOX_PATH_LINE_WIDTH_CELL_FRACTION, draw_limited_box_path_outline, draw_path_from_progress,
};
use crate::gameplay_animation::GameplayAnimationPolicy;
use crate::renderer::Renderer;
use crate::screen_requests::GameplayScreenRequest;
use sokobanitron_gameplay::BoardCell;

const SPEED_SCALE: f32 = 1.3;
const SPEED_EXPONENT: f32 = 0.5;
const LIMITED_BOX_PATH_MAX_FRAMES: usize = 3;

pub(super) fn box_path_animation_for_policy(
    policy: GameplayAnimationPolicy,
    path: Vec<BoardCell>,
) -> Option<Box<dyn GameplayAnimation>> {
    match policy {
        GameplayAnimationPolicy::Full if path.len() > 2 => {
            Some(Box::new(BoxPathAnimation::new(path)))
        }
        GameplayAnimationPolicy::Limited if limited_box_path_has_visible_samples(&path) => {
            Some(Box::new(LimitedBoxPathAnimation::new(path)))
        }
        _ => None,
    }
}

fn limited_box_path_has_visible_samples(path: &[BoardCell]) -> bool {
    limited_box_path_sample_cells(path).next().is_some()
}

pub(super) struct BoxPathAnimation {
    path: Vec<BoardCell>,
    path_progress_segments: f32,
    speed_per_tick: f32,
    total_segments: usize,
}

pub(super) struct LimitedBoxPathAnimation {
    sample_cells: Vec<BoardCell>,
    frame_count: usize,
    frame_index: usize,
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

impl LimitedBoxPathAnimation {
    pub(super) fn new(path: Vec<BoardCell>) -> Self {
        let sample_cells: Vec<BoardCell> = limited_box_path_sample_cells(&path).collect();
        let frame_count = limited_box_path_frame_count(sample_cells.len());
        Self {
            sample_cells,
            frame_count,
            frame_index: 0,
        }
    }

    fn current_sample_cells(&self) -> &[BoardCell] {
        if self.frame_index >= self.frame_count || self.sample_cells.is_empty() {
            return &[];
        }
        let start = (self.frame_index * self.sample_cells.len()) / self.frame_count;
        let end = ((self.frame_index + 1) * self.sample_cells.len()) / self.frame_count;
        &self.sample_cells[start..end]
    }
}

impl GameplayAnimation for BoxPathAnimation {
    fn hides_player(&self) -> bool {
        true
    }

    fn dirty_cells(&self) -> Vec<BoardCell> {
        full_box_path_visible_cells(&self.path, self.path_progress_segments)
    }

    fn draw_under_entities(
        &self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        scene: &GameplayScreenRequest,
        clip_cell: Option<BoardCell>,
    ) {
        draw_path_from_progress(
            renderer,
            frame,
            (width, height),
            scene,
            clip_cell,
            &self.path,
            self.path_progress_segments,
        );
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

fn full_box_path_visible_cells(path: &[BoardCell], path_progress_segments: f32) -> Vec<BoardCell> {
    let total_segments = path.len().saturating_sub(1);
    if total_segments == 0 {
        return Vec::new();
    }
    let consumed = path_progress_segments.min(total_segments as f32);
    if consumed >= total_segments as f32 {
        return Vec::new();
    }

    let start_segment = consumed
        .floor()
        .clamp(0.0, (total_segments.saturating_sub(1)) as f32) as usize;
    let start_fraction = consumed - start_segment as f32;
    // The stroke is centered on the segment centerline and is `line_width / 2` thick on each
    // side. Once the consumed front edge has crossed the next cell boundary by half the stroke
    // width, the previous cell is no longer touched by the visible path footprint.
    let leading_overlap_threshold = 0.5 + FULL_BOX_PATH_LINE_WIDTH_CELL_FRACTION / 2.0;
    let first_visible_index = if start_fraction >= leading_overlap_threshold {
        start_segment + 1
    } else {
        start_segment
    };

    normalize_cells(path[first_visible_index..].to_vec())
}

impl GameplayAnimation for LimitedBoxPathAnimation {
    fn dirty_cells(&self) -> Vec<BoardCell> {
        self.current_sample_cells().to_vec()
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
        for &cell in self.current_sample_cells() {
            if clip_cell.is_none_or(|clip| clip == cell) {
                draw_limited_box_path_outline(renderer, frame, width, height, scene, cell);
            }
        }
    }

    fn ticks_until_next_step(&self) -> Option<u32> {
        (self.frame_index < self.frame_count).then_some(1)
    }

    fn step(&mut self) {
        self.frame_index += 1;
    }
}

fn limited_box_path_frame_count(sample_count: usize) -> usize {
    if sample_count == 0 {
        0
    } else {
        sample_count
            .div_ceil(LIMITED_BOX_PATH_MAX_FRAMES)
            .min(LIMITED_BOX_PATH_MAX_FRAMES)
    }
}

fn limited_box_path_sample_cells(path: &[BoardCell]) -> impl Iterator<Item = BoardCell> + '_ {
    path.iter()
        .copied()
        .skip(1)
        .take(path.len().saturating_sub(3))
}

fn normalize_cells(mut cells: Vec<BoardCell>) -> Vec<BoardCell> {
    cells.sort_by_key(|cell| (cell.y, cell.x));
    cells.dedup();
    cells
}

#[cfg(test)]
mod tests {
    use super::{BoxPathAnimation, full_box_path_visible_cells, limited_box_path_sample_cells};
    use crate::gameplay_animation::GameplayAnimation;
    use sokobanitron_gameplay::BoardCell;

    #[test]
    fn limited_box_path_samples_skip_first_second_to_last_and_last_cells() {
        let path = vec![
            BoardCell::new(0, 0),
            BoardCell::new(1, 0),
            BoardCell::new(2, 0),
            BoardCell::new(3, 0),
            BoardCell::new(4, 0),
        ];

        let samples: Vec<BoardCell> = limited_box_path_sample_cells(&path).collect();

        assert_eq!(samples, vec![BoardCell::new(1, 0), BoardCell::new(2, 0)]);
    }

    #[test]
    fn full_box_path_dirty_cells_drop_consumed_prefix() {
        let path = vec![
            BoardCell::new(0, 0),
            BoardCell::new(1, 0),
            BoardCell::new(2, 0),
            BoardCell::new(3, 0),
        ];

        assert_eq!(full_box_path_visible_cells(&path, 0.0), path);
        assert_eq!(
            full_box_path_visible_cells(&path, 1.0),
            vec![
                BoardCell::new(1, 0),
                BoardCell::new(2, 0),
                BoardCell::new(3, 0),
            ]
        );
        assert_eq!(
            full_box_path_visible_cells(&path, 1.61),
            vec![BoardCell::new(2, 0), BoardCell::new(3, 0)]
        );
        assert_eq!(
            full_box_path_visible_cells(&path, 3.0),
            Vec::<BoardCell>::new()
        );
    }

    #[test]
    fn full_box_path_animation_dirty_cells_match_visible_path_footprint() {
        let path = vec![
            BoardCell::new(0, 0),
            BoardCell::new(1, 0),
            BoardCell::new(2, 0),
            BoardCell::new(3, 0),
        ];
        let mut animation = BoxPathAnimation::new(path.clone());

        assert_eq!(animation.dirty_cells(), path);

        animation.path_progress_segments = 1.61;

        assert_eq!(
            animation.dirty_cells(),
            vec![BoardCell::new(2, 0), BoardCell::new(3, 0)]
        );
    }
}
