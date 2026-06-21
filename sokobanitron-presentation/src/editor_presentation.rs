use crate::layout::{
    ScreenRect, board_cells_union_rect, editor_bottom_left_button_rect,
    editor_bottom_right_button_rect, editor_mode_button_rect, editor_mode_menu_damage_rect,
    top_menu_toggle_button_visible_rect,
};
use crate::renderer::{FrameDamage, Renderer};
use crate::screen_requests::{
    EditorModeIndicator, EditorModeMenuScreenRequest, EditorScreenRequest,
};
use sokobanitron_gameplay::BoardCell;

#[derive(Default)]
pub struct EditorPresentationState {
    current_screen: Option<EditorScreenRequest>,
    draw_mode_board_dirty: bool,
    transient_overlay_rect: Option<ScreenRect>,
}

impl EditorPresentationState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.current_screen = None;
        self.draw_mode_board_dirty = false;
        self.transient_overlay_rect = None;
    }

    pub fn draw_screen(
        &mut self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        screen: &EditorScreenRequest,
    ) -> FrameDamage {
        let previous = self.current_screen.clone();
        match editor_damage(
            previous.as_ref(),
            screen,
            self.draw_mode_board_dirty,
            width,
            height,
        ) {
            EditorDamage::Full => {
                self.transient_overlay_rect = None;
                self.draw_full_screen(renderer, frame, width, height, screen)
            }
            EditorDamage::Cells(cells) => {
                let mut damage = FrameDamage::Noop;
                if let (Some(previous), Some(rect)) =
                    (previous.as_ref(), self.transient_overlay_rect.take())
                {
                    self.restore_editor_rect_with_chrome(
                        renderer, frame, width, height, previous, rect,
                    );
                    damage = damage.merge(FrameDamage::Region(rect));
                }
                let chrome_damage = editor_chrome_damage(previous.as_ref(), screen, width, height);
                if let Some(rect) = chrome_damage {
                    let restore_screen = chrome_restore_screen(previous.as_ref(), screen);
                    self.restore_editor_rect(renderer, frame, width, height, restore_screen, rect);
                    renderer.draw_editor_chrome_on_frame(frame, width, height, screen);
                    damage = damage.merge(FrameDamage::Region(rect));
                }
                if !cells.is_empty() {
                    if matches!(screen.mode_indicator, EditorModeIndicator::Draw)
                        && board_cells_changed(previous.as_ref(), screen, &cells)
                    {
                        self.draw_mode_board_dirty = true;
                    }
                    renderer.draw_editor_screen_cells(frame, width, height, screen, &cells);
                    damage = damage.merge(frame_damage_from_editor_cells(
                        screen, &cells, width, height,
                    ));
                }
                if !matches!(screen.mode_indicator, EditorModeIndicator::Draw) {
                    self.draw_mode_board_dirty = false;
                }
                self.current_screen = Some(screen.clone());
                damage
            }
        }
    }

    pub fn draw_full_screen(
        &mut self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        screen: &EditorScreenRequest,
    ) -> FrameDamage {
        renderer.draw_editor_screen(frame, width, height, screen);
        self.current_screen = Some(screen.clone());
        self.draw_mode_board_dirty = false;
        self.transient_overlay_rect = None;
        FrameDamage::Full
    }

    pub fn draw_mode_menu(
        &mut self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        screen: &EditorModeMenuScreenRequest,
    ) -> FrameDamage {
        renderer.draw_editor_mode_menu(frame, width, height, screen);
        self.current_screen = Some(screen.editor.clone());
        self.transient_overlay_rect = Some(editor_mode_menu_damage_rect(width, height));
        FrameDamage::Full
    }

    fn restore_editor_rect(
        &mut self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        screen: &EditorScreenRequest,
        rect: ScreenRect,
    ) {
        renderer.restore_background_rect(frame, width, height, rect);
        let cells = cells_intersecting_rect(screen, rect, width, height);
        renderer.draw_editor_screen_cells(frame, width, height, screen, &cells);
    }

    fn restore_editor_rect_with_chrome(
        &mut self,
        renderer: &mut Renderer,
        frame: &mut [u8],
        width: u32,
        height: u32,
        screen: &EditorScreenRequest,
        rect: ScreenRect,
    ) {
        self.restore_editor_rect(renderer, frame, width, height, screen, rect);
        renderer.draw_editor_overlays_on_frame(frame, width, height, screen);
        renderer.draw_editor_chrome_on_frame(frame, width, height, screen);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EditorDamage {
    Full,
    Cells(Vec<BoardCell>),
}

fn editor_damage(
    previous: Option<&EditorScreenRequest>,
    current: &EditorScreenRequest,
    draw_mode_board_dirty: bool,
    surface_width: u32,
    surface_height: u32,
) -> EditorDamage {
    let Some(previous) = previous else {
        return EditorDamage::Full;
    };
    if !editor_cell_damage_compatible(previous, current) {
        return EditorDamage::Full;
    }
    if previous.sleeping_player != current.sleeping_player {
        return EditorDamage::Full;
    }

    let mut dirty = Vec::new();
    for cell in current.board.cells() {
        if previous.board.tile(cell) != current.board.tile(cell)
            || previous.board.has_box(cell) != current.board.has_box(cell)
        {
            dirty.push(cell);
        }
    }
    add_optional_cell(&mut dirty, previous.board.player());
    add_optional_cell(&mut dirty, current.board.player());
    add_optional_cell(&mut dirty, previous.board.selected_box());
    add_optional_cell(&mut dirty, current.board.selected_box());
    if previous.warnings != current.warnings {
        dirty.extend(previous.warnings.iter().map(|warning| warning.cell));
        dirty.extend(current.warnings.iter().map(|warning| warning.cell));
    }
    let dirty = normalize_cells(dirty);
    if matches!(previous.mode_indicator, EditorModeIndicator::Draw)
        && !matches!(current.mode_indicator, EditorModeIndicator::Draw)
        && (draw_mode_board_dirty || !dirty.is_empty())
    {
        return EditorDamage::Full;
    }
    if dirty_cells_intersect_editor_chrome(current, &dirty, surface_width, surface_height) {
        return EditorDamage::Full;
    }
    EditorDamage::Cells(dirty)
}

fn editor_cell_damage_compatible(
    previous: &EditorScreenRequest,
    current: &EditorScreenRequest,
) -> bool {
    if previous.viewport != current.viewport {
        return false;
    }
    if previous.board.width() != current.board.width()
        || previous.board.height() != current.board.height()
    {
        return false;
    }
    previous.move_counts == current.move_counts && previous.puzzle_solved == current.puzzle_solved
}

fn frame_damage_from_editor_cells(
    screen: &EditorScreenRequest,
    cells: &[BoardCell],
    surface_width: u32,
    surface_height: u32,
) -> FrameDamage {
    board_cells_union_rect(&screen.viewport, cells, surface_width, surface_height)
        .map(FrameDamage::Region)
        .unwrap_or(FrameDamage::Noop)
}

fn add_optional_cell(cells: &mut Vec<BoardCell>, cell: Option<BoardCell>) {
    if let Some(cell) = cell {
        cells.push(cell);
    }
}

fn normalize_cells(mut cells: Vec<BoardCell>) -> Vec<BoardCell> {
    cells.sort_by_key(|cell| (cell.y, cell.x));
    cells.dedup();
    cells
}

fn board_cells_changed(
    previous: Option<&EditorScreenRequest>,
    current: &EditorScreenRequest,
    cells: &[BoardCell],
) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    cells.iter().any(|&cell| {
        previous.board.tile(cell) != current.board.tile(cell)
            || previous.board.has_box(cell) != current.board.has_box(cell)
    })
}

