use crate::command::{DrawTool, EditorCommand, EditorEffects, EditorMode};
use crate::snapshot::{
    BoxMoveCountSnapshot, EditorBoardSnapshot, EditorCellSnapshot, EditorSnapshot,
    PullHintSnapshot, PullHintStatus,
};
use crate::world::{EditableTile, EditableWorld};
use sokobanitron_core::optimizer::{
    ReverseOptimizationInput, optimize_reverse_solution_in_place,
};
use sokobanitron_core::pathfinder::{Position, PullPathfinder};
use std::collections::{HashMap, VecDeque};

struct PullMovePlan {
    player_start: (i32, i32),
    box_path: Vec<(i32, i32)>,
}

struct PullPlanningContext {
    grid: Vec<Vec<bool>>,
    min_x: i32,
    min_y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PullHintState {
    Pending,
    Ready(u32),
}

struct PullHintCandidate {
    destination: (i32, i32),
    box_path: Vec<(i32, i32)>,
}

#[derive(Clone, Copy)]
struct TrackedBox {
    start_index: usize,
    position: (i32, i32),
    count: u32,
}

struct ActivePullHintJob {
    generation: u64,
    selected: (i32, i32),
    selected_start_index: usize,
    optimization_input: ReverseOptimizationInput,
    pending_candidates: VecDeque<PullHintCandidate>,
}

#[derive(Clone)]
struct UndoSnapshot {
    world: EditableWorld,
    solution_start_boxes: Vec<(i32, i32)>,
    solution_start_player: Option<(i32, i32)>,
    solution_history: Vec<Vec<(i32, i32)>>,
}

pub struct LevelEditor {
    mode: EditorMode,
    selected_box: Option<(i32, i32)>,
    world: EditableWorld,
    solution_start_boxes: Vec<(i32, i32)>,
    solution_start_player: Option<(i32, i32)>,
    solution_history: Vec<Vec<(i32, i32)>>,
    undo_history: Vec<UndoSnapshot>,
    pull_destination_hints: HashMap<(i32, i32), PullHintState>,
    pull_hints_dirty: bool,
    pull_hint_generation: u64,
    active_pull_hint_job: Option<ActivePullHintJob>,
}

impl LevelEditor {
    pub fn new() -> Self {
        let world = EditableWorld::new();
        let solution_start_boxes = world.box_positions();
        let solution_start_player = world.player();
        Self {
            mode: EditorMode::Draw,
            selected_box: None,
            world,
            solution_start_boxes,
            solution_start_player,
            solution_history: Vec::new(),
            undo_history: Vec::new(),
            pull_destination_hints: HashMap::new(),
            pull_hints_dirty: false,
            pull_hint_generation: 0,
            active_pull_hint_job: None,
        }
    }

    pub fn world(&self) -> &EditableWorld {
        &self.world
    }

    pub fn mode(&self) -> EditorMode {
        self.mode
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_history.is_empty()
    }

    pub fn can_restart(&self) -> bool {
        !self.is_reset_state()
    }

    pub fn selected_box(&self) -> Option<(i32, i32)> {
        self.selected_box
    }

    pub fn snapshot(&self) -> EditorSnapshot {
        let bounds = self.world.non_void_bounds();
        let mut cells = Vec::new();
        if let Some(bounds) = bounds {
            for y in bounds.min_y..=bounds.max_y {
                for x in bounds.min_x..=bounds.max_x {
                    let tile = self.world.tile(x, y);
                    if !matches!(tile, EditableTile::Void) {
                        cells.push(EditorCellSnapshot {
                            world_x: x,
                            world_y: y,
                            tile,
                        });
                    }
                }
            }
        }

        let move_counts = self
            .box_move_counts_by_position()
            .into_iter()
            .map(|((world_x, world_y), count)| BoxMoveCountSnapshot {
                world_x,
                world_y,
                count,
            })
            .collect::<Vec<_>>();

        let mut pull_destination_hints = self
            .pull_destination_hints
            .iter()
            .map(|(&(world_x, world_y), state)| PullHintSnapshot {
                world_x,
                world_y,
                state: match state {
                    PullHintState::Pending => PullHintStatus::Pending,
                    PullHintState::Ready(count) => PullHintStatus::Ready(*count),
                },
            })
            .collect::<Vec<_>>();
        pull_destination_hints.sort_unstable_by_key(|hint| (hint.world_y, hint.world_x));

        EditorSnapshot {
            board: EditorBoardSnapshot {
                bounds,
                cells,
                player: self.world.player(),
            },
            mode: self.mode,
            selected_box: self.selected_box,
            pull_destination_hints,
            move_counts,
            can_undo: self.can_undo(),
            can_restart: self.can_restart(),
        }
    }

