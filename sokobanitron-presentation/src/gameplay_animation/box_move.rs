use super::box_path_drawing::{
    FULL_BOX_PATH_LINE_WIDTH_CELL_FRACTION, draw_limited_box_path_outline, draw_path_from_progress,
};
use super::entity_flash::{
    EntityFlashPhase, FlashTargets, draw_flashing_entities, entity_flash_duration,
    entity_flash_phase_for_elapsed, flash_color, flash_dirty_cells, flash_targets_from_scenes,
};
use super::{GameplayAnimation, animation_tick_duration};
use crate::gameplay_animation::GameplayAnimationPolicy;
use crate::renderer::Renderer;
use crate::screen_requests::{
    GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenRequest,
};
use sokobanitron_gameplay::BoardCell;
use std::time::Duration;

const SPEED_SCALE: f32 = 1.8;
const SPEED_EXPONENT: f32 = 0.7;
const LIMITED_BOX_MOVE_MAX_FRAMES: usize = 3;

pub(super) fn box_move_animation_for_policy(
    policy: GameplayAnimationPolicy,
    previous_scene: Option<&GameplayScreenRequest>,
    update: &GameplayPresentationUpdate,
) -> Option<Box<dyn GameplayAnimation>> {
    let GameplayPresentationCause::BoxMoved { path } = &update.cause else {
        return None;
    };

    match policy {
        GameplayAnimationPolicy::Full => Some(Box::new(FullBoxMoveAnimation::new(
            previous_scene?,
            &update.scene,
            path.clone(),
        )?)),
        GameplayAnimationPolicy::Limited if box_move_path_is_visible_for_policy(policy, path) => {
            Some(Box::new(LimitedBoxMoveAnimation::new(path.clone())))
        }
        GameplayAnimationPolicy::Limited => None,
    }
}

pub(super) fn box_move_path_is_visible_for_policy(
    policy: GameplayAnimationPolicy,
    path: &[BoardCell],
) -> bool {
    match policy {
        GameplayAnimationPolicy::Full => path.len() > 2,
        GameplayAnimationPolicy::Limited => limited_box_move_has_visible_samples(path),
    }
}

fn limited_box_move_has_visible_samples(path: &[BoardCell]) -> bool {
    limited_box_move_sample_cells(path).next().is_some()
}

pub(super) struct FullBoxMoveAnimation {
    flash_targets: FlashTargets,
    path: Option<Vec<BoardCell>>,
    flash_phase: EntityFlashPhase,
    cleanup_progress_segments: f32,
    cleanup_duration: Duration,
    speed_per_tick: f32,
    total_segments: usize,
    duration: Duration,
}

pub(super) struct LimitedBoxMoveAnimation {
    sample_cells: Vec<BoardCell>,
    frame_count: usize,
    frame_index: usize,
}

impl FullBoxMoveAnimation {
    pub(super) fn new(
        previous_scene: &GameplayScreenRequest,
        current_scene: &GameplayScreenRequest,
        path: Vec<BoardCell>,
    ) -> Option<Self> {
        let flash_targets = flash_targets_from_scenes(previous_scene, current_scene)?;
        let path = box_move_path_is_visible_for_policy(GameplayAnimationPolicy::Full, &path)
            .then_some(path);
        let total_segments = path
            .as_ref()
            .map(|path| path.len().saturating_sub(1))
            .unwrap_or(0);
        let total = total_segments as f32;
        let speed_per_tick = SPEED_SCALE * total.powf(SPEED_EXPONENT);
        let cleanup_duration = cleanup_duration(total_segments, speed_per_tick);
        Some(Self {
            flash_targets,
            path,
            flash_phase: EntityFlashPhase::FlashDark,
            cleanup_progress_segments: 0.0,
            cleanup_duration,
            speed_per_tick,
            total_segments,
            duration: entity_flash_duration().max(cleanup_duration),
        })
    }

    fn current_path_dirty_cells(&self) -> Vec<BoardCell> {
        if let Some(path) = self.path.as_ref() {
            full_box_move_path_visible_cells(path, self.cleanup_progress_segments)
        } else {
            Vec::new()
        }
    }

    fn progress_for_elapsed(&self, elapsed: Duration) -> f32 {
        if self.total_segments == 0 {
            0.0
        } else if elapsed >= self.cleanup_duration {
            self.total_segments as f32
        } else {
            ((elapsed.as_secs_f32() / animation_tick_duration(1).as_secs_f32())
                * self.speed_per_tick)
                .min(self.total_segments as f32)
        }
    }
}