fn editor_chrome_damage(
    previous: Option<&EditorScreenRequest>,
    current: &EditorScreenRequest,
    surface_width: u32,
    surface_height: u32,
) -> Option<ScreenRect> {
    let previous = previous?;
    if previous.mode_indicator == current.mode_indicator
        && previous.can_zoom_out == current.can_zoom_out
        && previous.can_zoom_in == current.can_zoom_in
    {
        return None;
    }
    let mut damage = RectUnion::default();
    add_editor_chrome_rects(&mut damage, previous, surface_width, surface_height);
    add_editor_chrome_rects(&mut damage, current, surface_width, surface_height);
    damage.finish()
}

// Entering Draw mode should update only chrome; existing board cells keep their
// presentation-style pixels until they are changed by drawing.
fn chrome_restore_screen<'a>(
    previous: Option<&'a EditorScreenRequest>,
    current: &'a EditorScreenRequest,
) -> &'a EditorScreenRequest {
    let Some(previous) = previous else {
        return current;
    };
    if previous.mode_indicator == current.mode_indicator {
        current
    } else if matches!(current.mode_indicator, EditorModeIndicator::Draw) {
        previous
    } else {
        current
    }
}

fn add_editor_chrome_rects(
    damage: &mut RectUnion,
    screen: &EditorScreenRequest,
    surface_width: u32,
    surface_height: u32,
) {
    damage.add(editor_mode_button_rect(surface_width, surface_height));
    damage.add(top_menu_toggle_button_visible_rect(surface_width));
    if matches!(screen.mode_indicator, EditorModeIndicator::Draw) {
        if screen.can_zoom_out {
            damage.add(editor_bottom_left_button_rect(surface_height));
        }
        if screen.can_zoom_in {
            damage.add(editor_bottom_right_button_rect(
                surface_width,
                surface_height,
            ));
        }
    }
}

