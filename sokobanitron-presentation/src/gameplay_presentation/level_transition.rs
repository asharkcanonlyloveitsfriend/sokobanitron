use crate::layout::{BoardViewport, ScreenRect};
use crate::renderer::Renderer;
use crate::screen_requests::{
    GameplayPresentationCause, GameplayPresentationUpdate, GameplayScreenRequest,
};
use sokobanitron_gameplay::{BoardCell, TileKind};
use std::time::{Duration, Instant};

const TICKS_PER_STEP: u32 = 2;
const ANIMATION_TICK: Duration = Duration::from_millis(50);

const SWEEP_STEP_PERCENT: f32 = 14.0;
const FLASH_GAP_STEPS: f32 = 2.0;

// Normalized diagonal sweep coordinate: 0 at lower-left, 2 at upper-right.
const SWEEP_STEP: f32 = (SWEEP_STEP_PERCENT / 100.0) * 2.0;
const SWEEP_BAND_WIDTH: f32 = 3.0 * SWEEP_STEP;
const FLASH_GAP: f32 = FLASH_GAP_STEPS * SWEEP_STEP;

#[derive(Debug, Clone)]
pub(super) struct LevelTransition {
    state: TransitionState,
    scratch: TransitionScratch,
    next_frame_at: Instant,
}

impl LevelTransition {
    pub(super) fn for_update(
        previous_scene: Option<&GameplayScreenRequest>,
        update: &GameplayPresentationUpdate,
        now: Instant,
    ) -> Option<Self> {
        let previous_scene = previous_scene?;
        if !matches!(update.cause, GameplayPresentationCause::LevelTransition) {
            return None;
        }
        if previous_scene == &update.scene {
            return None;
        }
        if previous_scene.mode != update.scene.mode {
            return None;
        }
        Some(Self {
            state: TransitionState::new(previous_scene.clone(), update.scene.clone()),
            scratch: TransitionScratch::default(),
            next_frame_at: now,
        })
    }

    pub(super) fn is_done(&self) -> bool {
        self.state
            .flash_tiles
            .iter()
            .all(|tile| tile.phase_index >= TILE_FLASH_PHASES.len())
    }

    pub(super) fn is_ready_to_draw(&self, now: Instant) -> bool {
        self.is_done() || now >= self.next_frame_at
    }

    fn draw(&mut self, renderer: &mut Renderer, frame: &mut [u8], width: u32, height: u32) {
        self.state
            .draw(renderer, &mut self.scratch, frame, width, height);
    }

    pub(super) fn draw_and_step(
        &mut self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        now: Instant,
    ) -> bool {
        if self.is_done() {
            self.state.draw_final(renderer, frame, width, height);
            return true;
        }
        self.draw(renderer, frame, width, height);
        if self.is_ready_to_draw(now) {
            self.state.finish_drawn_frame();
            self.next_frame_at = next_frame_at(now);
        }
        false
    }
}

#[derive(Debug, Clone, Default)]
struct TransitionScratch {
    width: u32,
    height: u32,
    background: Vec<u8>,
    new_bitmap: Vec<u8>,
}

impl TransitionScratch {
    fn prepare_transition_bitmaps(
        &mut self,
        renderer: &mut Renderer,
        frame_len: usize,
        width: u32,
        height: u32,
        scene: &GameplayScreenRequest,
    ) -> (&[u8], &[u8]) {
        self.background.resize(frame_len, 0);
        renderer.draw_background_only(&mut self.background, width, height);

        if self.width != width || self.height != height || self.new_bitmap.len() != frame_len {
            self.width = width;
            self.height = height;
            self.new_bitmap = vec![0; frame_len];
            renderer.draw_background_only(&mut self.new_bitmap, width, height);
            renderer.draw_floor_tiles(
                &mut self.new_bitmap,
                width,
                height,
                &scene.board,
                &scene.viewport,
            );
        }

        (&self.background, &self.new_bitmap)
    }
}