    pub fn apply_command(&mut self, command: EditorCommand) -> EditorEffects {
        let mut effects = EditorEffects::default();
        match command {
            EditorCommand::SetMode(mode) => {
                if self.mode != mode {
                    self.set_mode(mode);
                    effects.mode_changed = true;
                    effects.selection_changed = true;
                    effects.hints_changed = true;
                }
            }
            EditorCommand::ToggleMode => {
                self.toggle_mode();
                effects.mode_changed = true;
                effects.selection_changed = true;
                effects.hints_changed = true;
            }
            EditorCommand::Undo => {
                if self.can_undo() {
                    self.undo_last_move();
                    effects.world_changed = true;
                    effects.selection_changed = true;
                    effects.history_changed = true;
                    effects.hints_changed = true;
                }
            }
            EditorCommand::RestartToGoals => {
                if self.can_restart() {
                    self.restart_to_goals();
                    effects.world_changed = true;
                    effects.selection_changed = true;
                    effects.history_changed = true;
                    effects.hints_changed = true;
                    effects.needs_revalidation = true;
                }
            }
            EditorCommand::ClearSelection => {
                if self.selected_box.take().is_some() {
                    self.clear_pull_destination_hints();
                    effects.selection_changed = true;
                    effects.hints_changed = true;
                }
            }
            EditorCommand::RecomputeHints => {
                self.mark_pull_destination_hints_dirty();
                self.refresh_pull_destination_hints_if_needed();
                effects.hints_changed = true;
            }
            EditorCommand::AdvanceHintJob { steps } => {
                let before = self.snapshot_hint_states();
                self.advance_pull_destination_hints_job(steps);
                effects.hints_changed = before != self.snapshot_hint_states();
            }
            EditorCommand::PaintCell {
                cell_x,
                cell_y,
                tool,
            } => {
                if self.paint_world_cell(cell_x, cell_y, tool) {
                    effects.world_changed = true;
                    effects.selection_changed = true;
                    effects.history_changed = true;
                    effects.hints_changed = true;
                    effects.needs_revalidation = true;
                }
            }
            EditorCommand::SelectBox { cell_x, cell_y } => {
                if self.select_box(cell_x, cell_y) {
                    effects.selection_changed = true;
                    effects.hints_changed = true;
                }
            }
            EditorCommand::MoveSelectedBoxTo { cell_x, cell_y } => {
                if self.move_selected_box_to(cell_x, cell_y) {
                    effects.world_changed = true;
                    effects.selection_changed = true;
                    effects.history_changed = true;
                    effects.hints_changed = true;
                    effects.needs_revalidation = true;
                }
            }
        }

        effects
    }

    pub(crate) fn refresh_pull_destination_hints_if_needed(&mut self) {
        if !self.pull_hints_dirty {
            return;
        }
        let Some(selected) = self.selected_box else {
            self.clear_pull_destination_hints();
            return;
        };
        if !matches!(self.mode, EditorMode::Manipulate) {
            self.clear_pull_destination_hints();
            return;
        }
        self.start_pull_destination_hints_job(selected);
        self.pull_hints_dirty = false;
    }

    fn snapshot_hint_states(&self) -> Vec<((i32, i32), PullHintState)> {
        let mut hints = self
            .pull_destination_hints
            .iter()
            .map(|(&position, &state)| (position, state))
            .collect::<Vec<_>>();
        hints.sort_unstable_by_key(|(position, _)| (position.1, position.0));
        hints
    }

    fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
        self.selected_box = None;
        self.clear_pull_destination_hints();
    }

    fn is_reset_state(&self) -> bool {
        self.solution_history.is_empty()
            && self.undo_history.is_empty()
            && self.world.player().is_none()
    }

    fn reset_solution_tracking(&mut self) {
        let goals = self.world.goal_positions();
        for (x, y) in self.world.box_positions() {
            let cleared = match self.world.tile(x, y) {
                EditableTile::BoxOnGoal => EditableTile::Goal,
                EditableTile::Box => EditableTile::Floor,
                other => other,
            };
            self.world.set_tile(x, y, cleared);
        }
        for (x, y) in goals {
            self.world.set_tile(x, y, EditableTile::BoxOnGoal);
        }
        self.world.set_player(None);
        self.solution_start_boxes = self.world.box_positions();
        self.solution_start_player = None;
        self.solution_history.clear();
        self.undo_history.clear();
        self.clear_pull_destination_hints();
    }