fn cells_intersecting_rect(
    screen: &EditorScreenRequest,
    rect: ScreenRect,
    surface_width: u32,
    surface_height: u32,
) -> Vec<BoardCell> {
    screen
        .board
        .cells()
        .filter(|&cell| {
            let (x, y, w, h) = screen.viewport.cell_to_screen_rect(cell);
            screen_rect_from_i32(x, y, w, h, surface_width, surface_height)
                .is_some_and(|cell_rect| screen_rects_intersect(cell_rect, rect))
        })
        .collect()
}

#[derive(Default)]
struct RectUnion {
    rect: Option<ScreenRect>,
}

impl RectUnion {
    fn add(&mut self, rect: ScreenRect) {
        self.rect = Some(match self.rect {
            Some(existing) => union_screen_rect(existing, rect),
            None => rect,
        });
    }

    fn finish(self) -> Option<ScreenRect> {
        self.rect
    }
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

fn dirty_cells_intersect_editor_chrome(
    screen: &EditorScreenRequest,
    cells: &[BoardCell],
    surface_width: u32,
    surface_height: u32,
) -> bool {
    if cells.is_empty() {
        return false;
    }
    let mut chrome_rects = vec![
        editor_mode_button_rect(surface_width, surface_height),
        top_menu_toggle_button_visible_rect(surface_width),
    ];
    if matches!(screen.mode_indicator, EditorModeIndicator::Draw) {
        if screen.can_zoom_out {
            chrome_rects.push(editor_bottom_left_button_rect(surface_height));
        }
        if screen.can_zoom_in {
            chrome_rects.push(editor_bottom_right_button_rect(
                surface_width,
                surface_height,
            ));
        }
    }

    cells.iter().any(|&cell| {
        let (x, y, w, h) = screen.viewport.cell_to_screen_rect(cell);
        let Some(cell_rect) = screen_rect_from_i32(x, y, w, h, surface_width, surface_height)
        else {
            return false;
        };
        chrome_rects
            .iter()
            .any(|&chrome_rect| screen_rects_intersect(cell_rect, chrome_rect))
    })
}

fn screen_rect_from_i32(
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    surface_width: u32,
    surface_height: u32,
) -> Option<ScreenRect> {
    if w == 0 || h == 0 || surface_width == 0 || surface_height == 0 {
        return None;
    }
    let left = x.max(0) as u32;
    let top = y.max(0) as u32;
    let right = (x + w as i32).clamp(0, surface_width as i32) as u32;
    let bottom = (y + h as i32).clamp(0, surface_height as i32) as u32;
    (left < right && top < bottom).then_some(ScreenRect {
        x: left,
        y: top,
        w: right - left,
        h: bottom - top,
    })
}

fn screen_rects_intersect(a: ScreenRect, b: ScreenRect) -> bool {
    let a_right = a.x.saturating_add(a.w);
    let a_bottom = a.y.saturating_add(a.h);
    let b_right = b.x.saturating_add(b.w);
    let b_bottom = b.y.saturating_add(b.h);
    a.x < b_right && b.x < a_right && a.y < b_bottom && b.y < a_bottom
}

#[cfg(test)]
mod tests {
    use super::EditorPresentationState;
    use crate::layout::{BoardViewport, ScreenRect, editor_mode_menu_damage_rect};
    use crate::renderer::{FrameDamage, Renderer};
    use crate::screen_requests::{
        EditorModeIndicator, EditorModeMenuScreenRequest, EditorScreenRequest,
    };
    use sokobanitron_gameplay::{BoardCell, BoardSolveState, BoardView, TileKind};

    fn board(tile_at_center: TileKind) -> BoardView {
        let mut tiles = vec![TileKind::Floor; 9];
        tiles[4] = tile_at_center;
        BoardView::new(
            3,
            3,
            tiles,
            vec![false; 9],
            None,
            None,
            BoardSolveState::Unsolved,
        )
    }

