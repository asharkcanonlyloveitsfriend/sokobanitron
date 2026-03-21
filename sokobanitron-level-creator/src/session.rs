use crate::constants::{
    BASE_VISIBLE_COLS, DOUBLE_TAP_WINDOW, GRID_MARGIN_TILES, INITIAL_HEIGHT, INITIAL_WIDTH,
    MAX_VISIBLE_COLS, MIN_VISIBLE_COLS,
};
use crate::ui::{ZoomButtonAction, draw_controls, mode_toggle_button_rect, zoom_button_action_at};
use crate::world::{EditableTile, EditableWorld, NonVoidBounds};
use renderer::{BoardViewport, Renderer};
use sokobanitron_gameplay::{BoardView, TileKind};
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
}

impl LevelCreatorSession {
    pub fn new() -> Self {
        Self {
            renderer: Renderer::new(),
            surface_width: INITIAL_WIDTH,
            surface_height: INITIAL_HEIGHT,
            cursor_position: None,
            mouse_paint_mode: None,
            active_touch_paint: None,
            mode: EditorMode::Draw,
            view_center_x: 0,
            view_center_y: 0,
            zoom_steps: 0,
            selected_box: None,
            last_tap: None,
            world: EditableWorld::new(),
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
            false,
            false,
        );
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

        let mut tiles = Vec::with_capacity((board_cols * board_rows) as usize);
        let mut boxes = Vec::with_capacity((board_cols * board_rows) as usize);
        let mut selected_box_local = None;
        for y in 0..board_rows {
            for x in 0..board_cols {
                let world_x = world_origin_x + x as i32;
                let world_y = world_origin_y + y as i32;
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
            None,
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

    fn paint_world_cell(&mut self, world_x: i32, world_y: i32, mode: PaintMode) {
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
        match self.mode {
            EditorMode::Draw => {
                let mode = self.resolve_paint_mode(world_x, world_y);
                self.paint_world_cell(world_x, world_y, mode);
                Some(mode)
            }
            EditorMode::Manipulate => {
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

    fn handle_manipulate_tap(&mut self, world_x: i32, world_y: i32) {
        let tapped_tile = self.world.tile(world_x, world_y);
        if Self::is_box_tile(tapped_tile) {
            self.selected_box = if self.selected_box == Some((world_x, world_y)) {
                None
            } else {
                Some((world_x, world_y))
            };
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
            return;
        }

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
        self.selected_box = None;
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
    }
}

impl Default for LevelCreatorSession {
    fn default() -> Self {
        Self::new()
    }
}