fn next_frame_at(now: Instant) -> Instant {
    now + ANIMATION_TICK * TICKS_PER_STEP
}

#[derive(Debug, Clone)]
struct TransitionState {
    old_scene: GameplayScreenRequest,
    new_scene: GameplayScreenRequest,
    union_board_rect: ScreenRect,
    flash_tiles: Vec<FlashTile>,
    step_index: u32,
}

impl TransitionState {
    fn new(old_scene: GameplayScreenRequest, new_scene: GameplayScreenRequest) -> Self {
        let old_board_rect = interior_board_rect(&old_scene.viewport, &old_scene.board);
        let new_board_rect = interior_board_rect(&new_scene.viewport, &new_scene.board);
        let union_board_rect = union_screen_rect(old_board_rect, new_board_rect);
        let flash_tiles = flash_tiles(&new_scene, union_board_rect);
        Self {
            old_scene,
            new_scene,
            union_board_rect,
            flash_tiles,
            step_index: 0,
        }
    }

    fn finish_drawn_frame(&mut self) {
        self.step_index = self.step_index.saturating_add(1);
        let back = self.front_sweep_position() - SWEEP_BAND_WIDTH;
        for tile in &mut self.flash_tiles {
            if tile.phase_index >= TILE_FLASH_PHASES.len() {
                continue;
            }
            if back < tile.completion_s + FLASH_GAP {
                continue;
            }
            tile.phase_index += 1;
        }
    }

    fn draw_final(&self, renderer: &mut Renderer, frame: &mut [u8], width: u32, height: u32) {
        renderer.draw_gameplay_scene_with_style_and_animation(
            frame,
            width,
            height,
            &self.new_scene,
            crate::renderer::EntityVisualStyle::Standard,
            &crate::gameplay_animation::GameplayAnimationRunner::default(),
        );
    }

    fn front_sweep_position(&self) -> f32 {
        self.step_index as f32 * SWEEP_STEP
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        scratch: &mut TransitionScratch,
        frame: &mut [u8],
        width: u32,
        height: u32,
    ) {
        let (background, new_bitmap) = scratch.prepare_transition_bitmaps(
            renderer,
            frame.len(),
            width,
            height,
            &self.new_scene,
        );
        frame.copy_from_slice(background);

        let front_sweep_position = self.front_sweep_position();
        let mut surface = TransitionSurface {
            frame,
            width,
            height,
        };
        draw_diagonal_sweep_band(
            &mut surface,
            self,
            -1000.0,
            front_sweep_position - SWEEP_BAND_WIDTH,
            new_bitmap,
            BandPaint::Normal,
        );
        draw_diagonal_sweep_band(
            &mut surface,
            self,
            front_sweep_position - SWEEP_BAND_WIDTH,
            front_sweep_position - 2.0 * SWEEP_STEP,
            new_bitmap,
            BandPaint::Invert,
        );
        draw_diagonal_sweep_band(
            &mut surface,
            self,
            front_sweep_position - 2.0 * SWEEP_STEP,
            front_sweep_position - SWEEP_STEP,
            new_bitmap,
            BandPaint::Normal,
        );
        draw_diagonal_sweep_band(
            &mut surface,
            self,
            front_sweep_position - SWEEP_STEP,
            front_sweep_position,
            background,
            BandPaint::Invert,
        );

        let flash_back =
            ((self.step_index.saturating_add(1)) as f32 * SWEEP_STEP) - SWEEP_BAND_WIDTH;
        for tile in self
            .flash_tiles
            .iter()
            .filter(|tile| flash_back >= tile.completion_s + FLASH_GAP)
        {
            match TILE_FLASH_PHASES.get(tile.phase_index).copied() {
                Some(TileFlashPhase::Black) => {
                    fill_rect(surface.frame, surface.width, surface.height, tile.rect, 0)
                }
                Some(TileFlashPhase::White) => {
                    fill_rect(surface.frame, surface.width, surface.height, tile.rect, 255)
                }
                Some(TileFlashPhase::Base) | None => {}
            }
        }
    }
}