    fn single_cell_board(tile: TileKind) -> BoardView {
        BoardView::new(
            1,
            1,
            vec![tile],
            vec![false],
            None,
            None,
            BoardSolveState::Unsolved,
        )
    }

    fn request(board: BoardView, mode_indicator: EditorModeIndicator) -> EditorScreenRequest {
        request_with_viewport(
            board,
            mode_indicator,
            BoardViewport {
                origin_x: 20,
                origin_y: 90,
                cell_size: 10,
                board_pixel_width: 30,
                board_pixel_height: 30,
                outer_margin_tiles: 0,
            },
        )
    }

    fn request_with_viewport(
        board: BoardView,
        mode_indicator: EditorModeIndicator,
        viewport: BoardViewport,
    ) -> EditorScreenRequest {
        EditorScreenRequest {
            board,
            viewport,
            move_counts: Vec::new(),
            warnings: Vec::new(),
            mode_indicator,
            puzzle_solved: false,
            can_zoom_out: false,
            can_zoom_in: false,
            sleeping_player: false,
        }
    }

    fn cell_pixels(
        frame: &[u8],
        frame_width: u32,
        viewport: BoardViewport,
        cell: BoardCell,
    ) -> Vec<u8> {
        let (x, y, w, h) = viewport.cell_to_screen_rect(cell);
        let mut pixels = Vec::new();
        for row in y as u32..(y as u32 + h) {
            let start = (row * frame_width + x as u32) as usize;
            let end = start + w as usize;
            pixels.extend_from_slice(&frame[start..end]);
        }
        pixels
    }

    fn assert_partial_damage_invariant(
        previous_frame: &[u8],
        partial_frame: &[u8],
        full_current_frame: &[u8],
        frame_width: u32,
        damage: FrameDamage,
    ) {
        let FrameDamage::Region(rect) = damage else {
            panic!("expected regional damage, got {damage:?}");
        };
        assert_eq!(previous_frame.len(), partial_frame.len());
        assert_eq!(partial_frame.len(), full_current_frame.len());
        for index in 0..partial_frame.len() {
            let x = index as u32 % frame_width;
            let y = index as u32 / frame_width;
            let inside_damage = x >= rect.x
                && x < rect.x.saturating_add(rect.w)
                && y >= rect.y
                && y < rect.y.saturating_add(rect.h);
            if inside_damage {
                assert_eq!(partial_frame[index], full_current_frame[index]);
            } else {
                assert_eq!(partial_frame[index], previous_frame[index]);
            }
        }
    }

    #[test]
    fn draw_mode_tile_change_redraws_only_changed_cell() {
        let initial = request(board(TileKind::Floor), EditorModeIndicator::Draw);
        let current = request(board(TileKind::Goal), EditorModeIndicator::Draw);
        let mut state = EditorPresentationState::new();
        let mut partial_renderer = Renderer::new();
        let mut full_renderer = Renderer::new();
        let mut partial_frame = vec![0; 160 * 160];
        let mut full_frame = vec![0; 160 * 160];

        assert_eq!(
            state.draw_screen(
                &mut partial_renderer,
                &mut partial_frame,
                160,
                160,
                &initial
            ),
            FrameDamage::Full
        );
        let damage = state.draw_screen(
            &mut partial_renderer,
            &mut partial_frame,
            160,
            160,
            &current,
        );
        full_renderer.draw_editor_screen(&mut full_frame, 160, 160, &current);

        assert_eq!(
            damage,
            FrameDamage::Region(ScreenRect {
                x: 30,
                y: 100,
                w: 10,
                h: 10,
            })
        );
        assert_eq!(partial_frame, full_frame);
    }

    #[test]
    fn partial_update_matches_full_render_only_inside_reported_damage() {
        let initial = request(board(TileKind::Floor), EditorModeIndicator::Draw);
        let current = request(board(TileKind::Goal), EditorModeIndicator::Draw);
        let mut state = EditorPresentationState::new();
        let mut partial_renderer = Renderer::new();
        let mut full_renderer = Renderer::new();
        let mut partial_frame = vec![0; 160 * 160];
        let mut full_current_frame = vec![0; 160 * 160];

        let _ = state.draw_screen(
            &mut partial_renderer,
            &mut partial_frame,
            160,
            160,
            &initial,
        );
        let previous_frame = partial_frame.clone();
        let damage = state.draw_screen(
            &mut partial_renderer,
            &mut partial_frame,
            160,
            160,
            &current,
        );
        full_renderer.draw_editor_screen(&mut full_current_frame, 160, 160, &current);

        assert_partial_damage_invariant(
            &previous_frame,
            &partial_frame,
            &full_current_frame,
            160,
            damage,
        );
    }

