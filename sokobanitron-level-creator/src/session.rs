use crate::constants::{
    BASE_VISIBLE_COLS, DOUBLE_TAP_WINDOW, GRID_MARGIN_TILES, INITIAL_HEIGHT, INITIAL_WIDTH,
    MAX_VISIBLE_COLS, MIN_VISIBLE_COLS, START_IN_MANIPULATE_MODE,
};
use crate::ui::{
    ManipulateButtonAction, ScreenRect, ZoomButtonAction, draw_box_move_count, draw_controls,
    draw_move_hint_count, manipulate_button_action_at, mode_toggle_button_rect,
    zoom_button_action_at,
};
use crate::world::{EditableTile, EditableWorld, NonVoidBounds};
use renderer::{BoardViewport, Renderer};
use sokobanitron_core::optimizer::{ReverseOptimizationInput, optimize_reverse_solution_in_place};
use sokobanitron_core::pathfinder::{Position, PullPathfinder};
use sokobanitron_gameplay::{BoardView, TileKind};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorMode {
    Draw,
    Manipulate,
}

#[derive(Clone, Copy)]
enum PaintMode {
    SetFloor,
    SetBoxOnGoal,
    SetVoid,
}

impl PaintMode {
    fn from_start_tile(tile: EditableTile) -> Self {
        if matches!(tile, EditableTile::Void) {
            Self::SetFloor
        } else {
            Self::SetVoid
        }
    }
}

struct VisibleBoardWindow {
    board: BoardView,
    viewport: BoardViewport,
    world_origin_x: i32,
    world_origin_y: i32,
}

#[derive(Clone, Copy)]
struct LastTap {
    world_x: i32,
    world_y: i32,
    at: Instant,
}

struct PullMovePlan {
    player_start: (i32, i32),
    box_path: Vec<(i32, i32)>,
}

#[derive(Clone)]
struct UndoSnapshot {
    world: EditableWorld,
    solution_start_boxes: Vec<(i32, i32)>,
    solution_start_player: Option<(i32, i32)>,
    solution_history: Vec<Vec<(i32, i32)>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchInputPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

pub struct LevelCreatorSession {
    renderer: Renderer,
    surface_width: u32,
    surface_height: u32,
    cursor_position: Option<(f64, f64)>,
    mouse_paint_mode: Option<PaintMode>,
    active_touch_paint: Option<(u64, PaintMode)>,
    mode: EditorMode,
    view_center_x: i32,
    view_center_y: i32,
    zoom_steps: i32,
    selected_box: Option<(i32, i32)>,
    last_tap: Option<LastTap>,
    world: EditableWorld,
    solution_start_boxes: Vec<(i32, i32)>,
    solution_start_player: Option<(i32, i32)>,
    solution_history: Vec<Vec<(i32, i32)>>,
    undo_history: Vec<UndoSnapshot>,
    pull_destination_hints: HashMap<(i32, i32), u32>,
    pull_hints_dirty: bool,
}

impl LevelCreatorSession {
    pub fn new() -> Self {
        let world = EditableWorld::new();
        let solution_start_boxes = world.box_positions();
        let solution_start_player = world.player();
        Self {
            renderer: Renderer::new(),
            surface_width: INITIAL_WIDTH,
            surface_height: INITIAL_HEIGHT,
            cursor_position: None,
            mouse_paint_mode: None,
            active_touch_paint: None,
            mode: if START_IN_MANIPULATE_MODE {
                EditorMode::Manipulate
            } else {
                EditorMode::Draw
            },
            view_center_x: 0,
            view_center_y: 0,
            zoom_steps: 0,
            selected_box: None,
            last_tap: None,
            world,
            solution_start_boxes,
            solution_start_player,
            solution_history: Vec::new(),
            undo_history: Vec::new(),
            pull_destination_hints: HashMap::new(),
            pull_hints_dirty: false,
        }
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_width = width.max(1);
        self.surface_height = height.max(1);
        self.zoom_steps = self.clamp_zoom_steps(self.zoom_steps);
    }