    fn make_undo_snapshot(&self) -> UndoSnapshot {
        UndoSnapshot {
            world: self.world.clone(),
            solution_start_boxes: self.solution_start_boxes.clone(),
            solution_start_player: self.solution_start_player,
            solution_history: self.solution_history.clone(),
        }
    }

    fn apply_undo_snapshot(&mut self, snapshot: UndoSnapshot) {
        self.world = snapshot.world;
        self.solution_start_boxes = snapshot.solution_start_boxes;
        self.solution_start_player = snapshot.solution_start_player;
        self.solution_history = snapshot.solution_history;
        self.selected_box = None;
        self.clear_pull_destination_hints();
    }

    fn undo_last_move(&mut self) {
        let Some(snapshot) = self.undo_history.pop() else {
            return;
        };
        self.apply_undo_snapshot(snapshot);
        if self.undo_history.is_empty() {
            self.world.set_player(None);
            self.solution_start_player = None;
        }
    }

    fn restart_to_goals(&mut self) {
        self.reset_solution_tracking();
        self.selected_box = None;
    }

    fn walkable_cells_for_optimizer(&self) -> Vec<(i32, i32)> {
        let Some(bounds) = self.world.non_void_bounds() else {
            return Vec::new();
        };

        let mut cells = Vec::new();
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                if !matches!(self.world.tile(x, y), EditableTile::Void) {
                    cells.push((x, y));
                }
            }
        }
        cells
    }

    fn record_box_move(&mut self, box_path: Vec<(i32, i32)>) {
        if box_path.len() < 2 {
            return;
        }
        self.solution_history.push(box_path);
        let input = ReverseOptimizationInput {
            walkable_cells: self.walkable_cells_for_optimizer(),
            box_positions: self.solution_start_boxes.clone(),
            player: self.solution_start_player,
        };
        optimize_reverse_solution_in_place(&input, &mut self.solution_history);
        self.pull_hints_dirty = true;
    }

    fn tracked_boxes_for_history(&self, history: &[Vec<(i32, i32)>]) -> Vec<TrackedBox> {
        let mut tracked = self
            .solution_start_boxes
            .iter()
            .copied()
            .enumerate()
            .map(|(start_index, position)| TrackedBox {
                start_index,
                position,
                count: 0,
            })
            .collect::<Vec<_>>();

        for movement in history {
            let Some(start_pos) = movement.first().copied() else {
                continue;
            };
            let Some(end_pos) = movement.last().copied() else {
                continue;
            };
            let Some(entry) = tracked.iter_mut().find(|entry| entry.position == start_pos) else {
                continue;
            };
            entry.position = end_pos;
            entry.count = entry.count.saturating_add(1);
        }

        tracked
    }

    fn box_move_counts_by_position_for_history(
        &self,
        history: &[Vec<(i32, i32)>],
    ) -> HashMap<(i32, i32), u32> {
        self.tracked_boxes_for_history(history)
            .into_iter()
            .map(|entry| (entry.position, entry.count))
            .collect::<HashMap<_, _>>()
    }

    fn box_move_counts_by_position(&self) -> HashMap<(i32, i32), u32> {
        self.box_move_counts_by_position_for_history(&self.solution_history)
    }

    fn box_start_index_for_position_in_history(
        &self,
        history: &[Vec<(i32, i32)>],
        target: (i32, i32),
    ) -> Option<usize> {
        self.tracked_boxes_for_history(history)
            .into_iter()
            .find_map(|entry| (entry.position == target).then_some(entry.start_index))
    }

    fn box_move_count_for_start_index_in_history(
        &self,
        history: &[Vec<(i32, i32)>],
        start_index: usize,
    ) -> Option<u32> {
        self.tracked_boxes_for_history(history)
            .into_iter()
            .find_map(|entry| (entry.start_index == start_index).then_some(entry.count))
    }

    fn clear_pull_destination_hints(&mut self) {
        self.pull_destination_hints.clear();
        self.pull_hints_dirty = false;
        self.pull_hint_generation = self.pull_hint_generation.wrapping_add(1);
        self.active_pull_hint_job = None;
    }

    fn mark_pull_destination_hints_dirty(&mut self) {
        if matches!(self.mode, EditorMode::Manipulate) && self.selected_box.is_some() {
            self.pull_destination_hints.clear();
            self.pull_hints_dirty = true;
            self.pull_hint_generation = self.pull_hint_generation.wrapping_add(1);
            self.active_pull_hint_job = None;
        } else {
            self.clear_pull_destination_hints();
        }
    }

    fn manhattan_distance(a: (i32, i32), b: (i32, i32)) -> i32 {
        (a.0 - b.0).abs() + (a.1 - b.1).abs()
    }

    fn has_chain_continuation_in_history(&self) -> bool {
        for i in 0..self.solution_history.len() {
            let Some(end_i) = self.solution_history[i].last().copied() else {
                continue;
            };
            for path in self.solution_history.iter().skip(i + 1) {
                if path.first().copied() == Some(end_i) {
                    return true;
                }
            }
        }
        false
    }

    fn quick_rewrite_feasibility_check(&self, selected: (i32, i32)) -> bool {
        if self.solution_history.len() < 2 {
            return false;
        }
        if self.has_chain_continuation_in_history() {
            return true;
        }
        self.solution_history
            .iter()
            .any(|path| path.last().copied() == Some(selected))
    }

    fn start_pull_destination_hints_job(&mut self, selected: (i32, i32)) {
        self.pull_destination_hints.clear();
        self.active_pull_hint_job = None;

        let legal_pull_destinations = self
            .enumerate_pull_move_plans(selected.0, selected.1)
            .into_iter()
            .map(|(destination, plan)| PullHintCandidate {
                destination,
                box_path: plan.box_path,
            })
            .collect::<Vec<_>>();
        if legal_pull_destinations.is_empty() {
            return;
        }

        let selected_start_index = self
            .box_start_index_for_position_in_history(&self.solution_history, selected)
            .expect("selected box must map to a tracked start position");
        let base_selected_count = self
            .box_move_count_for_start_index_in_history(&self.solution_history, selected_start_index)
            .unwrap_or(0)
            .saturating_add(1);
        if !self.quick_rewrite_feasibility_check(selected) {
            for candidate in legal_pull_destinations {
                self.pull_destination_hints.insert(
                    candidate.destination,
                    PullHintState::Ready(base_selected_count),
                );
            }
            return;
        }

        let mut candidates = legal_pull_destinations;
        candidates.sort_unstable_by_key(|candidate| {
            (
                Self::manhattan_distance(selected, candidate.destination),
                candidate.destination.1,
                candidate.destination.0,
            )
        });
        let mut pending_candidates = VecDeque::new();
        for candidate in candidates {
            self.pull_destination_hints
                .insert(candidate.destination, PullHintState::Pending);
            pending_candidates.push_back(candidate);
        }

        let optimization_input = ReverseOptimizationInput {
            walkable_cells: self.walkable_cells_for_optimizer(),
            box_positions: self.solution_start_boxes.clone(),
            player: self.solution_start_player,
        };
        if pending_candidates.is_empty() {
            return;
        }
        self.active_pull_hint_job = Some(ActivePullHintJob {
            generation: self.pull_hint_generation,
            selected,
            selected_start_index,
            optimization_input,
            pending_candidates,
        });
    }

    fn evaluate_pull_hint_candidate(
        &self,
        candidate: &PullHintCandidate,
        optimization_input: &ReverseOptimizationInput,
        selected_start_index: usize,
    ) -> u32 {
        let mut candidate_history = self.solution_history.clone();
        candidate_history.push(candidate.box_path.clone());
        optimize_reverse_solution_in_place(optimization_input, &mut candidate_history);

        let Some(count) = self
            .box_move_count_for_start_index_in_history(&candidate_history, selected_start_index)
        else {
            panic!(
                "hint count missing for selected_start_index={} destination=({}, {})",
                selected_start_index, candidate.destination.0, candidate.destination.1,
            );
        };
        count
    }

    fn advance_pull_destination_hints_job(&mut self, steps: usize) {
        if steps == 0 {
            return;
        }
        for _ in 0..steps {
            let (generation, selected, selected_start_index, optimization_input, candidate) = {
                let Some(job) = self.active_pull_hint_job.as_mut() else {
                    return;
                };
                let Some(candidate) = job.pending_candidates.pop_front() else {
                    self.active_pull_hint_job = None;
                    return;
                };
                (
                    job.generation,
                    job.selected,
                    job.selected_start_index,
                    job.optimization_input.clone(),
                    candidate,
                )
            };

            let count =
                self.evaluate_pull_hint_candidate(&candidate, &optimization_input, selected_start_index);

            if generation != self.pull_hint_generation || self.selected_box != Some(selected) {
                return;
            }

            let Some(job) = self.active_pull_hint_job.as_mut() else {
                return;
            };
            if job.generation != generation {
                return;
            }
            self.pull_destination_hints
                .insert(candidate.destination, PullHintState::Ready(count));

            if job.pending_candidates.is_empty() {
                self.active_pull_hint_job = None;
                return;
            }
        }
    }

    fn paint_world_cell(&mut self, world_x: i32, world_y: i32, tool: DrawTool) -> bool {
        let original_tile = self.world.tile(world_x, world_y);
        match tool {
            DrawTool::Floor => self.world.set_tile(world_x, world_y, EditableTile::Floor),
            DrawTool::BoxOnGoal => self
                .world
                .set_tile(world_x, world_y, EditableTile::BoxOnGoal),
            DrawTool::Void => self.world.set_tile(world_x, world_y, EditableTile::Void),
        }
        let updated_tile = self.world.tile(world_x, world_y);
        if self.selected_box == Some((world_x, world_y)) && !Self::is_box_tile(updated_tile) {
            self.selected_box = None;
        }
        if original_tile != updated_tile {
            self.reset_solution_tracking();
            self.mark_pull_destination_hints_dirty();
            return true;
        }
        false
    }

    fn is_box_tile(tile: EditableTile) -> bool {
        matches!(tile, EditableTile::Box | EditableTile::BoxOnGoal)
    }

    fn to_grid_position(world_x: i32, world_y: i32, min_x: i32, min_y: i32) -> Position {
        Position::new((world_y - min_y) as usize, (world_x - min_x) as usize)
    }

    fn to_world_position(grid: Position, min_x: i32, min_y: i32) -> (i32, i32) {
        (min_x + grid.col as i32, min_y + grid.row as i32)
    }

    fn build_pull_planning_context(
        &self,
        from_x: i32,
        from_y: i32,
        target: Option<(i32, i32)>,
    ) -> Option<PullPlanningContext> {
        let bounds = self.world.non_void_bounds()?;
        let mut min_x = bounds.min_x.min(from_x);
        let mut max_x = bounds.max_x.max(from_x);
        let mut min_y = bounds.min_y.min(from_y);
        let mut max_y = bounds.max_y.max(from_y);
        if let Some((to_x, to_y)) = target {
            min_x = min_x.min(to_x);
            max_x = max_x.max(to_x);
            min_y = min_y.min(to_y);
            max_y = max_y.max(to_y);
        }
        if let Some((px, py)) = self.world.player() {
            min_x = min_x.min(px);
            max_x = max_x.max(px);
            min_y = min_y.min(py);
            max_y = max_y.max(py);
        }

        let width = (max_x - min_x + 1) as usize;
        let height = (max_y - min_y + 1) as usize;
        let mut grid = vec![vec![false; width]; height];
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let walkable = match self.world.tile(x, y) {
                    EditableTile::Void => false,
                    EditableTile::Box | EditableTile::BoxOnGoal => (x, y) == (from_x, from_y),
                    EditableTile::Floor | EditableTile::Goal => true,
                };
                grid[(y - min_y) as usize][(x - min_x) as usize] = walkable;
            }
        }

        Some(PullPlanningContext { grid, min_x, min_y })
    }

    fn enumerate_pull_move_plans(
        &self,
        from_x: i32,
        from_y: i32,
    ) -> Vec<((i32, i32), PullMovePlan)> {
        let Some(context) = self.build_pull_planning_context(from_x, from_y, None) else {
            return Vec::new();
        };
        let PullPlanningContext { grid, min_x, min_y } = context;
        let box_start = Self::to_grid_position(from_x, from_y, min_x, min_y);
        let player_start = self
            .world
            .player()
            .map(|(x, y)| Self::to_grid_position(x, y, min_x, min_y));
        let mut pathfinder = PullPathfinder::new(grid, box_start, player_start);

        pathfinder
            .find_all_pull_paths()
            .into_iter()
            .map(|(origin, result)| {
                let destination = Self::to_world_position(origin, min_x, min_y);
                let box_path = result
                    .box_path
                    .into_iter()
                    .map(|pos| Self::to_world_position(pos, min_x, min_y))
                    .collect::<Vec<_>>();
                (
                    destination,
                    PullMovePlan {
                        player_start: Self::to_world_position(result.player_start, min_x, min_y),
                        box_path,
                    },
                )
            })
            .collect()
    }

    fn find_pull_move_plan(
        &self,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
    ) -> Option<PullMovePlan> {
        let context = self.build_pull_planning_context(from_x, from_y, Some((to_x, to_y)))?;
        let PullPlanningContext { grid, min_x, min_y } = context;
        let box_start = Self::to_grid_position(from_x, from_y, min_x, min_y);
        let origin = Self::to_grid_position(to_x, to_y, min_x, min_y);
        let player_start = self
            .world
            .player()
            .map(|(x, y)| Self::to_grid_position(x, y, min_x, min_y));

        let mut pathfinder = PullPathfinder::new(grid, box_start, player_start);
        let result = pathfinder.find_pull_path(origin)?;
        let box_path = result
            .box_path
            .into_iter()
            .map(|pos| Self::to_world_position(pos, min_x, min_y))
            .collect::<Vec<_>>();
        Some(PullMovePlan {
            player_start: Self::to_world_position(result.player_start, min_x, min_y),
            box_path,
        })
    }

    fn select_box(&mut self, world_x: i32, world_y: i32) -> bool {
        let tapped_tile = self.world.tile(world_x, world_y);
        if !Self::is_box_tile(tapped_tile) {
            return false;
        }

        let previous = self.selected_box;
        self.selected_box = if previous == Some((world_x, world_y)) {
            None
        } else {
            Some((world_x, world_y))
        };
        self.mark_pull_destination_hints_dirty();
        previous != self.selected_box
    }

    fn move_selected_box_to(&mut self, world_x: i32, world_y: i32) -> bool {
        let tapped_tile = self.world.tile(world_x, world_y);
        if matches!(tapped_tile, EditableTile::Void) {
            if self.selected_box.take().is_some() {
                self.clear_pull_destination_hints();
            }
            return false;
        }

        let Some((from_x, from_y)) = self.selected_box else {
            return false;
        };
        if from_x == world_x && from_y == world_y {
            return false;
        }

        let from_tile = self.world.tile(from_x, from_y);
        if !Self::is_box_tile(from_tile) {
            self.selected_box = None;
            self.clear_pull_destination_hints();
            return false;
        }
        let Some(plan) = self.find_pull_move_plan(from_x, from_y, world_x, world_y) else {
            return false;
        };

        let undo_snapshot = self.make_undo_snapshot();
        let from_base = match from_tile {
            EditableTile::BoxOnGoal => EditableTile::Goal,
            EditableTile::Box => EditableTile::Floor,
            _ => from_tile,
        };
        self.world.set_tile(from_x, from_y, from_base);

        let to_tile = self.world.tile(world_x, world_y);
        let to_with_box = match to_tile {
            EditableTile::Goal => EditableTile::BoxOnGoal,
            EditableTile::BoxOnGoal => EditableTile::BoxOnGoal,
            _ => EditableTile::Box,
        };
        self.world.set_tile(world_x, world_y, to_with_box);
        self.world.set_player(Some(plan.player_start));
        self.record_box_move(plan.box_path);
        self.undo_history.push(undo_snapshot);
        self.selected_box = None;
        self.clear_pull_destination_hints();
        true
    }

    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            EditorMode::Draw => EditorMode::Manipulate,
            EditorMode::Manipulate => EditorMode::Draw,
        };
        self.selected_box = None;
        self.clear_pull_destination_hints();
    }
}