#[derive(Debug, Clone)]
struct FlashTile {
    rect: ScreenRect,
    completion_s: f32,
    phase_index: usize,
}

#[derive(Debug, Clone, Copy)]
enum TileFlashPhase {
    Black,
    Base,
    White,
}

const TILE_FLASH_PHASES: [TileFlashPhase; 5] = [
    TileFlashPhase::Black,
    TileFlashPhase::Base,
    TileFlashPhase::White,
    TileFlashPhase::Base,
    TileFlashPhase::Black,
];

#[derive(Debug, Clone, Copy)]
enum BandPaint {
    Normal,
    Invert,
}

struct TransitionSurface<'a> {
    frame: &'a mut [u8],
    width: u32,
    height: u32,
}

fn draw_diagonal_sweep_band(
    surface: &mut TransitionSurface<'_>,
    state: &TransitionState,
    a: f32,
    b: f32,
    source: &[u8],
    paint: BandPaint,
) {
    let lo = a.min(b);
    let hi = a.max(b);
    let board_width = state.union_board_rect.w as f32;
    let board_height = state.union_board_rect.h as f32;
    if board_width <= 0.0 || board_height <= 0.0 {
        return;
    }

    let board_left = state.union_board_rect.x as f32;
    let board_bottom = state
        .union_board_rect
        .y
        .saturating_add(state.union_board_rect.h) as f32;
    let slice_width = (board_width * (SWEEP_STEP_PERCENT / 100.0)).max(1.0);
    let right = state
        .union_board_rect
        .x
        .saturating_add(state.union_board_rect.w) as f32;
    let mut x = state.union_board_rect.x as f32;
    while x < right {
        let x2 = (x + slice_width).min(right);
        let y_top_0 =
            y_for_sweep_position(hi, x, board_width, board_height, board_left, board_bottom);
        let y_top_1 =
            y_for_sweep_position(hi, x2, board_width, board_height, board_left, board_bottom);
        let y_bot_0 =
            y_for_sweep_position(lo, x, board_width, board_height, board_left, board_bottom);
        let y_bot_1 =
            y_for_sweep_position(lo, x2, board_width, board_height, board_left, board_bottom);
        let top = y_top_0.min(y_top_1).clamp(
            state.union_board_rect.y as f32,
            state
                .union_board_rect
                .y
                .saturating_add(state.union_board_rect.h) as f32,
        );
        let bottom = y_bot_0.max(y_bot_1).clamp(
            state.union_board_rect.y as f32,
            state
                .union_board_rect
                .y
                .saturating_add(state.union_board_rect.h) as f32,
        );
        if top < bottom {
            blit_transition_rect(
                surface,
                source,
                ScreenRect {
                    x: x.round().max(0.0) as u32,
                    y: top.round().max(0.0) as u32,
                    w: (x2.round() - x.round()).max(0.0) as u32,
                    h: (bottom.round() - top.round()).max(0.0) as u32,
                },
                paint,
                state,
            );
        }
        x = x2;
    }
}

fn blit_transition_rect(
    surface: &mut TransitionSurface<'_>,
    source: &[u8],
    rect: ScreenRect,
    paint: BandPaint,
    state: &TransitionState,
) {
    let start_x = rect.x.min(surface.width);
    let end_x = rect.x.saturating_add(rect.w).min(surface.width);
    let start_y = rect.y.min(surface.height);
    let end_y = rect.y.saturating_add(rect.h).min(surface.height);
    for y in start_y..end_y {
        for x in start_x..end_x {
            if is_stable_void_pixel(state, x, y) {
                continue;
            }
            let idx = (y as usize * surface.width as usize) + x as usize;
            surface.frame[idx] = match paint {
                BandPaint::Normal => source[idx],
                BandPaint::Invert => 255u8.saturating_sub(source[idx]),
            };
        }
    }
}