    pub fn cursor_moved(&mut self, x: f64, y: f64) {
        self.cursor_position = Some((x, y));
        if let Some(mode) = self.mouse_paint_mode {
            self.continue_paint_stroke(x, y, mode);
        }
    }

    pub fn mouse_pressed_left(&mut self) {
        if let Some((x, y)) = self.cursor_position {
            self.mouse_paint_mode = self.begin_paint_stroke(x, y);
        }
    }

    pub fn mouse_released_left(&mut self) {
        self.mouse_paint_mode = None;
    }

    pub fn touch(&mut self, id: u64, phase: TouchInputPhase, x: f64, y: f64) {
        match phase {
            TouchInputPhase::Started => {
                if self.active_touch_paint.is_none() {
                    self.active_touch_paint = self.begin_paint_stroke(x, y).map(|mode| (id, mode));
                }
            }
            TouchInputPhase::Moved => {
                if let Some((active_id, mode)) = self.active_touch_paint
                    && active_id == id
                {
                    self.continue_paint_stroke(x, y, mode);
                }
            }
            TouchInputPhase::Ended | TouchInputPhase::Cancelled => {
                if self
                    .active_touch_paint
                    .is_some_and(|(active_id, _)| active_id == id)
                {
                    self.active_touch_paint = None;
                }
            }
        }
    }

    pub fn reset_interaction_state(&mut self) {
        self.mouse_paint_mode = None;
        self.active_touch_paint = None;
    }

    pub fn render(&mut self, frame: &mut [u8], width: u32, height: u32) {
        self.resize_surface(width, height);
        self.refresh_pull_destination_hints_if_needed();

        let visible = self.build_visible_window();
        let can_zoom_out = self.can_zoom_out();
        let can_zoom_in = self.can_zoom_in();

        self.renderer
            .draw_background_only(frame, self.surface_width, self.surface_height);
        self.renderer.draw_board_on_frame(
            frame,
            self.surface_width,
            self.surface_height,
            &visible.board,
            &visible.viewport,
            true,
            false,
        );
        self.draw_box_move_counts_on_visible_window(frame, &visible);
        self.draw_pull_destination_hints_on_visible_window(frame, &visible);
        draw_controls(
            frame,
            self.surface_width,
            self.surface_height,
            can_zoom_out,
            can_zoom_in,
            matches!(self.mode, EditorMode::Draw),
        );
    }

    fn build_visible_window(&self) -> VisibleBoardWindow {
        let (board_cols, board_rows) = self.board_dimensions_for_steps(self.zoom_steps);
        let cell_size = self.compute_cell_size(board_cols);
        let world_origin_x = self.view_center_x - (board_cols as i32 / 2);
        let world_origin_y = self.view_center_y - (board_rows as i32 / 2);
        let hide_player = self.selected_box.is_some();

        let mut tiles = Vec::with_capacity((board_cols * board_rows) as usize);
        let mut boxes = Vec::with_capacity((board_cols * board_rows) as usize);
        let mut selected_box_local = None;
        let mut player_local = None;
        for y in 0..board_rows {
            for x in 0..board_cols {
                let world_x = world_origin_x + x as i32;
                let world_y = world_origin_y + y as i32;
                if !hide_player && self.world.player() == Some((world_x, world_y)) {
                    player_local = Some((x, y));
                }
                match self.world.tile(world_x, world_y) {
                    EditableTile::Void => {
                        tiles.push(TileKind::Void);
                        boxes.push(false);
                    }
                    EditableTile::Floor => {
                        tiles.push(TileKind::Floor);
                        boxes.push(false);
                    }
                    EditableTile::Goal => {
                        tiles.push(TileKind::Goal);
                        boxes.push(false);
                    }
                    EditableTile::Box => {
                        tiles.push(TileKind::Floor);
                        boxes.push(true);
                        if self.selected_box == Some((world_x, world_y))
                            && matches!(self.mode, EditorMode::Manipulate)
                        {
                            selected_box_local = Some((x, y));
                        }
                    }
                    EditableTile::BoxOnGoal => {
                        tiles.push(TileKind::Goal);
                        boxes.push(true);
                        if self.selected_box == Some((world_x, world_y))
                            && matches!(self.mode, EditorMode::Manipulate)
                        {
                            selected_box_local = Some((x, y));
                        }
                    }
                }
            }
        }
        let board = BoardView::new(
            board_cols,
            board_rows,
            tiles,
            boxes,
            player_local,
            selected_box_local,
            false,
        );

        let board_pixel_width = (board_cols + GRID_MARGIN_TILES.saturating_mul(2)) * cell_size;
        let board_pixel_height = (board_rows + GRID_MARGIN_TILES.saturating_mul(2)) * cell_size;
        let viewport = BoardViewport {
            origin_x: ((self.surface_width as i32) - (board_pixel_width as i32)) / 2,
            origin_y: ((self.surface_height as i32) - (board_pixel_height as i32)) / 2,
            cell_size,
            board_pixel_width,
            board_pixel_height,
            outer_margin_tiles: GRID_MARGIN_TILES,
        };

        VisibleBoardWindow {
            board,
            viewport,
            world_origin_x,
            world_origin_y,
        }
    }