    #[test]
    fn entering_draw_mode_does_not_redraw_existing_board_cells() {
        let viewport = BoardViewport {
            origin_x: 60,
            origin_y: 110,
            cell_size: 20,
            board_pixel_width: 20,
            board_pixel_height: 20,
            outer_margin_tiles: 0,
        };
        let initial = request_with_viewport(
            single_cell_board(TileKind::Floor),
            EditorModeIndicator::Move,
            viewport,
        );
        let current = request_with_viewport(
            single_cell_board(TileKind::Floor),
            EditorModeIndicator::Draw,
            viewport,
        );
        let mut state = EditorPresentationState::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 160 * 160];

        let _ = state.draw_screen(&mut renderer, &mut frame, 160, 160, &initial);
        let before = cell_pixels(&frame, 160, viewport, BoardCell::new(0, 0));
        let damage = state.draw_screen(&mut renderer, &mut frame, 160, 160, &current);
        let after = cell_pixels(&frame, 160, viewport, BoardCell::new(0, 0));

        assert_ne!(damage, FrameDamage::Full);
        assert_eq!(before, after);
    }

    #[test]
    fn entering_draw_mode_after_mode_menu_does_not_force_full_redraw() {
        let initial = request(board(TileKind::Floor), EditorModeIndicator::Move);
        let current = request(board(TileKind::Floor), EditorModeIndicator::Draw);
        let menu = EditorModeMenuScreenRequest {
            editor: initial.clone(),
            can_enter_play: true,
        };
        let mut state = EditorPresentationState::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 160 * 160];

        let _ = state.draw_screen(&mut renderer, &mut frame, 160, 160, &initial);
        let _ = state.draw_mode_menu(&mut renderer, &mut frame, 160, 160, &menu);

