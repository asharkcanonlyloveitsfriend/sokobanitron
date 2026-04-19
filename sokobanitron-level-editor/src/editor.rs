use crate::command::{DrawTool, EditorCommand, EditorEffects, EditorMode};
use crate::snapshot::{
    BoxMoveCountSnapshot, EditorBoardSnapshot, EditorCellSnapshot, EditorSnapshot,
    PullHintSnapshot, PullHintStatus, PullHintTotalMoveChange,
};
use crate::world::{EditableWorld, NonVoidBounds, Tile};
use sokobanitron_core::optimizer::{ReverseOptimizationInput, optimize_reverse_solution_in_place};
use sokobanitron_core::pathfinder::{PullPathfinder, WorldBounds, WorldGrid};
use std::cmp::Ordering;
use std::collections::{HashMap, VecDeque};

struct PullMovePlan {
    player_start: (i32, i32),
    box_path: Vec<(i32, i32)>,
}

struct PullPlanningContext {
    grid: WorldGrid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PullHintState {
    Pending,
    Ready(PullHintTotalMoveChange),
}

struct PullHintCandidate {
    destination: (i32, i32),
    box_path: Vec<(i32, i32)>,
}

#[derive(Clone, Copy)]
struct TrackedBox {
    position: (i32, i32),
    count: u32,
}

struct ActivePullHintJob {
    generation: u64,
    selected: (i32, i32),
    optimization_input: Option<ReverseOptimizationInput>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedPuzzle {
    pub level_ascii: String,
    pub reference_solution: Vec<Vec<(usize, usize)>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPuzzleError {
    EmptyBoard,
    MissingPlayer,
    MissingReferenceSolution,
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

    pub fn last_move_destination(&self) -> Option<(i32, i32)> {
        let snapshot = self.undo_history.last()?;
        self.world
            .box_positions()
            .into_iter()
            .find(|position| !snapshot.world.has_box(position.0, position.1))
    }

    pub fn export_puzzle(&self) -> Result<ExportedPuzzle, ExportPuzzleError> {
        let Some(bounds) = self.world.non_void_bounds() else {
            return Err(ExportPuzzleError::EmptyBoard);
        };
        if self.world.player().is_none() {
            return Err(ExportPuzzleError::MissingPlayer);
        }
        if self.solution_history.is_empty() {
            return Err(ExportPuzzleError::MissingReferenceSolution);
        }

        Ok(ExportedPuzzle {
            level_ascii: self.level_ascii_in_bounds(bounds),
            reference_solution: self.forward_reference_solution_in_bounds(bounds),
        })
    }

    pub fn selected_box(&self) -> Option<(i32, i32)> {
        self.selected_box
    }

    pub fn has_active_pull_hint_job(&self) -> bool {
        self.active_pull_hint_job.is_some()
    }

    pub fn box_has_pull_move(&self, world_x: i32, world_y: i32) -> bool {
        self.world.has_box(world_x, world_y)
            && !self.enumerate_pull_move_plans(world_x, world_y).is_empty()
    }

    pub fn snapshot(&self) -> EditorSnapshot {
        let bounds = self.world.non_void_bounds();
        let mut cells = Vec::new();
        if let Some(bounds) = bounds {
            for y in bounds.min_y..=bounds.max_y {
                for x in bounds.min_x..=bounds.max_x {
                    let tile = self.world.tile(x, y);
                    if matches!(tile, Tile::Void) {
                        assert!(
                            !self.world.has_box(x, y),
                            "void tile cannot contain a box in the editor snapshot"
                        );
                    } else {
                        cells.push(EditorCellSnapshot {
                            world_x: x,
                            world_y: y,
                            tile,
                            has_box: self.world.has_box(x, y),
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
                    PullHintState::Ready(change) => PullHintStatus::Ready(*change),
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

        self.refresh_pull_destination_hints_if_needed();
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
        if !matches!(self.mode, EditorMode::Move) {
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

    fn level_ascii_in_bounds(&self, bounds: NonVoidBounds) -> String {
        let mut lines = Vec::new();
        for y in bounds.min_y..=bounds.max_y {
            let mut line = String::with_capacity((bounds.max_x - bounds.min_x + 1) as usize);
            for x in bounds.min_x..=bounds.max_x {
                let is_player = self.world.player() == Some((x, y));
                let tile = self.world.tile(x, y);
                let has_box = self.world.has_box(x, y);
                let ch = if is_player {
                    match tile {
                        Tile::Goal => '+',
                        Tile::Floor => '@',
                        Tile::Void => panic!("player cannot stand on void during export"),
                    }
                } else if has_box {
                    match tile {
                        Tile::Goal => '*',
                        Tile::Floor => '$',
                        Tile::Void => panic!("void tile cannot contain a box during export"),
                    }
                } else {
                    match tile {
                        Tile::Void => '#',
                        Tile::Floor => ' ',
                        Tile::Goal => '.',
                    }
                };
                line.push(ch);
            }
            lines.push(line);
        }
        lines.join("\n")
    }

    fn forward_reference_solution_in_bounds(
        &self,
        bounds: NonVoidBounds,
    ) -> Vec<Vec<(usize, usize)>> {
        self.solution_history
            .iter()
            .rev()
            .map(|path| {
                path.iter()
                    .rev()
                    .map(|&(x, y)| ((y - bounds.min_y) as usize, (x - bounds.min_x) as usize))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn rebuild_start_state_from_terrain(&mut self) {
        let goals = self.world.goal_positions();
        for (x, y) in self.world.box_positions() {
            self.world.set_box(x, y, false);
        }
        for (x, y) in goals {
            self.world.set_box(x, y, true);
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
        self.rebuild_start_state_from_terrain();
        self.selected_box = None;
    }

    fn walkable_cells_for_optimizer(&self) -> Vec<(i32, i32)> {
        let Some(bounds) = self.world.non_void_bounds() else {
            return Vec::new();
        };

        let mut cells = Vec::new();
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                if !matches!(self.world.tile(x, y), Tile::Void) {
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
            .map(|position| TrackedBox { position, count: 0 })
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

    fn clear_pull_destination_hints(&mut self) {
        self.pull_destination_hints.clear();
        self.pull_hints_dirty = false;
        self.pull_hint_generation = self.pull_hint_generation.wrapping_add(1);
        self.active_pull_hint_job = None;
    }

    fn mark_pull_destination_hints_dirty(&mut self) {
        if matches!(self.mode, EditorMode::Move) && self.selected_box.is_some() {
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

        let mut candidates = legal_pull_destinations;
        candidates.sort_unstable_by_key(|candidate| {
            (
                Self::manhattan_distance(selected, candidate.destination),
                candidate.destination.1,
                candidate.destination.0,
            )
        });
        let optimization_input =
            self.quick_rewrite_feasibility_check(selected)
                .then(|| ReverseOptimizationInput {
                    walkable_cells: self.walkable_cells_for_optimizer(),
                    box_positions: self.solution_start_boxes.clone(),
                    player: self.solution_start_player,
                });

        if optimization_input.is_none() {
            for candidate in candidates {
                let change = self.evaluate_pull_hint_candidate(&candidate, None);
                self.pull_destination_hints
                    .insert(candidate.destination, PullHintState::Ready(change));
            }
            return;
        }

        let mut pending_candidates = VecDeque::new();
        for candidate in candidates {
            self.pull_destination_hints
                .insert(candidate.destination, PullHintState::Pending);
            pending_candidates.push_back(candidate);
        }

        if pending_candidates.is_empty() {
            return;
        }
        self.active_pull_hint_job = Some(ActivePullHintJob {
            generation: self.pull_hint_generation,
            selected,
            optimization_input,
            pending_candidates,
        });
    }

    fn evaluate_pull_hint_candidate(
        &self,
        candidate: &PullHintCandidate,
        optimization_input: Option<&ReverseOptimizationInput>,
    ) -> PullHintTotalMoveChange {
        let mut candidate_history = self.solution_history.clone();
        candidate_history.push(candidate.box_path.clone());
        if let Some(optimization_input) = optimization_input {
            optimize_reverse_solution_in_place(optimization_input, &mut candidate_history);
        }

        match candidate_history.len().cmp(&self.solution_history.len()) {
            Ordering::Less => PullHintTotalMoveChange::Decrease,
            Ordering::Equal => PullHintTotalMoveChange::Equal,
            Ordering::Greater => PullHintTotalMoveChange::Increase,
        }
    }

    fn advance_pull_destination_hints_job(&mut self, steps: usize) {
        if steps == 0 {
            return;
        }
        for _ in 0..steps {
            let (generation, selected, optimization_input, candidate) = {
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
                    job.optimization_input.clone(),
                    candidate,
                )
            };

            let change = self.evaluate_pull_hint_candidate(&candidate, optimization_input.as_ref());

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
                .insert(candidate.destination, PullHintState::Ready(change));

            if job.pending_candidates.is_empty() {
                self.active_pull_hint_job = None;
                return;
            }
        }
    }

    fn paint_world_cell(&mut self, world_x: i32, world_y: i32, tool: DrawTool) -> bool {
        let original_tile = self.world.tile(world_x, world_y);
        match tool {
            DrawTool::Floor => self.world.set_tile(world_x, world_y, Tile::Floor),
            DrawTool::GoalWithBox => self.world.set_tile(world_x, world_y, Tile::Goal),
            DrawTool::Void => self.world.set_tile(world_x, world_y, Tile::Void),
        }
        if original_tile != self.world.tile(world_x, world_y) {
            self.rebuild_start_state_from_terrain();
            self.mark_pull_destination_hints_dirty();
            return true;
        }
        false
    }

    fn build_pull_planning_context(
        &self,
        from_x: i32,
        from_y: i32,
        target: Option<(i32, i32)>,
    ) -> Option<PullPlanningContext> {
        let bounds = self.world.non_void_bounds()?;
        let mut bounds = WorldBounds {
            min_x: bounds.min_x,
            max_x: bounds.max_x,
            min_y: bounds.min_y,
            max_y: bounds.max_y,
        };
        bounds.include((from_x, from_y));
        if let Some((to_x, to_y)) = target {
            bounds.include((to_x, to_y));
        }
        if let Some(player) = self.world.player() {
            bounds.include(player);
        }

        let grid = WorldGrid::from_bounds(bounds, |(x, y)| match self.world.tile(x, y) {
            Tile::Void => false,
            Tile::Floor | Tile::Goal => !self.world.has_box(x, y) || (x, y) == (from_x, from_y),
        });

        Some(PullPlanningContext { grid })
    }

    fn enumerate_pull_move_plans(
        &self,
        from_x: i32,
        from_y: i32,
    ) -> Vec<((i32, i32), PullMovePlan)> {
        let Some(context) = self.build_pull_planning_context(from_x, from_y, None) else {
            return Vec::new();
        };
        let PullPlanningContext { grid } = context;
        let world_origin = grid.origin();
        let Some(box_start) = grid.to_grid_position((from_x, from_y)) else {
            return Vec::new();
        };
        let player_start = self
            .world
            .player()
            .and_then(|position| grid.to_grid_position(position));
        let mut pathfinder = PullPathfinder::new(grid.into_rows(), box_start, player_start);

        pathfinder
            .find_all_pull_paths()
            .into_iter()
            .map(|(destination_grid, result)| {
                let destination = world_origin.to_world_position(destination_grid);
                let box_path = result
                    .box_path
                    .into_iter()
                    .map(|pos| world_origin.to_world_position(pos))
                    .collect::<Vec<_>>();
                (
                    destination,
                    PullMovePlan {
                        player_start: world_origin.to_world_position(result.player_start),
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
        let PullPlanningContext { grid } = context;
        let world_origin = grid.origin();
        let box_start = grid.to_grid_position((from_x, from_y))?;
        let origin = grid.to_grid_position((to_x, to_y))?;
        let player_start = self
            .world
            .player()
            .and_then(|position| grid.to_grid_position(position));

        let mut pathfinder = PullPathfinder::new(grid.into_rows(), box_start, player_start);
        let result = pathfinder.find_pull_path(origin)?;
        let box_path = result
            .box_path
            .into_iter()
            .map(|pos| world_origin.to_world_position(pos))
            .collect::<Vec<_>>();
        Some(PullMovePlan {
            player_start: world_origin.to_world_position(result.player_start),
            box_path,
        })
    }

    fn select_box(&mut self, world_x: i32, world_y: i32) -> bool {
        if !self.world.has_box(world_x, world_y) {
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
        if matches!(self.world.tile(world_x, world_y), Tile::Void) {
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

        if !self.world.has_box(from_x, from_y) {
            self.selected_box = None;
            self.clear_pull_destination_hints();
            return false;
        }
        let Some(plan) = self.find_pull_move_plan(from_x, from_y, world_x, world_y) else {
            return false;
        };

        let undo_snapshot = self.make_undo_snapshot();
        self.world.set_box(from_x, from_y, false);
        self.world.set_box(world_x, world_y, true);
        self.world.set_player(Some(plan.player_start));
        self.record_box_move(plan.box_path);
        self.undo_history.push(undo_snapshot);
        self.selected_box = None;
        self.clear_pull_destination_hints();
        true
    }

    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            EditorMode::Draw => EditorMode::Move,
            EditorMode::Move => EditorMode::Draw,
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
    use super::{ExportPuzzleError, LevelEditor, PullHintState};
    use crate::command::{DrawTool, EditorCommand, EditorMode};
    use crate::snapshot::{PullHintStatus, PullHintTotalMoveChange};
    use crate::world::Tile;

    fn clear_world(editor: &mut LevelEditor) {
        let Some(bounds) = editor.world.non_void_bounds() else {
            return;
        };
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                editor.world.set_tile(x, y, Tile::Void);
            }
        }
        editor.world.set_player(None);
    }

    #[test]
    fn export_puzzle_requires_player_start() {
        let editor = LevelEditor::new();

        assert_eq!(
            editor.export_puzzle(),
            Err(ExportPuzzleError::MissingPlayer)
        );
    }

    #[test]
    fn export_puzzle_requires_reference_solution() {
        let mut editor = LevelEditor::new();
        editor.world.set_player(Some((0, 0)));

        assert_eq!(
            editor.export_puzzle(),
            Err(ExportPuzzleError::MissingReferenceSolution)
        );
    }

    #[test]
    fn export_puzzle_returns_forward_reference_solution() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for y in 0..3 {
            for x in 0..5 {
                editor.world.set_tile(x, y, Tile::Floor);
            }
        }
        editor.world.set_tile(3, 1, Tile::Goal);
        editor.world.set_box(1, 1, true);
        editor.world.set_player(Some((0, 1)));
        editor.solution_history = vec![vec![(3, 1), (2, 1), (1, 1)]];

        let exported = editor.export_puzzle().expect("export puzzle");

        assert_eq!(exported.level_ascii, "     \n@$ . \n     ");
        assert_eq!(
            exported.reference_solution,
            vec![vec![(1, 1), (1, 2), (1, 3)]]
        );
    }

    #[test]
    fn export_puzzle_reverses_move_order_for_multiple_paths() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for y in 0..3 {
            for x in 0..6 {
                editor.world.set_tile(x, y, Tile::Floor);
            }
        }
        editor.world.set_tile(4, 1, Tile::Goal);
        editor.world.set_tile(3, 1, Tile::Goal);
        editor.world.set_box(1, 1, true);
        editor.world.set_box(2, 1, true);
        editor.world.set_player(Some((0, 1)));
        editor.solution_history = vec![vec![(4, 1), (3, 1), (2, 1)], vec![(3, 1), (2, 1), (1, 1)]];

        let exported = editor.export_puzzle().expect("export puzzle");

        assert_eq!(
            exported.reference_solution,
            vec![vec![(1, 1), (1, 2), (1, 3)], vec![(1, 2), (1, 3), (1, 4)],]
        );
    }

    #[test]
    fn paint_command_updates_world() {
        let mut editor = LevelEditor::new();
        let effects = editor.apply_command(EditorCommand::PaintCell {
            cell_x: 0,
            cell_y: 0,
            tool: DrawTool::Void,
        });

        assert_eq!(editor.world().tile(0, 0), Tile::Void);
        assert!(effects.world_changed);
        assert!(effects.needs_revalidation);
    }

    #[test]
    fn snapshot_exposes_mode_and_selection() {
        let mut editor = LevelEditor::new();
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.world.set_box(1, 0, true);
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 1,
            cell_y: 0,
        });

        let snapshot = editor.snapshot();
        assert_eq!(snapshot.mode, EditorMode::Move);
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

        editor.world.set_tile(0, 0, Tile::Void);
        editor.world.set_player(Some((1, 1)));
        editor.solution_history.push(vec![(1, 1), (1, 2)]);
        editor.undo_history.push(snapshot);
        editor.undo_last_move();

        assert!(editor.solution_history.is_empty());
        assert_ne!(editor.world.tile(0, 0), Tile::Void);
        assert_eq!(editor.world.player(), None);
    }

    #[test]
    fn restart_resets_boxes_on_goals_and_clears_undo_history() {
        let mut editor = LevelEditor::new();
        editor.world.set_tile(-2, -1, Tile::Goal);
        editor.world.set_box(0, 0, true);
        editor.solution_history.push(vec![(-2, -1), (0, 0)]);
        editor.undo_history.push(editor.make_undo_snapshot());

        editor.restart_to_goals();

        assert!(editor.solution_history.is_empty());
        assert!(editor.undo_history.is_empty());
        assert_eq!(editor.world.player(), None);
        for (x, y) in editor.world.box_positions() {
            assert_eq!(editor.world.tile(x, y), Tile::Goal);
        }
    }

    #[test]
    fn trivial_hint_path_resolves_immediately_without_pending_job() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=4 {
            for y in 0..=1 {
                editor.world.set_tile(x, y, Tile::Floor);
            }
        }
        editor.world.set_box(2, 0, true);
        editor.world.set_player(None);
        editor.solution_start_boxes = vec![(2, 0)];
        editor.solution_start_player = None;
        editor.solution_history.clear();
        editor.selected_box = Some((2, 0));
        editor.mode = EditorMode::Move;

        editor.start_pull_destination_hints_job((2, 0));

        assert!(!editor.pull_destination_hints.is_empty());
        assert!(editor.active_pull_hint_job.is_none());
        for hint in editor.pull_destination_hints.values() {
            assert_eq!(
                *hint,
                PullHintState::Ready(PullHintTotalMoveChange::Increase)
            );
        }
    }

    #[test]
    fn destination_hints_cover_only_legal_destinations() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=4 {
            for y in 0..=2 {
                editor.world.set_tile(x, y, Tile::Floor);
            }
        }
        editor.world.set_box(2, 1, true);
        editor.world.set_box(0, 0, true);
        editor.world.set_player(None);
        editor.solution_start_boxes = vec![(0, 0), (2, 1)];
        editor.solution_start_player = None;
        editor.solution_history.clear();
        editor.selected_box = Some((2, 1));
        editor.mode = EditorMode::Move;

        editor.start_pull_destination_hints_job((2, 1));

        assert!(!editor.pull_destination_hints.contains_key(&(2, 1)));
        assert!(!editor.pull_destination_hints.contains_key(&(0, 0)));
        for destination in editor.pull_destination_hints.keys().copied() {
            assert!(
                editor
                    .find_pull_move_plan(2, 1, destination.0, destination.1)
                    .is_some(),
                "hint at ({}, {}) must be a legal selected-box destination",
                destination.0,
                destination.1
            );
        }
    }

    #[test]
    fn nontrivial_hint_path_starts_incremental_pending_job() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=4 {
            for y in 0..=1 {
                editor.world.set_tile(x, y, Tile::Floor);
            }
        }
        editor.world.set_box(2, 0, true);
        editor.world.set_player(None);
        editor.solution_start_boxes = vec![(2, 0)];
        editor.solution_start_player = None;
        editor.solution_history = vec![vec![(2, 0), (3, 0)], vec![(3, 0), (2, 0)]];
        editor.selected_box = Some((2, 0));
        editor.mode = EditorMode::Move;

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
                editor.world.set_tile(x, y, Tile::Floor);
            }
        }
        editor.world.set_box(2, 0, true);
        editor.world.set_player(None);
        editor.solution_start_boxes = vec![(2, 0)];
        editor.solution_start_player = None;
        editor.solution_history = vec![vec![(2, 0), (3, 0)], vec![(3, 0), (2, 0)]];
        editor.selected_box = Some((2, 0));
        editor.mode = EditorMode::Move;

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
                editor.world.set_tile(x, y, Tile::Floor);
            }
        }
        editor.world.set_box(2, 0, true);
        editor.world.set_player(None);
        editor.solution_start_boxes = vec![(2, 0)];
        editor.solution_start_player = None;
        editor.solution_history = vec![vec![(2, 0), (3, 0)], vec![(3, 0), (2, 0)]];
        editor.selected_box = Some((2, 0));
        editor.mode = EditorMode::Move;

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

    #[test]
    fn box_has_pull_move_is_false_for_blocked_box() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        editor.world.set_tile(1, 1, Tile::Floor);
        editor.world.set_box(1, 1, true);
        editor.world.set_player(None);

        assert!(!editor.box_has_pull_move(1, 1));
    }
}