    fn board_dimensions_for_steps(&self, zoom_steps: i32) -> (u32, u32) {
        let steps = self.clamp_zoom_steps(zoom_steps);
        let cols_i32 = (BASE_VISIBLE_COLS as i32 + steps * 2).max(1);
        let cols = cols_i32 as u32;
        let cell_size = self.compute_cell_size(cols);
        let rows = self.compute_visible_rows(cell_size);
        (cols, rows)
    }

    fn compute_cell_size(&self, board_cols: u32) -> u32 {
        let cols_with_margins = board_cols + GRID_MARGIN_TILES.saturating_mul(2);
        (self.surface_width / cols_with_margins).max(1)
    }

    fn compute_visible_rows(&self, cell_size: u32) -> u32 {
        let total_rows = (self.surface_height / cell_size).max(1);
        let mut board_rows = total_rows
            .saturating_sub(GRID_MARGIN_TILES.saturating_mul(2))
            .max(1);
        if board_rows % 2 == 0 {
            board_rows = board_rows.saturating_sub(1).max(1);
        }
        board_rows
    }

    fn min_zoom_in_steps(&self) -> i32 {
        -((BASE_VISIBLE_COLS.saturating_sub(MIN_VISIBLE_COLS) / 2) as i32)
    }

    fn max_zoom_out_steps(&self) -> i32 {
        (MAX_VISIBLE_COLS.saturating_sub(BASE_VISIBLE_COLS) / 2) as i32
    }

    fn clamp_zoom_steps(&self, steps: i32) -> i32 {
        steps.clamp(self.min_zoom_in_steps(), self.max_zoom_out_steps())
    }

    fn can_zoom_out(&self) -> bool {
        self.zoom_steps < self.max_zoom_out_steps()
    }

    fn bounds_fit_with_center(
        &self,
        bounds: NonVoidBounds,
        center_x: i32,
        center_y: i32,
        cols: u32,
        rows: u32,
    ) -> bool {
        let origin_x = center_x - cols as i32 / 2;
        let origin_y = center_y - rows as i32 / 2;
        let max_x = origin_x + cols as i32 - 1;
        let max_y = origin_y + rows as i32 - 1;
        bounds.min_x >= origin_x
            && bounds.max_x <= max_x
            && bounds.min_y >= origin_y
            && bounds.max_y <= max_y
    }

    fn centered_view_for_bounds(
        &self,
        bounds: NonVoidBounds,
        cols: u32,
        rows: u32,
    ) -> Option<(i32, i32)> {
        let half_cols = cols as i32 / 2;
        let half_rows = rows as i32 / 2;

        let min_center_x = bounds.max_x - half_cols;
        let max_center_x = bounds.min_x + half_cols;
        if min_center_x > max_center_x {
            return None;
        }

        let min_center_y = bounds.max_y - half_rows;
        let max_center_y = bounds.min_y + half_rows;
        if min_center_y > max_center_y {
            return None;
        }

        let center_x = self.view_center_x.clamp(min_center_x, max_center_x);
        let center_y = self.view_center_y.clamp(min_center_y, max_center_y);
        Some((center_x, center_y))
    }