impl LimitedBoxMoveAnimation {
    pub(super) fn new(path: Vec<BoardCell>) -> Self {
        let sample_cells: Vec<BoardCell> = limited_box_move_sample_cells(&path).collect();
        let frame_count = limited_box_move_frame_count(sample_cells.len());
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

impl GameplayAnimation for FullBoxMoveAnimation {
    fn hides_player(&self) -> bool {
        true
    }

    fn dirty_cells(&self) -> Vec<BoardCell> {
        let mut dirty = self.current_path_dirty_cells();
        if matches!(
            self.flash_phase,
            EntityFlashPhase::FlashDark | EntityFlashPhase::FlashLight
        ) {
            dirty.extend(flash_dirty_cells(&self.flash_targets));
        }
        normalize_cells(dirty)
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
        let Some(path) = self.path.as_ref() else {
            return;
        };
        draw_path_from_progress(
            renderer,
            frame,
            (width, height),
            scene,
            clip_cell,
            path,
            self.cleanup_progress_segments,
        );
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
        let Some(color) = flash_color(self.flash_phase, renderer) else {
            return;
        };
        draw_flashing_entities(
            &self.flash_targets,
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
        self.duration
    }

    fn set_elapsed(&mut self, elapsed: Duration) {
        self.flash_phase = entity_flash_phase_for_elapsed(elapsed);
        self.cleanup_progress_segments = self.progress_for_elapsed(elapsed);
    }

    fn advance_to_elapsed(&mut self, elapsed: Duration) -> Vec<BoardCell> {
        let previous_flash_phase = self.flash_phase;
        let previous_progress = self.cleanup_progress_segments;
        self.set_elapsed(elapsed);

        let mut dirty = Vec::new();
        if previous_flash_phase != self.flash_phase {
            dirty.extend(flash_dirty_cells(&self.flash_targets));
        }
        if let Some(path) = self.path.as_ref() {
            dirty.extend(full_box_move_path_interval_cells(
                path,
                previous_progress,
                self.cleanup_progress_segments,
            ));
        }
        normalize_cells(dirty)
    }
}

fn cleanup_duration(total_segments: usize, speed_per_tick: f32) -> Duration {
    if total_segments == 0 || speed_per_tick <= 0.0 {
        Duration::ZERO
    } else {
        Duration::from_secs_f32(
            (total_segments as f32 / speed_per_tick) * animation_tick_duration(1).as_secs_f32(),
        )
    }
}

pub(super) fn full_box_move_path_visible_cells(
    path: &[BoardCell],
    path_progress_segments: f32,
) -> Vec<BoardCell> {
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

pub(super) fn full_box_move_path_interval_cells(
    path: &[BoardCell],
    from_progress_segments: f32,
    to_progress_segments: f32,
) -> Vec<BoardCell> {
    let total_segments = path.len().saturating_sub(1);
    if total_segments == 0 {
        return Vec::new();
    }
    let from = from_progress_segments.clamp(0.0, total_segments as f32);
    let to = to_progress_segments.clamp(0.0, total_segments as f32);
    if to <= from {
        return Vec::new();
    }

    let first_segment = from
        .floor()
        .clamp(0.0, (total_segments.saturating_sub(1)) as f32) as usize;
    let last_segment = if to >= total_segments as f32 {
        total_segments - 1
    } else {
        to.floor()
            .clamp(0.0, (total_segments.saturating_sub(1)) as f32) as usize
    };
    normalize_cells(path[first_segment..=(last_segment + 1)].to_vec())
}

impl GameplayAnimation for LimitedBoxMoveAnimation {
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

    fn duration(&self) -> Duration {
        animation_tick_duration(self.frame_count as u32)
    }

    fn set_elapsed(&mut self, elapsed: Duration) {
        let tick_nanos = animation_tick_duration(1).as_nanos();
        let elapsed_ticks = if tick_nanos == 0 {
            self.frame_count
        } else {
            (elapsed.as_nanos() / tick_nanos) as usize
        };
        self.frame_index = elapsed_ticks.min(self.frame_count);
    }

    fn advance_to_elapsed(&mut self, elapsed: Duration) -> Vec<BoardCell> {
        let previous_frame_index = self.frame_index;
        let previous_dirty = self.dirty_cells();
        self.set_elapsed(elapsed);
        if previous_frame_index == self.frame_index {
            Vec::new()
        } else {
            let mut dirty = previous_dirty;
            dirty.extend(self.dirty_cells());
            normalize_cells(dirty)
        }
    }
}

fn limited_box_move_frame_count(sample_count: usize) -> usize {
    if sample_count == 0 {
        0
    } else {
        sample_count
            .div_ceil(LIMITED_BOX_MOVE_MAX_FRAMES)
            .min(LIMITED_BOX_MOVE_MAX_FRAMES)
    }
}

fn limited_box_move_sample_cells(path: &[BoardCell]) -> impl Iterator<Item = BoardCell> + '_ {
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
    use super::{
        FullBoxMoveAnimation, LimitedBoxMoveAnimation, full_box_move_path_interval_cells,
        full_box_move_path_visible_cells, limited_box_move_sample_cells,
    };
    use crate::gameplay_animation::GameplayAnimation;
    use crate::gameplay_animation::entity_flash::EntityFlashPhase;
    use crate::layout::fit_board_viewport_for_controls;
    use crate::screen_requests::{GameplayScreenMode, GameplayScreenRequest};
    use sokobanitron_gameplay::{BoardCell, BoardSolveState, BoardView, TileKind};

    #[test]
    fn limited_box_move_samples_skip_first_second_to_last_and_last_cells() {
        let path = vec![
            BoardCell::new(0, 0),
            BoardCell::new(1, 0),
            BoardCell::new(2, 0),
            BoardCell::new(3, 0),
            BoardCell::new(4, 0),
        ];

        let samples: Vec<BoardCell> = limited_box_move_sample_cells(&path).collect();

        assert_eq!(samples, vec![BoardCell::new(1, 0), BoardCell::new(2, 0)]);
    }

    #[test]
    fn full_box_move_path_dirty_cells_drop_consumed_prefix() {
        let path = vec![
            BoardCell::new(0, 0),
            BoardCell::new(1, 0),
            BoardCell::new(2, 0),
            BoardCell::new(3, 0),
        ];

        assert_eq!(full_box_move_path_visible_cells(&path, 0.0), path);
        assert_eq!(
            full_box_move_path_visible_cells(&path, 1.0),
            vec![
                BoardCell::new(1, 0),
                BoardCell::new(2, 0),
                BoardCell::new(3, 0),
            ]
        );
        assert_eq!(
            full_box_move_path_visible_cells(&path, 1.61),
            vec![BoardCell::new(2, 0), BoardCell::new(3, 0)]
        );
        assert_eq!(
            full_box_move_path_visible_cells(&path, 3.0),
            Vec::<BoardCell>::new()
        );
    }

    #[test]
    fn full_box_move_cleanup_dirty_cells_match_visible_path_footprint() {
        let path = vec![
            BoardCell::new(0, 0),
            BoardCell::new(1, 0),
            BoardCell::new(2, 0),
            BoardCell::new(3, 0),
        ];
        let board = BoardView::new(
            4,
            1,
            vec![TileKind::Floor; 4],
            vec![false, false, true, false],
            Some(BoardCell::new(1, 0)),
            None,
            BoardSolveState::Unsolved,
        );
        let previous_scene = GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(96, 64, &board),
            board: board.clone(),
            level_number: 1,
            mode: GameplayScreenMode::Normal,
            sleeping_player: false,
        };
        let current_board = BoardView::new(
            4,
            1,
            vec![TileKind::Floor; 4],
            vec![false, false, false, true],
            Some(BoardCell::new(2, 0)),
            None,
            BoardSolveState::Unsolved,
        );
        let current_scene = GameplayScreenRequest {
            viewport: fit_board_viewport_for_controls(96, 64, &current_board),
            board: current_board,
            level_number: 1,
            mode: GameplayScreenMode::Normal,
            sleeping_player: false,
        };
        let mut animation =
            FullBoxMoveAnimation::new(&previous_scene, &current_scene, path.clone()).unwrap();

        assert_eq!(animation.dirty_cells(), path);

        animation.flash_phase = EntityFlashPhase::Complete;
        animation.cleanup_progress_segments = 1.61;

        assert_eq!(
            animation.dirty_cells(),
            vec![BoardCell::new(2, 0), BoardCell::new(3, 0)]
        );
    }

    #[test]
    fn limited_box_move_dirty_cells_match_visible_sample_cells() {
        let path = vec![
            BoardCell::new(0, 0),
            BoardCell::new(1, 0),
            BoardCell::new(2, 0),
            BoardCell::new(3, 0),
            BoardCell::new(4, 0),
        ];
        let animation = LimitedBoxMoveAnimation::new(path);

        assert_eq!(
            animation.dirty_cells(),
            vec![BoardCell::new(1, 0), BoardCell::new(2, 0)]
        );
    }

    #[test]
    fn full_box_move_path_interval_dirty_cells_cover_disappeared_strip() {
        let path = vec![
            BoardCell::new(0, 0),
            BoardCell::new(1, 0),
            BoardCell::new(2, 0),
            BoardCell::new(3, 0),
        ];

        assert_eq!(
            full_box_move_path_interval_cells(&path, 0.2, 0.4),
            vec![BoardCell::new(0, 0), BoardCell::new(1, 0)]
        );
        assert_eq!(
            full_box_move_path_interval_cells(&path, 0.9, 1.1),
            vec![
                BoardCell::new(0, 0),
                BoardCell::new(1, 0),
                BoardCell::new(2, 0)
            ]
        );
        assert_eq!(
            full_box_move_path_interval_cells(&path, 2.2, 3.0),
            vec![BoardCell::new(2, 0), BoardCell::new(3, 0)]
        );
    }
}