fn fill_rect(frame: &mut [u8], width: u32, height: u32, rect: ScreenRect, color: u8) {
    let start_x = rect.x.min(width);
    let end_x = rect.x.saturating_add(rect.w).min(width);
    let start_y = rect.y.min(height);
    let end_y = rect.y.saturating_add(rect.h).min(height);
    for y in start_y..end_y {
        let row_start = y as usize * width as usize;
        for x in start_x..end_x {
            frame[row_start + x as usize] = color;
        }
    }
}

fn y_for_sweep_position(
    sweep_position: f32,
    x: f32,
    board_width: f32,
    board_height: f32,
    board_left: f32,
    board_bottom: f32,
) -> f32 {
    board_bottom - board_height * (sweep_position - (x - board_left) / board_width)
}

fn interior_board_rect(
    viewport: &BoardViewport,
    board: &sokobanitron_gameplay::BoardView,
) -> ScreenRect {
    let left = viewport.origin_x + viewport.outer_margin_tiles as i32 * viewport.cell_size as i32;
    let top = viewport.origin_y + viewport.outer_margin_tiles as i32 * viewport.cell_size as i32;
    ScreenRect {
        x: left.max(0) as u32,
        y: top.max(0) as u32,
        w: board.width().saturating_mul(viewport.cell_size),
        h: board.height().saturating_mul(viewport.cell_size),
    }
}

// Keep stable void outside both board shapes from flashing during the sweep.
fn is_stable_void_pixel(state: &TransitionState, x: u32, y: u32) -> bool {
    is_void_in_scene(&state.old_scene, x, y) && is_void_in_scene(&state.new_scene, x, y)
}

fn is_void_in_scene(scene: &GameplayScreenRequest, x: u32, y: u32) -> bool {
    scene
        .viewport
        .screen_to_cell(f64::from(x), f64::from(y), &scene.board)
        .is_none_or(|cell| scene.board.tile(cell) == TileKind::Void)
}

fn flash_tiles(scene: &GameplayScreenRequest, union_board_rect: ScreenRect) -> Vec<FlashTile> {
    scene
        .board
        .cells()
        .map(|cell| {
            let rect = cell_rect(&scene.viewport, cell);
            let left = rect.x as f32;
            let top = rect.y as f32;
            let right = rect.x.saturating_add(rect.w) as f32;
            let bottom = rect.y.saturating_add(rect.h) as f32;
            let completion_s = sweep_position_for_point(left, top, union_board_rect)
                .max(sweep_position_for_point(right, top, union_board_rect))
                .max(sweep_position_for_point(left, bottom, union_board_rect))
                .max(sweep_position_for_point(right, bottom, union_board_rect));
            FlashTile {
                rect,
                completion_s,
                phase_index: 0,
            }
        })
        .collect()
}

fn cell_rect(viewport: &BoardViewport, cell: BoardCell) -> ScreenRect {
    let (x, y, w, h) = viewport.cell_to_screen_rect(cell);
    ScreenRect {
        x: x.max(0) as u32,
        y: y.max(0) as u32,
        w,
        h,
    }
}

fn sweep_position_for_point(x: f32, y: f32, board_rect: ScreenRect) -> f32 {
    let board_width = board_rect.w.max(1) as f32;
    let board_height = board_rect.h.max(1) as f32;
    let board_left = board_rect.x as f32;
    let board_bottom = board_rect.y.saturating_add(board_rect.h) as f32;
    (x - board_left) / board_width + (board_bottom - y) / board_height
}

fn union_screen_rect(a: ScreenRect, b: ScreenRect) -> ScreenRect {
    let left = a.x.min(b.x);
    let top = a.y.min(b.y);
    let right = a.x.saturating_add(a.w).max(b.x.saturating_add(b.w));
    let bottom = a.y.saturating_add(a.h).max(b.y.saturating_add(b.h));
    ScreenRect {
        x: left,
        y: top,
        w: right.saturating_sub(left),
        h: bottom.saturating_sub(top),
    }
}