    fn can_zoom_in(&self) -> bool {
        if self.zoom_steps <= self.min_zoom_in_steps() {
            return false;
        }

        let target_steps = self.zoom_steps - 1;
        let (target_cols, target_rows) = self.board_dimensions_for_steps(target_steps);
        let Some(bounds) = self.world.non_void_bounds() else {
            return true;
        };

        if self.bounds_fit_with_center(
            bounds,
            self.view_center_x,
            self.view_center_y,
            target_cols,
            target_rows,
        ) {
            return true;
        }

        self.centered_view_for_bounds(bounds, target_cols, target_rows)
            .is_some()
    }

    fn apply_zoom(&mut self, action: ZoomButtonAction) {
        match action {
            ZoomButtonAction::ZoomIn => {
                if self.zoom_steps <= self.min_zoom_in_steps() {
                    return;
                }
                let target_steps = self.zoom_steps - 1;
                let (target_cols, target_rows) = self.board_dimensions_for_steps(target_steps);
                if let Some(bounds) = self.world.non_void_bounds() {
                    if self.bounds_fit_with_center(
                        bounds,
                        self.view_center_x,
                        self.view_center_y,
                        target_cols,
                        target_rows,
                    ) {
                        self.zoom_steps = target_steps;
                    } else if let Some((center_x, center_y)) =
                        self.centered_view_for_bounds(bounds, target_cols, target_rows)
                    {
                        self.view_center_x = center_x;
                        self.view_center_y = center_y;
                        self.zoom_steps = target_steps;
                    }
                } else {
                    self.zoom_steps = target_steps;
                }
            }
            ZoomButtonAction::ZoomOut => {
                if self.can_zoom_out() {
                    self.zoom_steps += 1;
                }
            }
        }
    }

