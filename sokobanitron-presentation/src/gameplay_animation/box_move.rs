use super::box_path_drawing::{draw_limited_box_path_outline, draw_path_from_progress};
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
        Some(Self {
            flash_targets,
            path,
            flash_phase: EntityFlashPhase::FlashDark,
        })
    }

    fn current_path_dirty_cells(&self) -> Vec<BoardCell> {
        if self.path_is_visible() {
            self.path.as_ref().cloned().unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    fn path_is_visible(&self) -> bool {
        self.path.is_some()
            && matches!(
                self.flash_phase,
                EntityFlashPhase::FlashDark | EntityFlashPhase::FlashLight
            )
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
            0.0,
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
        entity_flash_duration()
    }

    fn set_elapsed(&mut self, elapsed: Duration) {
        self.flash_phase = entity_flash_phase_for_elapsed(elapsed);
    }

    fn advance_to_elapsed(&mut self, elapsed: Duration) -> Vec<BoardCell> {
        let previous_flash_phase = self.flash_phase;
        let previous_path_visible = self.path_is_visible();
        self.set_elapsed(elapsed);

        let mut dirty = Vec::new();
        if previous_flash_phase != self.flash_phase {
            dirty.extend(flash_dirty_cells(&self.flash_targets));
        }
        if previous_path_visible != self.path_is_visible()
            && let Some(path) = self.path.as_ref()
        {
            dirty.extend(path.iter().copied());
        }
        normalize_cells(dirty)
    }
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
        let elapsed_ticks = elapsed
            .as_nanos()
            .checked_div(tick_nanos)
            .map_or(self.frame_count, |ticks| ticks as usize);
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
    use super::{FullBoxMoveAnimation, LimitedBoxMoveAnimation, limited_box_move_sample_cells};
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
    fn full_box_move_dirty_cells_clear_path_when_flash_completes() {
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

        assert_eq!(animation.dirty_cells(), Vec::<BoardCell>::new());
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
}
