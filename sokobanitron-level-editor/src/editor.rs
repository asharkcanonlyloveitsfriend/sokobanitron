use crate::command::{DrawTool, EditorCommand, EditorEffects, EditorMode};
use crate::snapshot::{EditorBoardSnapshot, EditorCellSnapshot, EditorSnapshot};
use crate::world::{EditableWorld, NonVoidBounds, Tile};
use sokobanitron_gameplay::{BoardCell, BoardView, GameplayController, GameplayTapEvent, TileKind};

struct EditorPlaySession {
    origin_x: i32,
    origin_y: i32,
    initial_boxes: Vec<(usize, usize)>,
    controller: GameplayController,
}

enum EditorState {
    Draw,
    Move,
    Play(Box<EditorPlaySession>),
}

impl EditorState {
    fn mode(&self) -> EditorMode {
        match self {
            Self::Draw => EditorMode::Draw,
            Self::Move => EditorMode::Move,
            Self::Play(_) => EditorMode::Play,
        }
    }
}

pub struct EditorPlayBoard<'a> {
    pub origin_x: i32,
    pub origin_y: i32,
    pub board: &'a BoardView,
}

pub struct LevelEditor {
    state: EditorState,
    selected_box: Option<(i32, i32)>,
    world: EditableWorld,
    validated_solution: Option<SolutionHistory>,
}