    fn world_cell_at_screen_position(&self, screen_x: f64, screen_y: f64) -> Option<(i32, i32)> {
        let visible = self.build_visible_window();
        visible
            .viewport
            .screen_to_cell(screen_x, screen_y, &visible.board)
            .map(|(x, y)| {
                (
                    visible.world_origin_x + x as i32,
                    visible.world_origin_y + y as i32,
                )
            })
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
        self.last_tap = None;
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
        self.last_tap = None;
        self.mouse_paint_mode = None;
        self.active_touch_paint = None;
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

    fn box_move_counts_by_position_for_history(
        &self,
        history: &[Vec<(i32, i32)>],
    ) -> HashMap<(i32, i32), u32> {
        let mut box_counts = self
            .solution_start_boxes
            .iter()
            .copied()
            .map(|pos| (pos, 0u32))
            .collect::<HashMap<_, _>>();

        for movement in history {
            let Some(start_pos) = movement.first().copied() else {
                continue;
            };
            let Some(end_pos) = movement.last().copied() else {
                continue;
            };
            let Some(current_count) = box_counts.remove(&start_pos) else {
                continue;
            };
            box_counts.insert(end_pos, current_count.saturating_add(1));
        }

        box_counts
    }

    fn box_move_counts_by_position(&self) -> HashMap<(i32, i32), u32> {
        self.box_move_counts_by_position_for_history(&self.solution_history)
    }

    fn draw_box_move_counts_on_visible_window(
        &self,
        frame: &mut [u8],
        visible: &VisibleBoardWindow,
    ) {
        let box_counts = self.box_move_counts_by_position();
        for y in 0..visible.board.height() {
            for x in 0..visible.board.width() {
                let world_x = visible.world_origin_x + x as i32;
                let world_y = visible.world_origin_y + y as i32;
                if !Self::is_box_tile(self.world.tile(world_x, world_y)) {
                    continue;
                }

                let count = box_counts.get(&(world_x, world_y)).copied().unwrap_or(0);
                let (cell_x, cell_y, cell_w, cell_h) = visible.viewport.cell_to_screen_rect(x, y);
                let inset = (cell_w / 24).max(1);
                let box_x = cell_x + inset as i32;
                let box_y = cell_y + inset as i32;
                if box_x < 0 || box_y < 0 {
                    continue;
                }

                let rect = ScreenRect {
                    x: box_x as u32,
                    y: box_y as u32,
                    w: cell_w.saturating_sub(inset * 2),
                    h: cell_h.saturating_sub(inset * 2),
                };
                if rect.w == 0 || rect.h == 0 {
                    continue;
                }
                draw_box_move_count(frame, self.surface_width, self.surface_height, rect, count);
            }
        }
    }

    fn draw_pull_destination_hints_on_visible_window(
        &self,
        frame: &mut [u8],
        visible: &VisibleBoardWindow,
    ) {
        if !matches!(self.mode, EditorMode::Manipulate) || self.selected_box.is_none() {
            return;
        }

        let width = visible.board.width() as i32;
        let height = visible.board.height() as i32;
        for (&(world_x, world_y), &count) in &self.pull_destination_hints {
            let local_x = world_x - visible.world_origin_x;
            let local_y = world_y - visible.world_origin_y;
            if local_x < 0 || local_y < 0 || local_x >= width || local_y >= height {
                continue;
            }
            let (cell_x, cell_y, cell_w, cell_h) = visible
                .viewport
                .cell_to_screen_rect(local_x as u32, local_y as u32);
            if cell_x < 0 || cell_y < 0 {
                continue;
            }
            let rect = ScreenRect {
                x: cell_x as u32,
                y: cell_y as u32,
                w: cell_w,
                h: cell_h,
            };
            draw_move_hint_count(frame, self.surface_width, self.surface_height, rect, count);
        }
    }

    fn clear_pull_destination_hints(&mut self) {
        self.pull_destination_hints.clear();
        self.pull_hints_dirty = false;
    }

    fn mark_pull_destination_hints_dirty(&mut self) {
        if matches!(self.mode, EditorMode::Manipulate) && self.selected_box.is_some() {
            self.pull_hints_dirty = true;
        } else {
            self.clear_pull_destination_hints();
        }
    }

    fn compute_pull_destination_hints(&self, selected: (i32, i32)) -> HashMap<(i32, i32), u32> {
        let Some(bounds) = self.world.non_void_bounds() else {
            return HashMap::new();
        };
        let optimization_input = ReverseOptimizationInput {
            walkable_cells: self.walkable_cells_for_optimizer(),
            box_positions: self.solution_start_boxes.clone(),
            player: self.solution_start_player,
        };
        let mut hints = HashMap::new();
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                if (x, y) == selected {
                    continue;
                }
                match self.world.tile(x, y) {
                    EditableTile::Void | EditableTile::Box | EditableTile::BoxOnGoal => continue,
                    EditableTile::Floor | EditableTile::Goal => {}
                }
                let Some(plan) = self.find_pull_move_plan(selected.0, selected.1, x, y) else {
                    continue;
                };

                let mut candidate_history = self.solution_history.clone();
                candidate_history.push(plan.box_path);
                optimize_reverse_solution_in_place(&optimization_input, &mut candidate_history);
                let candidate_counts =
                    self.box_move_counts_by_position_for_history(&candidate_history);
                if let Some(count) = candidate_counts.get(&(x, y)).copied() {
                    hints.insert((x, y), count);
                }
            }
        }
        hints
    }

    fn refresh_pull_destination_hints_if_needed(&mut self) {
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
        self.pull_destination_hints = self.compute_pull_destination_hints(selected);
        self.pull_hints_dirty = false;
    }