impl Default for LevelEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{LevelEditor, PullHintState};
    use crate::command::{DrawTool, EditorCommand, EditorMode};
    use crate::snapshot::PullHintStatus;
    use crate::world::EditableTile;

    fn clear_world(editor: &mut LevelEditor) {
        let Some(bounds) = editor.world.non_void_bounds() else {
            return;
        };
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                editor.world.set_tile(x, y, EditableTile::Void);
            }
        }
        editor.world.set_player(None);
    }

    #[test]
    fn paint_command_updates_world() {
        let mut editor = LevelEditor::new();
        let effects = editor.apply_command(EditorCommand::PaintCell {
            cell_x: 0,
            cell_y: 0,
            tool: DrawTool::Void,
        });

        assert_eq!(editor.world().tile(0, 0), EditableTile::Void);
        assert!(effects.world_changed);
        assert!(effects.needs_revalidation);
    }

    #[test]
    fn snapshot_exposes_mode_and_selection() {
        let mut editor = LevelEditor::new();
        editor.apply_command(EditorCommand::SetMode(EditorMode::Manipulate));
        editor.world.set_tile(1, 0, EditableTile::Box);
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 1,
            cell_y: 0,
        });

        let snapshot = editor.snapshot();
        assert_eq!(snapshot.mode, EditorMode::Manipulate);
        assert_eq!(snapshot.selected_box, Some((1, 0)));
    }

    #[test]
    fn consecutive_moves_of_the_same_box_are_consolidated() {
        let mut editor = LevelEditor::new();
        editor.solution_start_boxes = vec![(0, 0)];
        editor.solution_history.clear();

        editor.record_box_move(vec![(0, 0), (1, 0)]);
        editor.record_box_move(vec![(1, 0), (2, 0)]);
        editor.record_box_move(vec![(2, 0), (3, 0)]);

        assert_eq!(editor.solution_history.len(), 1);
        assert_eq!(
            editor.solution_history[0],
            vec![(0, 0), (1, 0), (2, 0), (3, 0)]
        );

        let counts = editor.box_move_counts_by_position();
        assert_eq!(counts.get(&(3, 0)).copied(), Some(1));
    }

    #[test]
    fn non_consecutive_moves_are_not_consolidated() {
        let mut editor = LevelEditor::new();
        editor.solution_start_boxes = vec![(0, 0), (2, 0)];
        editor.solution_history.clear();

        editor.record_box_move(vec![(0, 0), (1, 0)]);
        editor.record_box_move(vec![(2, 0), (3, 0)]);
        editor.record_box_move(vec![(1, 0), (0, 0)]);

        assert_eq!(editor.solution_history.len(), 3);

        let counts = editor.box_move_counts_by_position();
        assert_eq!(counts.get(&(0, 0)).copied(), Some(2));
        assert_eq!(counts.get(&(3, 0)).copied(), Some(1));
    }

    #[test]
    fn undo_restores_previous_solution_snapshot() {
        let mut editor = LevelEditor::new();
        let snapshot = editor.make_undo_snapshot();

        editor.world.set_tile(0, 0, EditableTile::Void);
        editor.world.set_player(Some((1, 1)));
        editor.solution_history.push(vec![(1, 1), (1, 2)]);
        editor.undo_history.push(snapshot);
        editor.undo_last_move();

        assert!(editor.solution_history.is_empty());
        assert_ne!(editor.world.tile(0, 0), EditableTile::Void);
        assert_eq!(editor.world.player(), None);
    }

    #[test]
    fn restart_resets_boxes_on_goals_and_clears_undo_history() {
        let mut editor = LevelEditor::new();
        editor.world.set_tile(-2, -1, EditableTile::Goal);
        editor.world.set_tile(0, 0, EditableTile::Box);
        editor.solution_history.push(vec![(-2, -1), (0, 0)]);
        editor.undo_history.push(editor.make_undo_snapshot());

        editor.restart_to_goals();

        assert!(editor.solution_history.is_empty());
        assert!(editor.undo_history.is_empty());
        assert_eq!(editor.world.player(), None);
        for (x, y) in editor.world.box_positions() {
            assert_eq!(editor.world.tile(x, y), EditableTile::BoxOnGoal);
        }
    }

    #[test]
    fn tracked_box_identity_preserves_selected_box_count() {
        let mut editor = LevelEditor::new();
        editor.solution_start_boxes = vec![(0, 0), (2, 0)];
        let history = vec![
            vec![(0, 0), (1, 0)],
            vec![(2, 0), (0, 0)],
            vec![(1, 0), (2, 0)],
        ];

        assert_eq!(
            editor.box_start_index_for_position_in_history(&history, (2, 0)),
            Some(0)
        );
        assert_eq!(
            editor.box_start_index_for_position_in_history(&history, (0, 0)),
            Some(1)
        );
        assert_eq!(
            editor.box_move_count_for_start_index_in_history(&history, 0),
            Some(2)
        );
        assert_eq!(
            editor.box_move_count_for_start_index_in_history(&history, 1),
            Some(1)
        );
    }

    #[test]
    fn trivial_hint_path_marks_all_destinations_ready_without_job() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=4 {
            for y in 0..=1 {
                editor.world.set_tile(x, y, EditableTile::Floor);
            }
        }
        editor.world.set_tile(2, 0, EditableTile::Box);
        editor.world.set_player(None);
        editor.solution_start_boxes = vec![(2, 0)];
        editor.solution_start_player = None;
        editor.solution_history.clear();
        editor.selected_box = Some((2, 0));
        editor.mode = EditorMode::Manipulate;

        editor.start_pull_destination_hints_job((2, 0));

        assert!(!editor.pull_destination_hints.is_empty());
        assert!(editor.active_pull_hint_job.is_none());
        for hint in editor.pull_destination_hints.values() {
            assert_eq!(*hint, PullHintState::Ready(1));
        }
    }

    #[test]
    fn nontrivial_hint_path_starts_incremental_pending_job() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=4 {
            for y in 0..=1 {
                editor.world.set_tile(x, y, EditableTile::Floor);
            }
        }
        editor.world.set_tile(2, 0, EditableTile::Box);
        editor.world.set_player(None);
        editor.solution_start_boxes = vec![(2, 0)];
        editor.solution_start_player = None;
        editor.solution_history = vec![vec![(2, 0), (3, 0)], vec![(3, 0), (2, 0)]];
        editor.selected_box = Some((2, 0));
        editor.mode = EditorMode::Manipulate;

        editor.start_pull_destination_hints_job((2, 0));

        assert!(!editor.pull_destination_hints.is_empty());
        assert!(editor.active_pull_hint_job.is_some());
        assert!(
            editor
                .pull_destination_hints
                .values()
                .any(|hint| matches!(hint, PullHintState::Pending))
        );

        let pending_before = editor
            .pull_destination_hints
            .values()
            .filter(|hint| matches!(hint, PullHintState::Pending))
            .count();
        editor.advance_pull_destination_hints_job(1);
        let pending_after = editor
            .pull_destination_hints
            .values()
            .filter(|hint| matches!(hint, PullHintState::Pending))
            .count();
        assert!(pending_after < pending_before);
    }

    #[test]
    fn stale_hint_results_are_ignored_when_selection_changes() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=4 {
            for y in 0..=1 {
                editor.world.set_tile(x, y, EditableTile::Floor);
            }
        }
        editor.world.set_tile(2, 0, EditableTile::Box);
        editor.world.set_player(None);
        editor.solution_start_boxes = vec![(2, 0)];
        editor.solution_start_player = None;
        editor.solution_history = vec![vec![(2, 0), (3, 0)], vec![(3, 0), (2, 0)]];
        editor.selected_box = Some((2, 0));
        editor.mode = EditorMode::Manipulate;

        editor.start_pull_destination_hints_job((2, 0));
        assert!(editor.active_pull_hint_job.is_some());
        assert!(
            editor
                .pull_destination_hints
                .values()
                .all(|hint| matches!(hint, PullHintState::Pending))
        );

        editor.selected_box = None;
        editor.advance_pull_destination_hints_job(1);

        assert!(
            editor
                .pull_destination_hints
                .values()
                .all(|hint| matches!(hint, PullHintState::Pending))
        );
    }

    #[test]
    fn hint_job_completes_and_snapshot_reports_ready_hints() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=4 {
            for y in 0..=1 {
                editor.world.set_tile(x, y, EditableTile::Floor);
            }
        }
        editor.world.set_tile(2, 0, EditableTile::Box);
        editor.world.set_player(None);
        editor.solution_start_boxes = vec![(2, 0)];
        editor.solution_start_player = None;
        editor.solution_history = vec![vec![(2, 0), (3, 0)], vec![(3, 0), (2, 0)]];
        editor.selected_box = Some((2, 0));
        editor.mode = EditorMode::Manipulate;

        editor.start_pull_destination_hints_job((2, 0));
        assert!(editor.active_pull_hint_job.is_some());

        let mut steps = 0usize;
        while editor.active_pull_hint_job.is_some() && steps < 64 {
            editor.advance_pull_destination_hints_job(1);
            steps += 1;
        }

        let snapshot = editor.snapshot();
        assert!(editor.active_pull_hint_job.is_none());
        assert!(steps > 0);
        assert!(
            snapshot
                .pull_destination_hints
                .iter()
                .all(|hint| matches!(hint.state, PullHintStatus::Ready(_)))
        );
    }
}