        assert_ne!(
            state.draw_screen(&mut renderer, &mut frame, 160, 160, &current),
            FrameDamage::Full
        );
    }

    #[test]
    fn closing_mode_menu_on_current_mode_restores_attached_area_and_chrome() {
        let current = request(board(TileKind::Floor), EditorModeIndicator::Draw);
        let menu = EditorModeMenuScreenRequest {
            editor: current.clone(),
            can_enter_play: true,
        };
        let mut state = EditorPresentationState::new();
        let mut partial_renderer = Renderer::new();
        let mut full_renderer = Renderer::new();
        let mut partial_frame = vec![0; 256 * 500];
        let mut full_current_frame = vec![0; 256 * 500];

        let _ = state.draw_screen(
            &mut partial_renderer,
            &mut partial_frame,
            256,
            500,
            &current,
        );
        let _ = state.draw_mode_menu(&mut partial_renderer, &mut partial_frame, 256, 500, &menu);
        let menu_frame = partial_frame.clone();
        let damage = state.draw_screen(
            &mut partial_renderer,
            &mut partial_frame,
            256,
            500,
            &current,
        );
        full_renderer.draw_editor_screen(&mut full_current_frame, 256, 500, &current);

        assert_eq!(
            damage,
            FrameDamage::Region(editor_mode_menu_damage_rect(256, 500))
        );
        assert_partial_damage_invariant(
            &menu_frame,
            &partial_frame,
            &full_current_frame,
            256,
            damage,
        );
    }

    #[test]
    fn draw_mode_menu_does_not_dirty_board_for_later_mode_exit() {
        let initial = request(board(TileKind::Floor), EditorModeIndicator::Move);
        let draw = request(board(TileKind::Floor), EditorModeIndicator::Draw);
        let current = request(board(TileKind::Floor), EditorModeIndicator::Move);
        let menu = EditorModeMenuScreenRequest {
            editor: draw.clone(),
            can_enter_play: true,
        };
        let mut state = EditorPresentationState::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 160 * 160];

        let _ = state.draw_screen(&mut renderer, &mut frame, 160, 160, &initial);
        let _ = state.draw_screen(&mut renderer, &mut frame, 160, 160, &draw);
        let _ = state.draw_mode_menu(&mut renderer, &mut frame, 160, 160, &menu);

        assert_ne!(
            state.draw_screen(&mut renderer, &mut frame, 160, 160, &current),
            FrameDamage::Full
        );
    }

    #[test]
    fn full_draw_in_draw_mode_does_not_dirty_board_for_later_mode_exit() {
        let draw = request(board(TileKind::Floor), EditorModeIndicator::Draw);
        let current = request(board(TileKind::Floor), EditorModeIndicator::Move);
        let mut state = EditorPresentationState::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 160 * 160];

        let _ = state.draw_full_screen(&mut renderer, &mut frame, 160, 160, &draw);

        assert_ne!(
            state.draw_screen(&mut renderer, &mut frame, 160, 160, &current),
            FrameDamage::Full
        );
    }

    #[test]
    fn leaving_draw_mode_without_board_changes_does_not_force_full_redraw() {
        let initial = request(board(TileKind::Goal), EditorModeIndicator::Move);
        let draw = request(board(TileKind::Goal), EditorModeIndicator::Draw);
        let current = request(board(TileKind::Goal), EditorModeIndicator::Move);
        let mut state = EditorPresentationState::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 160 * 160];

        let _ = state.draw_screen(&mut renderer, &mut frame, 160, 160, &initial);
        let _ = state.draw_screen(&mut renderer, &mut frame, 160, 160, &draw);

        assert_eq!(
            state.draw_screen(&mut renderer, &mut frame, 160, 160, &current),
            FrameDamage::Region(ScreenRect {
                x: 16,
                y: 16,
                w: 136,
                h: 76,
            })
        );
    }

    #[test]
    fn leaving_draw_mode_after_board_change_forces_full_editor_redraw() {
        let initial = request(board(TileKind::Floor), EditorModeIndicator::Draw);
        let changed = request(board(TileKind::Goal), EditorModeIndicator::Draw);
        let current = request(board(TileKind::Goal), EditorModeIndicator::Move);
        let mut state = EditorPresentationState::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 160 * 160];

        let _ = state.draw_screen(&mut renderer, &mut frame, 160, 160, &initial);
        let _ = state.draw_screen(&mut renderer, &mut frame, 160, 160, &changed);

        assert_eq!(
            state.draw_screen(&mut renderer, &mut frame, 160, 160, &current),
            FrameDamage::Full
        );
    }

    #[test]
    fn draw_mode_tile_change_under_chrome_falls_back_to_full_redraw() {
        let viewport = BoardViewport {
            origin_x: 16,
            origin_y: 16,
            cell_size: 10,
            board_pixel_width: 30,
            board_pixel_height: 30,
            outer_margin_tiles: 0,
        };
        let initial =
            request_with_viewport(board(TileKind::Floor), EditorModeIndicator::Draw, viewport);
        let current =
            request_with_viewport(board(TileKind::Goal), EditorModeIndicator::Draw, viewport);
        let mut state = EditorPresentationState::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 160 * 160];

        let _ = state.draw_screen(&mut renderer, &mut frame, 160, 160, &initial);

        assert_eq!(
            state.draw_screen(&mut renderer, &mut frame, 160, 160, &current),
            FrameDamage::Full
        );
    }

    #[test]
    fn moved_player_redraws_old_and_new_cells_in_draw_mode() {
        let initial_board = BoardView::new(
            3,
            3,
            vec![TileKind::Floor; 9],
            vec![false; 9],
            Some(BoardCell::new(0, 0)),
            None,
            BoardSolveState::Unsolved,
        );
        let current_board = BoardView::new(
            3,
            3,
            vec![TileKind::Floor; 9],
            vec![false; 9],
            Some(BoardCell::new(2, 2)),
            None,
            BoardSolveState::Unsolved,
        );
        let viewport = BoardViewport {
            origin_x: 20,
            origin_y: 100,
            cell_size: 10,
            board_pixel_width: 30,
            board_pixel_height: 30,
            outer_margin_tiles: 0,
        };
        let initial = request_with_viewport(initial_board, EditorModeIndicator::Draw, viewport);
        let current = request_with_viewport(current_board, EditorModeIndicator::Draw, viewport);
        let mut state = EditorPresentationState::new();
        let mut renderer = Renderer::new();
        let mut frame = vec![0; 160 * 160];

        let _ = state.draw_screen(&mut renderer, &mut frame, 160, 160, &initial);

        assert_eq!(
            state.draw_screen(&mut renderer, &mut frame, 160, 160, &current),
            FrameDamage::Region(ScreenRect {
                x: 20,
                y: 100,
                w: 30,
                h: 30,
            })
        );
    }
}