    fn paint_world_cell(&mut self, world_x: i32, world_y: i32, mode: PaintMode) {
        let original_tile = self.world.tile(world_x, world_y);
        match mode {
            PaintMode::SetFloor => self.world.set_tile(world_x, world_y, EditableTile::Floor),
            PaintMode::SetBoxOnGoal => {
                self.world
                    .set_tile(world_x, world_y, EditableTile::BoxOnGoal)
            }
            PaintMode::SetVoid => self.world.set_tile(world_x, world_y, EditableTile::Void),
        }
        let updated_tile = self.world.tile(world_x, world_y);
        if self.selected_box == Some((world_x, world_y)) && !Self::is_box_tile(updated_tile) {
            self.selected_box = None;
        }
        if original_tile != updated_tile {
            self.reset_solution_tracking();
            self.mark_pull_destination_hints_dirty();
        }
    }

    fn resolve_paint_mode(&mut self, world_x: i32, world_y: i32) -> PaintMode {
        let current_tile = self.world.tile(world_x, world_y);
        if current_tile == EditableTile::BoxOnGoal {
            self.last_tap = None;
            return PaintMode::SetVoid;
        }

        let now = Instant::now();
        let is_double_tap = self.last_tap.is_some_and(|last| {
            last.world_x == world_x
                && last.world_y == world_y
                && now.duration_since(last.at) <= DOUBLE_TAP_WINDOW
        });

        if is_double_tap {
            self.last_tap = None;
            PaintMode::SetBoxOnGoal
        } else {
            self.last_tap = Some(LastTap {
                world_x,
                world_y,
                at: now,
            });
            PaintMode::from_start_tile(current_tile)
        }
    }

    fn begin_paint_stroke(&mut self, screen_x: f64, screen_y: f64) -> Option<PaintMode> {
        if mode_toggle_button_rect().contains(screen_x, screen_y) {
            self.toggle_mode();
            return None;
        }
        match self.mode {
            EditorMode::Draw => {
                if let Some(action) = zoom_button_action_at(
                    screen_x,
                    screen_y,
                    self.surface_width,
                    self.surface_height,
                    self.can_zoom_out(),
                    self.can_zoom_in(),
                ) {
                    self.apply_zoom(action);
                    return None;
                }
                let (world_x, world_y) = self.world_cell_at_screen_position(screen_x, screen_y)?;
                let mode = self.resolve_paint_mode(world_x, world_y);
                self.paint_world_cell(world_x, world_y, mode);
                Some(mode)
            }
            EditorMode::Manipulate => {
                if let Some(action) = manipulate_button_action_at(
                    screen_x,
                    screen_y,
                    self.surface_width,
                    self.surface_height,
                ) {
                    match action {
                        ManipulateButtonAction::Restart => self.restart_to_goals(),
                        ManipulateButtonAction::Undo => self.undo_last_move(),
                    }
                    return None;
                }
                let (world_x, world_y) = self.world_cell_at_screen_position(screen_x, screen_y)?;
                self.handle_manipulate_tap(world_x, world_y);
                None
            }
        }
    }