type SolutionHistory = Vec<Vec<(usize, usize)>>;
type PlayClickResult = (PlayCellOutcome, Option<SolutionHistory>);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MoveSelectedBoxOutcome {
    Moved,
    Rejected,
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlayCellOutcome {
    changed: bool,
    solved: bool,
}

impl LevelEditor {
    pub fn new() -> Self {
        Self {
            state: EditorState::Draw,
            selected_box: None,
            world: EditableWorld::new(),
            validated_solution: None,
        }
    }

    pub fn world(&self) -> &EditableWorld {
        &self.world
    }

    pub fn mode(&self) -> EditorMode {
        self.state.mode()
    }

    pub fn can_enter_play(&self) -> bool {
        self.world.player().is_some() && self.has_box_off_goal()
    }

    pub fn can_save(&self) -> bool {
        self.validated_solution.is_some()
    }

    pub fn export_puzzle(&self) -> Result<ExportedPuzzle, ExportPuzzleError> {
        let Some(bounds) = self.world.non_void_bounds() else {
            return Err(ExportPuzzleError::EmptyBoard);
        };
        if self.world.player().is_none() {
            return Err(ExportPuzzleError::MissingPlayer);
        }
        let Some(reference_solution) = &self.validated_solution else {
            return Err(ExportPuzzleError::MissingReferenceSolution);
        };

        Ok(ExportedPuzzle {
            level_ascii: self.level_ascii_in_bounds(bounds),
            reference_solution: reference_solution.clone(),
        })
    }

    pub fn selected_box(&self) -> Option<(i32, i32)> {
        self.selected_box
    }

    pub fn play_board(&self) -> Option<EditorPlayBoard<'_>> {
        match &self.state {
            EditorState::Play(play) => Some(EditorPlayBoard {
                origin_x: play.origin_x,
                origin_y: play.origin_y,
                board: play.controller.board(),
            }),
            EditorState::Draw | EditorState::Move => None,
        }
    }

    pub fn view_tile(&self, world_x: i32, world_y: i32) -> Tile {
        if let Some((play, cell)) = self.play_cell_for_world(world_x, world_y) {
            return match play.controller.board().tile(cell) {
                TileKind::Void => Tile::Void,
                TileKind::Floor => Tile::Floor,
                TileKind::Goal => Tile::Goal,
            };
        }
        self.world.tile(world_x, world_y)
    }

    pub fn view_has_box(&self, world_x: i32, world_y: i32) -> bool {
        if let Some((play, cell)) = self.play_cell_for_world(world_x, world_y) {
            return play.controller.board().has_box(cell);
        }
        self.world.has_box(world_x, world_y)
    }

    pub fn view_player(&self) -> Option<(i32, i32)> {
        if let EditorState::Play(play) = &self.state {
            return play
                .controller
                .board()
                .player()
                .map(|cell| (play.origin_x + cell.x as i32, play.origin_y + cell.y as i32));
        }
        self.world.player()
    }

    pub fn view_selected_box(&self) -> Option<(i32, i32)> {
        if let EditorState::Play(play) = &self.state {
            return play
                .controller
                .board()
                .selected_box()
                .map(|cell| (play.origin_x + cell.x as i32, play.origin_y + cell.y as i32));
        }
        self.selected_box
    }

    pub fn view_is_solved(&self) -> bool {
        matches!(&self.state, EditorState::Play(play) if play.controller.board().is_solved())
    }

    pub fn box_move_counts(&self) -> Vec<BoxMoveCount> {
        let Some(solution) = &self.validated_solution else {
            return Vec::new();
        };

        if let EditorState::Play(play) = &self.state {
            if !play.controller.board().is_solved() {
                return Vec::new();
            }
            return solution_box_move_counts(&play.initial_boxes, solution)
                .into_iter()
                .map(|count| BoxMoveCount {
                    world_x: play.origin_x + count.current_cell_x as i32,
                    world_y: play.origin_y + count.current_cell_y as i32,
                    count: count.count,
                })
                .collect();
        }

        let Some(bounds) = self.world.non_void_bounds() else {
            return Vec::new();
        };
        let initial_boxes = self.initial_box_positions_in_bounds(bounds);
        solution_box_move_counts(&initial_boxes, solution)
            .into_iter()
            .map(|count| BoxMoveCount {
                world_x: bounds.min_x + count.initial_cell_x as i32,
                world_y: bounds.min_y + count.initial_cell_y as i32,
                count: count.count,
            })
            .collect()
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

        EditorSnapshot {
            board: EditorBoardSnapshot {
                bounds,
                cells,
                player: self.world.player(),
            },
            mode: self.mode(),
            selected_box: self.selected_box,
            can_enter_play: self.can_enter_play(),
            can_save: self.can_save(),
        }
    }

    pub fn apply_command(&mut self, command: EditorCommand) -> EditorEffects {
        let mut effects = EditorEffects::default();
        match command {
            EditorCommand::SetMode(mode) => {
                if self.mode() != mode && self.set_mode(mode) {
                    effects.mode_changed = true;
                    effects.selection_changed = true;
                    effects.world_changed = true;
                }
            }
            EditorCommand::ToggleMode => {
                if self.toggle_mode() {
                    effects.mode_changed = true;
                    effects.selection_changed = true;
                    effects.world_changed = true;
                }
            }
            EditorCommand::ClearSelection => {
                if matches!(&self.state, EditorState::Move) && self.clear_selection() {
                    effects.selection_changed = true;
                }
            }
            EditorCommand::PaintCell {
                cell_x,
                cell_y,
                tool,
            } => {
                if matches!(&self.state, EditorState::Draw)
                    && self.paint_world_cell(cell_x, cell_y, tool)
                {
                    effects.world_changed = true;
                    effects.selection_changed = true;
                    effects.needs_revalidation = true;
                }
            }
            EditorCommand::PositionPlayer { cell_x, cell_y } => {
                if matches!(&self.state, EditorState::Move) && self.position_player(cell_x, cell_y)
                {
                    effects.world_changed = true;
                    effects.selection_changed = true;
                    effects.needs_revalidation = true;
                }
            }
            EditorCommand::SelectBox { cell_x, cell_y } => {
                if matches!(&self.state, EditorState::Move) && self.select_box(cell_x, cell_y) {
                    effects.selection_changed = true;
                }
            }
            EditorCommand::MoveSelectedBoxTo { cell_x, cell_y } => {
                if matches!(&self.state, EditorState::Move) {
                    let previous_selection = self.selected_box;
                    match self.move_selected_box_to(cell_x, cell_y) {
                        MoveSelectedBoxOutcome::Moved => {
                            effects.world_changed = true;
                            effects.selection_changed = true;
                            effects.needs_revalidation = true;
                        }
                        MoveSelectedBoxOutcome::Rejected => {
                            effects.move_rejected = true;
                        }
                        MoveSelectedBoxOutcome::Noop => {}
                    }
                    if self.selected_box != previous_selection {
                        effects.selection_changed = true;
                    }
                }
            }
            EditorCommand::PlayCell { cell_x, cell_y } => {
                let outcome = self.play_cell(cell_x, cell_y);
                if outcome.changed {
                    effects.world_changed = true;
                }
                if outcome.solved {
                    effects.play_solved = true;
                    effects.needs_revalidation = true;
                }
            }
            EditorCommand::PlayDoubleTap { cell_x, cell_y } => {
                let outcome = self.play_double_tap(cell_x, cell_y);
                if outcome.changed {
                    effects.world_changed = true;
                }
                if outcome.solved {
                    effects.play_solved = true;
                    effects.needs_revalidation = true;
                }
            }
        }

        effects
    }

    fn set_mode(&mut self, mode: EditorMode) -> bool {
        match mode {
            EditorMode::Draw => {
                self.state = EditorState::Draw;
                self.selected_box = None;
                true
            }
            EditorMode::Move => {
                self.state = EditorState::Move;
                self.selected_box = None;
                true
            }
            EditorMode::Play => {
                if !self.can_enter_play() {
                    return false;
                }
                self.selected_box = None;
                let Some(play) = self.build_play_session() else {
                    return false;
                };
                self.state = EditorState::Play(Box::new(play));
                true
            }
        }
    }

    fn toggle_mode(&mut self) -> bool {
        match self.mode() {
            EditorMode::Draw => self.set_mode(EditorMode::Move),
            EditorMode::Move if self.can_enter_play() => self.set_mode(EditorMode::Play),
            EditorMode::Move => self.set_mode(EditorMode::Draw),
            EditorMode::Play => self.set_mode(EditorMode::Draw),
        }
    }

    fn build_play_session(&self) -> Option<EditorPlaySession> {
        let bounds = self.world.non_void_bounds()?;
        self.world.player()?;
        let level_ascii = self.level_ascii_in_bounds(bounds);
        let initial_boxes = self.initial_box_positions_in_bounds(bounds);
        Some(EditorPlaySession {
            origin_x: bounds.min_x,
            origin_y: bounds.min_y,
            initial_boxes,
            controller: GameplayController::new_strict(vec![level_ascii], None),
        })
    }

    fn initial_box_positions_in_bounds(&self, bounds: NonVoidBounds) -> Vec<(usize, usize)> {
        self.world
            .box_positions()
            .into_iter()
            .map(|(x, y)| ((y - bounds.min_y) as usize, (x - bounds.min_x) as usize))
            .collect()
    }

    fn invalidate_solution(&mut self) {
        debug_assert!(
            !matches!(&self.state, EditorState::Play(_)),
            "play mode should not mutate the editable world or invalidate validation"
        );
        self.validated_solution = None;
    }

    fn has_box_off_goal(&self) -> bool {
        self.world
            .box_positions()
            .into_iter()
            .any(|(x, y)| matches!(self.world.tile(x, y), Tile::Floor))
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

    fn clear_selection(&mut self) -> bool {
        let in_move_mode = matches!(&self.state, EditorState::Move);
        debug_assert!(
            in_move_mode,
            "ClearSelection should only mutate selection in move mode"
        );
        if !in_move_mode {
            return false;
        }

        self.selected_box.take().is_some()
    }

    fn paint_world_cell(&mut self, world_x: i32, world_y: i32, tool: DrawTool) -> bool {
        let in_draw_mode = matches!(&self.state, EditorState::Draw);
        debug_assert!(
            in_draw_mode,
            "PaintCell should only mutate the editable world in draw mode"
        );
        if !in_draw_mode {
            return false;
        }

        let before_tile = self.world.tile(world_x, world_y);
        let before_boxes = self.world.box_positions();
        let before_player = self.world.player();

        match tool {
            DrawTool::Floor => self.paint_floor(world_x, world_y),
            DrawTool::GoalWithBox => self.paint_goal_with_box(world_x, world_y),
            DrawTool::Void => self.paint_void(world_x, world_y),
        }

        self.drop_invalid_selection();
        let changed = before_tile != self.world.tile(world_x, world_y)
            || before_boxes != self.world.box_positions()
            || before_player != self.world.player();
        if changed {
            self.invalidate_solution();
        }
        changed
    }

    fn paint_floor(&mut self, world_x: i32, world_y: i32) {
        if matches!(self.world.tile(world_x, world_y), Tile::Goal) {
            self.remove_goal_box((world_x, world_y));
        }
        self.world.set_tile(world_x, world_y, Tile::Floor);
    }

    fn paint_goal_with_box(&mut self, world_x: i32, world_y: i32) {
        let was_goal = matches!(self.world.tile(world_x, world_y), Tile::Goal);
        if self.world.has_box(world_x, world_y) {
            self.world.set_box(world_x, world_y, false);
            if !was_goal {
                self.restore_box_to_first_available_goal();
            }
        } else if was_goal {
            self.remove_first_box();
        }

        self.world.set_tile(world_x, world_y, Tile::Goal);
        if self.world.player() == Some((world_x, world_y)) {
            self.world.set_player(None);
        }
        self.world.set_box(world_x, world_y, true);
    }

    fn paint_void(&mut self, world_x: i32, world_y: i32) {
        let position = (world_x, world_y);
        match self.world.tile(world_x, world_y) {
            Tile::Goal => {
                self.remove_goal_box(position);
                self.world.set_tile(world_x, world_y, Tile::Void);
            }
            Tile::Floor => {
                let had_box = self.world.has_box(world_x, world_y);
                if had_box {
                    self.world.set_box(world_x, world_y, false);
                }
                self.world.set_tile(world_x, world_y, Tile::Void);
                if had_box {
                    self.restore_box_to_first_available_goal();
                }
            }
            Tile::Void => {}
        }
    }

    fn remove_goal_box(&mut self, position: (i32, i32)) {
        if self.world.has_box(position.0, position.1) {
            self.world.set_box(position.0, position.1, false);
        } else {
            self.remove_first_box();
        }
    }

    fn remove_first_box(&mut self) -> bool {
        if let Some((x, y)) = self.world.box_positions().into_iter().next() {
            self.world.set_box(x, y, false);
            true
        } else {
            false
        }
    }

    fn restore_box_to_first_available_goal(&mut self) -> bool {
        let Some(goal) = self
            .world
            .goal_positions()
            .into_iter()
            .find(|&(x, y)| !self.world.has_box(x, y))
        else {
            return false;
        };

        if self.world.player() == Some(goal) {
            self.world.set_player(None);
        }
        self.world.set_box(goal.0, goal.1, true);
        true
    }

    fn drop_invalid_selection(&mut self) {
        if let Some((x, y)) = self.selected_box
            && !self.world.has_box(x, y)
        {
            self.selected_box = None;
        }
    }

    fn position_player(&mut self, world_x: i32, world_y: i32) -> bool {
        let in_move_mode = matches!(&self.state, EditorState::Move);
        debug_assert!(
            in_move_mode,
            "PositionPlayer should only mutate the editable world in move mode"
        );
        if !in_move_mode {
            return false;
        }

        if matches!(self.world.tile(world_x, world_y), Tile::Void)
            || self.world.has_box(world_x, world_y)
            || self.world.player() == Some((world_x, world_y))
        {
            return false;
        }

        self.selected_box = None;
        self.world.set_player(Some((world_x, world_y)));
        self.invalidate_solution();
        true
    }

    fn select_box(&mut self, world_x: i32, world_y: i32) -> bool {
        let in_move_mode = matches!(&self.state, EditorState::Move);
        debug_assert!(
            in_move_mode,
            "SelectBox should only mutate selection in move mode"
        );
        if !in_move_mode {
            return false;
        }

        if !self.world.has_box(world_x, world_y) {
            return false;
        }

        let previous = self.selected_box;
        self.selected_box = Some((world_x, world_y));
        previous != self.selected_box
    }

    fn move_selected_box_to(&mut self, world_x: i32, world_y: i32) -> MoveSelectedBoxOutcome {
        let in_move_mode = matches!(&self.state, EditorState::Move);
        debug_assert!(
            in_move_mode,
            "MoveSelectedBoxTo should only mutate the editable world in move mode"
        );
        if !in_move_mode {
            return MoveSelectedBoxOutcome::Noop;
        }

        let Some((from_x, from_y)) = self.selected_box else {
            return MoveSelectedBoxOutcome::Noop;
        };
        if from_x == world_x && from_y == world_y {
            return MoveSelectedBoxOutcome::Noop;
        }
        if !self.world.has_box(from_x, from_y) {
            self.selected_box = None;
            return MoveSelectedBoxOutcome::Noop;
        }
        if matches!(self.world.tile(world_x, world_y), Tile::Void)
            || self.world.has_box(world_x, world_y)
        {
            self.selected_box = None;
            return MoveSelectedBoxOutcome::Rejected;
        }

        self.world.set_box(from_x, from_y, false);
        if self.world.player() == Some((world_x, world_y)) {
            self.world.set_player(None);
        }
        self.world.set_box(world_x, world_y, true);
        self.selected_box = None;
        self.invalidate_solution();
        MoveSelectedBoxOutcome::Moved
    }

    fn play_cell(&mut self, world_x: i32, world_y: i32) -> PlayCellOutcome {
        let EditorState::Play(play) = &mut self.state else {
            return PlayCellOutcome {
                changed: false,
                solved: false,
            };
        };
        let Some(cell) = Self::play_cell_in_session(play, world_x, world_y) else {
            return PlayCellOutcome {
                changed: false,
                solved: false,
            };
        };

        let (outcome, solution) = Self::click_play_cell(play, cell);
        if let Some(solution) = solution {
            self.validated_solution = Some(solution);
        }
        outcome
    }

    fn play_double_tap(&mut self, world_x: i32, world_y: i32) -> PlayCellOutcome {
        let EditorState::Play(play) = &mut self.state else {
            return PlayCellOutcome {
                changed: false,
                solved: false,
            };
        };
        let Some(cell) = Self::play_cell_in_session(play, world_x, world_y) else {
            return PlayCellOutcome {
                changed: false,
                solved: false,
            };
        };

        if play.controller.can_restart() && play.controller.board().player() == Some(cell) {
            let before_board = play.controller.board().clone();
            play.controller.restart_with_changes();
            return PlayCellOutcome {
                changed: before_board != *play.controller.board(),
                solved: false,
            };
        }

        if play.controller.board().is_solved() {
            return PlayCellOutcome {
                changed: false,
                solved: false,
            };
        }

        if play.controller.can_undo() && play.controller.last_box_move_destination() == Some(cell) {
            let before_board = play.controller.board().clone();
            play.controller.undo_with_changes();
            return PlayCellOutcome {
                changed: before_board != *play.controller.board(),
                solved: false,
            };
        }

        let (outcome, solution) = Self::click_play_cell(play, cell);
        if let Some(solution) = solution {
            self.validated_solution = Some(solution);
        }
        outcome
    }

    fn play_cell_for_world(
        &self,
        world_x: i32,
        world_y: i32,
    ) -> Option<(&EditorPlaySession, BoardCell)> {
        let EditorState::Play(play) = &self.state else {
            return None;
        };
        Self::play_cell_in_session(play, world_x, world_y).map(|cell| (play.as_ref(), cell))
    }

    fn play_cell_in_session(
        play: &EditorPlaySession,
        world_x: i32,
        world_y: i32,
    ) -> Option<BoardCell> {
        let local_x = u32::try_from(world_x - play.origin_x).ok()?;
        let local_y = u32::try_from(world_y - play.origin_y).ok()?;
        if local_x >= play.controller.board().width() || local_y >= play.controller.board().height()
        {
            return None;
        }
        Some(BoardCell::new(local_x, local_y))
    }

    fn click_play_cell(play: &mut EditorPlaySession, cell: BoardCell) -> PlayClickResult {
        let before_board = play.controller.board().clone();
        let outcome = play.controller.click_cell_with_outcome(cell);
        let solved = matches!(outcome.event, GameplayTapEvent::PuzzleSolved { .. });
        let solution = solved.then(|| play.controller.solution_history());

        (
            PlayCellOutcome {
                changed: before_board != *play.controller.board(),
                solved,
            },
            solution,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxMoveCount {
    pub world_x: i32,
    pub world_y: i32,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalBoxMoveCount {
    initial_cell_x: usize,
    initial_cell_y: usize,
    current_cell_x: usize,
    current_cell_y: usize,
    count: u32,
}

fn solution_box_move_counts(
    initial_boxes: &[(usize, usize)],
    solution: &[Vec<(usize, usize)>],
) -> Vec<LocalBoxMoveCount> {
    let mut tracked = initial_boxes
        .iter()
        .copied()
        .map(|(row, col)| LocalBoxMoveCount {
            initial_cell_x: col,
            initial_cell_y: row,
            current_cell_x: col,
            current_cell_y: row,
            count: 0,
        })
        .collect::<Vec<_>>();

    for path in solution {
        let (Some(&start), Some(&end)) = (path.first(), path.last()) else {
            continue;
        };
        let Some(entry) = tracked
            .iter_mut()
            .find(|entry| (entry.current_cell_y, entry.current_cell_x) == start)
        else {
            continue;
        };
        entry.current_cell_x = end.1;
        entry.current_cell_y = end.0;
        entry.count = entry.count.saturating_add(1);
    }

    tracked.sort_unstable_by_key(|count| (count.current_cell_y, count.current_cell_x));
    tracked
}

impl Default for LevelEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportPuzzleError, LevelEditor, solution_box_move_counts};
    use crate::command::{DrawTool, EditorCommand, EditorEffects, EditorMode};
    use crate::world::{NonVoidBounds, Tile};

    #[derive(Debug, PartialEq, Eq)]
    struct EditableWorldState {
        bounds: Option<NonVoidBounds>,
        tiles: Vec<(i32, i32, Tile)>,
        boxes: Vec<(i32, i32)>,
        player: Option<(i32, i32)>,
    }

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

    fn paint_floor(editor: &mut LevelEditor, x: i32, y: i32) {
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: x,
            cell_y: y,
            tool: DrawTool::Floor,
        });
    }

    fn paint_goal_with_box(editor: &mut LevelEditor, x: i32, y: i32) {
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: x,
            cell_y: y,
            tool: DrawTool::GoalWithBox,
        });
    }

    fn assert_no_effects(effects: EditorEffects) {
        assert_eq!(effects, EditorEffects::default());
    }

    fn editable_world_state(editor: &LevelEditor) -> EditableWorldState {
        let bounds = editor.world.non_void_bounds();
        let mut tiles = Vec::new();
        if let Some(bounds) = bounds {
            for y in bounds.min_y..=bounds.max_y {
                for x in bounds.min_x..=bounds.max_x {
                    tiles.push((x, y, editor.world.tile(x, y)));
                }
            }
        }
        EditableWorldState {
            bounds,
            tiles,
            boxes: editor.world.box_positions(),
            player: editor.world.player(),
        }
    }

    fn setup_simple_playable_editor() -> LevelEditor {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=3 {
            paint_floor(&mut editor, x, 0);
        }
        paint_goal_with_box(&mut editor, 2, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 2,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 0,
            cell_y: 0,
        });
        editor
    }

    fn setup_unsolved_play_editor() -> LevelEditor {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=4 {
            paint_floor(&mut editor, x, 0);
        }
        paint_goal_with_box(&mut editor, 3, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 3,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::ToggleMode);
        editor
    }

    fn setup_play_editor_with_void_push() -> LevelEditor {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        paint_floor(&mut editor, 0, 0);
        paint_floor(&mut editor, 1, 0);
        paint_goal_with_box(&mut editor, 3, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 3,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::ToggleMode);
        editor
    }

    fn setup_validated_editor() -> LevelEditor {
        let mut editor = setup_simple_playable_editor();
        editor.apply_command(EditorCommand::ToggleMode);
        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 1,
            cell_y: 0,
        });
        let solved = editor.apply_command(EditorCommand::PlayCell {
            cell_x: 2,
            cell_y: 0,
        });

        assert!(solved.play_solved);
        assert!(editor.can_save());
        editor
    }

    #[test]
    fn draw_toggles_to_move() {
        let mut editor = LevelEditor::new();

        let effects = editor.apply_command(EditorCommand::ToggleMode);

        assert!(effects.mode_changed);
        assert_eq!(editor.mode(), EditorMode::Move);
        assert!(editor.play_board().is_none());
    }

    #[test]
    fn move_toggles_to_play_when_play_can_start() {
        let mut editor = setup_simple_playable_editor();

        let effects = editor.apply_command(EditorCommand::ToggleMode);

        assert!(editor.can_enter_play());
        assert!(effects.mode_changed);
        assert_eq!(editor.mode(), EditorMode::Play);
        assert!(editor.play_board().is_some());
    }

    #[test]
    fn set_mode_can_enter_play_directly_from_draw_when_play_can_start() {
        let mut editor = setup_simple_playable_editor();
        editor.apply_command(EditorCommand::SetMode(EditorMode::Draw));

        let effects = editor.apply_command(EditorCommand::SetMode(EditorMode::Play));

        assert!(editor.can_enter_play());
        assert!(effects.mode_changed);
        assert_eq!(editor.mode(), EditorMode::Play);
        assert!(editor.play_board().is_some());
    }

    #[test]
    fn move_toggles_to_draw_when_play_cannot_start() {
        let mut editor = LevelEditor::new();
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));

        let effects = editor.apply_command(EditorCommand::ToggleMode);

        assert!(!editor.can_enter_play());
        assert!(effects.mode_changed);
        assert_eq!(editor.mode(), EditorMode::Draw);
        assert!(editor.play_board().is_none());
    }

    #[test]
    fn play_toggles_back_to_draw() {
        let mut editor = setup_simple_playable_editor();
        editor.apply_command(EditorCommand::ToggleMode);

        let effects = editor.apply_command(EditorCommand::ToggleMode);

        assert!(effects.mode_changed);
        assert_eq!(editor.mode(), EditorMode::Draw);
        assert!(editor.play_board().is_none());
    }

    #[test]
    fn play_mode_views_read_from_gameplay_session() {
        let mut editor = setup_simple_playable_editor();
        editor.apply_command(EditorCommand::ToggleMode);
        editor.world.set_tile(2, 0, Tile::Floor);
        editor.world.set_player(Some((3, 0)));
        editor.world.set_box(3, 0, true);

        assert_eq!(editor.view_tile(2, 0), Tile::Goal);
        assert_eq!(editor.view_player(), Some((0, 0)));
        assert!(!editor.view_has_box(3, 0));

        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 1,
            cell_y: 0,
        });

        assert_eq!(editor.view_selected_box(), Some((1, 0)));
        assert_eq!(editor.selected_box(), None);

        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 2,
            cell_y: 0,
        });

        assert!(editor.world.has_box(1, 0));
        assert!(!editor.view_has_box(1, 0));
        assert!(editor.view_has_box(2, 0));
        assert_eq!(editor.view_player(), Some((1, 0)));
    }

    #[test]
    fn play_mode_rejects_pushing_box_into_void() {
        let mut editor = setup_play_editor_with_void_push();

        let select = editor.apply_command(EditorCommand::PlayCell {
            cell_x: 1,
            cell_y: 0,
        });
        let reject = editor.apply_command(EditorCommand::PlayCell {
            cell_x: 2,
            cell_y: 0,
        });

        assert!(select.world_changed);
        assert!(reject.world_changed);
        assert!(!reject.play_solved);
        assert_eq!(editor.view_selected_box(), None);
        assert_eq!(editor.view_player(), Some((0, 0)));
        assert!(editor.view_has_box(1, 0));
        assert!(!editor.view_has_box(2, 0));
    }

    #[test]
    fn paint_cell_in_play_does_not_mutate_editable_world_or_leave_play() {
        let mut editor = setup_simple_playable_editor();
        editor.apply_command(EditorCommand::ToggleMode);
        let before = editable_world_state(&editor);

        let effects = editor.apply_command(EditorCommand::PaintCell {
            cell_x: 3,
            cell_y: 0,
            tool: DrawTool::Void,
        });

        assert_no_effects(effects);
        assert_eq!(editor.mode(), EditorMode::Play);
        assert_eq!(editable_world_state(&editor), before);
    }

    #[test]
    fn move_commands_in_play_do_not_mutate_editable_world_or_leave_play() {
        let mut editor = setup_simple_playable_editor();
        editor.apply_command(EditorCommand::ToggleMode);
        editor.selected_box = Some((1, 0));
        let before = editable_world_state(&editor);

        let move_effects = editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 3,
            cell_y: 0,
        });
        let player_effects = editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 3,
            cell_y: 0,
        });

        assert_no_effects(move_effects);
        assert_no_effects(player_effects);
        assert_eq!(editor.mode(), EditorMode::Play);
        assert_eq!(editable_world_state(&editor), before);
    }

    #[test]
    fn play_cell_outside_play_is_noop() {
        let mut editor = setup_simple_playable_editor();
        let before = editor.snapshot();

        let effects = editor.apply_command(EditorCommand::PlayCell {
            cell_x: 1,
            cell_y: 0,
        });

        assert_no_effects(effects);
        assert_eq!(editor.mode(), EditorMode::Move);
        assert_eq!(editor.snapshot(), before);
        assert!(!editor.can_save());
    }

    #[test]
    fn play_double_tap_outside_play_is_noop() {
        let mut editor = setup_simple_playable_editor();
        let before = editor.snapshot();

        let effects = editor.apply_command(EditorCommand::PlayDoubleTap {
            cell_x: 1,
            cell_y: 0,
        });

        assert_no_effects(effects);
        assert_eq!(editor.mode(), EditorMode::Move);
        assert_eq!(editor.snapshot(), before);
        assert!(!editor.can_save());
    }

    #[test]
    fn only_solving_in_play_sets_validated_solution() {
        let mut editor = setup_simple_playable_editor();

        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 1,
            cell_y: 0,
        });
        assert!(!editor.can_save());

        editor.apply_command(EditorCommand::ToggleMode);
        assert!(!editor.can_save());

        let selected = editor.apply_command(EditorCommand::PlayCell {
            cell_x: 1,
            cell_y: 0,
        });
        assert!(!selected.play_solved);
        assert!(!editor.can_save());

        let solved = editor.apply_command(EditorCommand::PlayCell {
            cell_x: 2,
            cell_y: 0,
        });
        assert!(solved.play_solved);
        assert!(editor.can_save());
    }

    #[test]
    fn draw_mutation_invalidates_validated_solution() {
        let mut editor = setup_validated_editor();
        editor.apply_command(EditorCommand::SetMode(EditorMode::Draw));

        let effects = editor.apply_command(EditorCommand::PaintCell {
            cell_x: 3,
            cell_y: 0,
            tool: DrawTool::Void,
        });

        assert!(effects.needs_revalidation);
        assert!(!editor.can_save());
    }

    #[test]
    fn move_mutation_invalidates_validated_solution() {
        let mut editor = setup_validated_editor();
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 1,
            cell_y: 0,
        });

        let effects = editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 3,
            cell_y: 0,
        });

        assert!(effects.needs_revalidation);
        assert!(!editor.can_save());
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
    fn export_puzzle_requires_validated_solution() {
        let editor = setup_simple_playable_editor();

        assert_eq!(
            editor.export_puzzle(),
            Err(ExportPuzzleError::MissingReferenceSolution)
        );
    }

    #[test]
    fn moving_box_rejects_occupied_destination() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=5 {
            paint_floor(&mut editor, x, 0);
        }
        paint_goal_with_box(&mut editor, 4, 0);
        paint_goal_with_box(&mut editor, 2, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 4,
            cell_y: 0,
        });

        let moved = editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });

        assert!(moved.world_changed);
        assert!(editor.world.has_box(1, 0));
        assert!(editor.world.has_box(2, 0));

        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 1,
            cell_y: 0,
        });
        let rejected = editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 2,
            cell_y: 0,
        });

        assert!(rejected.move_rejected);
        assert!(editor.world.has_box(1, 0));
        assert!(editor.world.has_box(2, 0));
    }

    #[test]
    fn moving_box_does_not_require_pullable_destination() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        paint_goal_with_box(&mut editor, 0, 0);
        paint_floor(&mut editor, 2, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 0,
            cell_y: 0,
        });

        let moved = editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 2,
            cell_y: 0,
        });

        assert!(moved.world_changed);
        assert!(editor.world.has_box(2, 0));
        assert!(!editor.world.has_box(0, 0));
    }

    #[test]
    fn invalid_move_rejects_and_deselects_box() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        paint_floor(&mut editor, 0, 0);
        paint_goal_with_box(&mut editor, 1, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 1,
            cell_y: 0,
        });

        let effects = editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 4,
            cell_y: 4,
        });

        assert!(effects.move_rejected);
        assert_eq!(editor.selected_box(), None);
        assert!(editor.world.has_box(1, 0));
    }

    #[test]
    fn selecting_and_moving_box_leaves_player_in_place() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=3 {
            paint_floor(&mut editor, x, 0);
        }
        paint_goal_with_box(&mut editor, 2, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 0,
            cell_y: 0,
        });

        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 2,
            cell_y: 0,
        });
        assert_eq!(editor.world.player(), Some((0, 0)));

        let effects = editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });

        assert!(effects.world_changed);
        assert_eq!(editor.world.player(), Some((0, 0)));
        assert!(editor.world.has_box(1, 0));
        assert!(!editor.world.has_box(2, 0));
    }

    #[test]
    fn moving_box_onto_player_removes_player() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=3 {
            paint_floor(&mut editor, x, 0);
        }
        paint_goal_with_box(&mut editor, 2, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 1,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 2,
            cell_y: 0,
        });

        let effects = editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });

        assert!(effects.world_changed);
        assert_eq!(editor.world.player(), None);
        assert!(editor.world.has_box(1, 0));
        assert!(!editor.world.has_box(2, 0));
    }

    #[test]
    fn empty_move_mode_tap_positions_player_without_reachability() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        paint_floor(&mut editor, 0, 0);
        paint_floor(&mut editor, 2, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));

        let effects = editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 2,
            cell_y: 0,
        });

        assert!(effects.world_changed);
        assert_eq!(editor.world.player(), Some((2, 0)));
    }

    #[test]
    fn play_mode_solving_level_enables_export_with_forward_solution() {
        let mut editor = setup_simple_playable_editor();
        let effects = editor.apply_command(EditorCommand::ToggleMode);

        assert!(effects.mode_changed);
        assert_eq!(editor.mode(), EditorMode::Play);

        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 1,
            cell_y: 0,
        });
        let solved = editor.apply_command(EditorCommand::PlayCell {
            cell_x: 2,
            cell_y: 0,
        });

        assert!(solved.play_solved);
        let exported = editor.export_puzzle().expect("validated export");
        assert_eq!(exported.reference_solution, vec![vec![(0, 1), (0, 2)]]);
        assert_eq!(exported.level_ascii, "@$. ");
    }

    #[test]
    fn solution_box_move_counts_chains_moves_by_start_and_previous_end() {
        let counts = solution_box_move_counts(
            &[(0, 0), (0, 4)],
            &[
                vec![(0, 0), (0, 1)],
                vec![(0, 4), (0, 5)],
                vec![(0, 1), (0, 2)],
            ],
        );

        assert_eq!(counts.len(), 2);
        assert!(
            counts
                .iter()
                .any(|count| count.current_cell_x == 2 && count.count == 2)
        );
        assert!(
            counts
                .iter()
                .any(|count| count.current_cell_x == 5 && count.count == 1)
        );
    }

    #[test]
    fn solution_box_move_counts_keeps_unmoved_boxes() {
        let counts = solution_box_move_counts(&[(0, 0), (0, 4)], &[vec![(0, 0), (0, 1)]]);

        assert_eq!(counts.len(), 2);
        assert!(
            counts
                .iter()
                .any(|count| count.current_cell_x == 1 && count.count == 1)
        );
        assert!(
            counts
                .iter()
                .any(|count| count.current_cell_x == 4 && count.count == 0)
        );
    }

    #[test]
    fn solution_box_move_counts_treats_solution_tuples_as_row_col() {
        let counts = solution_box_move_counts(&[(3, 0)], &[vec![(3, 0), (3, 1)]]);

        assert_eq!(counts.len(), 1);
        assert_eq!(counts[0].current_cell_x, 1);
        assert_eq!(counts[0].current_cell_y, 3);
        assert_eq!(counts[0].count, 1);
    }

    #[test]
    fn play_mode_restarts_from_editor_setup_each_time() {
        let mut editor = setup_simple_playable_editor();
        editor.apply_command(EditorCommand::ToggleMode);
        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 1,
            cell_y: 0,
        });

        let selected_during_first_play = editor.view_selected_box();
        editor.apply_command(EditorCommand::ToggleMode);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::ToggleMode);

        assert_eq!(selected_during_first_play, Some((1, 0)));
        assert_eq!(editor.mode(), EditorMode::Play);
        assert_eq!(editor.view_selected_box(), None);
        assert!(editor.view_has_box(1, 0));
        assert!(!editor.view_has_box(2, 0));
    }

    #[test]
    fn play_double_tap_on_last_move_destination_undoes_play_move() {
        let mut editor = setup_unsolved_play_editor();
        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 1,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 2,
            cell_y: 0,
        });

        let effects = editor.apply_command(EditorCommand::PlayDoubleTap {
            cell_x: 2,
            cell_y: 0,
        });

        assert!(effects.world_changed);
        assert!(editor.view_has_box(1, 0));
        assert!(!editor.view_has_box(2, 0));
        assert_eq!(editor.view_selected_box(), None);
    }

    #[test]
    fn play_double_tap_on_player_restarts_active_play_session() {
        let mut editor = setup_unsolved_play_editor();
        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 1,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::PlayCell {
            cell_x: 2,
            cell_y: 0,
        });

        let effects = editor.apply_command(EditorCommand::PlayDoubleTap {
            cell_x: 1,
            cell_y: 0,
        });

        assert!(effects.world_changed);
        assert_eq!(editor.mode(), EditorMode::Play);
        assert_eq!(editor.view_player(), Some((0, 0)));
        assert!(editor.view_has_box(1, 0));
        assert!(!editor.view_has_box(2, 0));
    }

    #[test]
    fn returning_from_move_to_draw_preserves_moved_boxes() {
        let mut editor = setup_simple_playable_editor();

        editor.apply_command(EditorCommand::SetMode(EditorMode::Draw));

        assert!(editor.world.has_box(1, 0));
        assert!(!editor.world.has_box(2, 0));
        assert_eq!(editor.world.player(), Some((0, 0)));
    }

    #[test]
    fn erasing_floor_under_moved_box_restores_it_to_first_available_goal() {
        let mut editor = setup_simple_playable_editor();

        editor.apply_command(EditorCommand::SetMode(EditorMode::Draw));
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 1,
            cell_y: 0,
            tool: DrawTool::Void,
        });

        assert!(!editor.world.has_box(1, 0));
        assert!(editor.world.has_box(2, 0));
    }

    #[test]
    fn restoring_box_to_first_available_goal_uses_sorted_empty_goal() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        for x in 0..=5 {
            paint_floor(&mut editor, x, 0);
        }
        paint_goal_with_box(&mut editor, 0, 0);
        paint_goal_with_box(&mut editor, 2, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 5,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 2,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 4,
            cell_y: 0,
        });

        editor.apply_command(EditorCommand::SetMode(EditorMode::Draw));
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 4,
            cell_y: 0,
            tool: DrawTool::Void,
        });

        assert!(editor.world.has_box(0, 0));
        assert!(!editor.world.has_box(2, 0));
        assert!(!editor.world.has_box(4, 0));
        assert!(editor.world.has_box(5, 0));
    }

    #[test]
    fn restoring_box_to_first_available_goal_removes_player_on_goal() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        paint_goal_with_box(&mut editor, 0, 0);
        paint_floor(&mut editor, 1, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::PositionPlayer {
            cell_x: 0,
            cell_y: 0,
        });

        editor.apply_command(EditorCommand::SetMode(EditorMode::Draw));
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 1,
            cell_y: 0,
            tool: DrawTool::Void,
        });

        assert_eq!(editor.world.player(), None);
        assert!(editor.world.has_box(0, 0));
        assert!(!editor.world.has_box(1, 0));
    }

    #[test]
    fn drawing_goal_with_box_over_moved_box_restores_moved_box_to_first_available_goal() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        paint_goal_with_box(&mut editor, 0, 0);
        paint_floor(&mut editor, 1, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });

        editor.apply_command(EditorCommand::SetMode(EditorMode::Draw));
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 1,
            cell_y: 0,
            tool: DrawTool::GoalWithBox,
        });

        assert_eq!(editor.world.tile(1, 0), Tile::Goal);
        assert!(editor.world.has_box(0, 0));
        assert!(editor.world.has_box(1, 0));
    }

    #[test]
    fn drawing_goal_with_box_on_empty_goal_replaces_first_box() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        paint_goal_with_box(&mut editor, 0, 0);
        paint_floor(&mut editor, 1, 0);
        editor.apply_command(EditorCommand::SetMode(EditorMode::Move));
        editor.apply_command(EditorCommand::SelectBox {
            cell_x: 0,
            cell_y: 0,
        });
        editor.apply_command(EditorCommand::MoveSelectedBoxTo {
            cell_x: 1,
            cell_y: 0,
        });

        editor.apply_command(EditorCommand::SetMode(EditorMode::Draw));
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 0,
            cell_y: 0,
            tool: DrawTool::GoalWithBox,
        });

        assert!(editor.world.has_box(0, 0));
        assert!(!editor.world.has_box(1, 0));
    }

    #[test]
    fn deleting_empty_goal_removes_first_box() {
        let mut editor = setup_simple_playable_editor();

        editor.apply_command(EditorCommand::SetMode(EditorMode::Draw));
        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 2,
            cell_y: 0,
            tool: DrawTool::Void,
        });

        assert!(!editor.world.has_box(1, 0));
        assert!(!editor.world.has_box(2, 0));
    }

    #[test]
    fn deleting_boxed_goal_removes_that_box() {
        let mut editor = LevelEditor::new();
        clear_world(&mut editor);
        paint_goal_with_box(&mut editor, 0, 0);
        paint_goal_with_box(&mut editor, 1, 0);

        editor.apply_command(EditorCommand::PaintCell {
            cell_x: 1,
            cell_y: 0,
            tool: DrawTool::Void,
        });

        assert!(editor.world.has_box(0, 0));
        assert!(!editor.world.has_box(1, 0));
        assert_eq!(editor.world.box_positions().len(), 1);
        assert_eq!(editor.world.goal_positions().len(), 1);
    }
}