    fn continue_paint_stroke(&mut self, screen_x: f64, screen_y: f64, mode: PaintMode) {
        if !matches!(self.mode, EditorMode::Draw) {
            return;
        }
        if let Some((world_x, world_y)) = self.world_cell_at_screen_position(screen_x, screen_y) {
            self.paint_world_cell(world_x, world_y, mode);
        }
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

    fn find_pull_move_plan(
        &self,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
    ) -> Option<PullMovePlan> {
        let Some(bounds) = self.world.non_void_bounds() else {
            return None;
        };

        let mut min_x = bounds.min_x.min(from_x).min(to_x);
        let mut max_x = bounds.max_x.max(from_x).max(to_x);
        let mut min_y = bounds.min_y.min(from_y).min(to_y);
        let mut max_y = bounds.max_y.max(from_y).max(to_y);
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

    fn handle_manipulate_tap(&mut self, world_x: i32, world_y: i32) {
        let tapped_tile = self.world.tile(world_x, world_y);
        if Self::is_box_tile(tapped_tile) {
            self.selected_box = if self.selected_box == Some((world_x, world_y)) {
                None
            } else {
                Some((world_x, world_y))
            };
            self.mark_pull_destination_hints_dirty();
            return;
        }

        if matches!(tapped_tile, EditableTile::Void) {
            self.selected_box = None;
            self.clear_pull_destination_hints();
            return;
        }

        let Some((from_x, from_y)) = self.selected_box else {
            return;
        };
        if from_x == world_x && from_y == world_y {
            return;
        }

        let from_tile = self.world.tile(from_x, from_y);
        if !Self::is_box_tile(from_tile) {
            self.selected_box = None;
            self.clear_pull_destination_hints();
            return;
        }

        let Some(plan) = self.find_pull_move_plan(from_x, from_y, world_x, world_y) else {
            return;
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
    }

    fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            EditorMode::Draw => EditorMode::Manipulate,
            EditorMode::Manipulate => EditorMode::Draw,
        };
        self.mouse_paint_mode = None;
        self.active_touch_paint = None;
        self.selected_box = None;
        self.last_tap = None;
        self.clear_pull_destination_hints();
    }
}

impl Default for LevelCreatorSession {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::LevelCreatorSession;
    use crate::world::EditableTile;

    #[test]
    fn consecutive_moves_of_the_same_box_are_consolidated() {
        let mut session = LevelCreatorSession::new();
        session.solution_start_boxes = vec![(0, 0)];
        session.solution_history.clear();

        session.record_box_move(vec![(0, 0), (1, 0)]);
        session.record_box_move(vec![(1, 0), (2, 0)]);
        session.record_box_move(vec![(2, 0), (3, 0)]);

        assert_eq!(session.solution_history.len(), 1);
        assert_eq!(
            session.solution_history[0],
            vec![(0, 0), (1, 0), (2, 0), (3, 0)]
        );

        let counts = session.box_move_counts_by_position();
        assert_eq!(counts.get(&(3, 0)).copied(), Some(1));
    }

    #[test]
    fn non_consecutive_moves_are_not_consolidated() {
        let mut session = LevelCreatorSession::new();
        session.solution_start_boxes = vec![(0, 0), (2, 0)];
        session.solution_history.clear();

        session.record_box_move(vec![(0, 0), (1, 0)]);
        session.record_box_move(vec![(2, 0), (3, 0)]);
        session.record_box_move(vec![(1, 0), (0, 0)]);

        assert_eq!(session.solution_history.len(), 3);

        let counts = session.box_move_counts_by_position();
        assert_eq!(counts.get(&(0, 0)).copied(), Some(2));
        assert_eq!(counts.get(&(3, 0)).copied(), Some(1));
    }

    #[test]
    fn undo_restores_previous_solution_snapshot() {
        let mut session = LevelCreatorSession::new();
        let snapshot = session.make_undo_snapshot();

        session.world.set_tile(0, 0, EditableTile::Void);
        session.world.set_player(Some((1, 1)));
        session.solution_history.push(vec![(1, 1), (1, 2)]);
        session.undo_history.push(snapshot);
        session.undo_last_move();

        assert!(session.solution_history.is_empty());
        assert_ne!(session.world.tile(0, 0), EditableTile::Void);
        assert_eq!(session.world.player(), None);
    }

    #[test]
    fn restart_resets_boxes_on_goals_and_clears_undo_history() {
        let mut session = LevelCreatorSession::new();
        session.world.set_tile(-2, -1, EditableTile::Goal);
        session.world.set_tile(0, 0, EditableTile::Box);
        session.solution_history.push(vec![(-2, -1), (0, 0)]);
        session.undo_history.push(session.make_undo_snapshot());

        session.restart_to_goals();

        assert!(session.solution_history.is_empty());
        assert!(session.undo_history.is_empty());
        assert_eq!(session.world.player(), None);
        for (x, y) in session.world.box_positions() {
            assert_eq!(session.world.tile(x, y), EditableTile::BoxOnGoal);
        }
    }
}
