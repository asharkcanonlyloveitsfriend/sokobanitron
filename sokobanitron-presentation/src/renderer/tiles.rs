use crate::layout::{BoardViewport, ScreenRect};
use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};

use super::{Renderer, TileBorderPolicy, pixels::fill_rect};

impl Renderer {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_floor_tile_cell(
        &mut self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        cell: BoardCell,
        border_policy: TileBorderPolicy,
    ) {
        let (rect_x, rect_y, rect_w, rect_h) = viewport.cell_to_screen_rect(cell);
        let (fill, stroke, stroke_width) = match board.tile(cell) {
            // Pass-one gameplay partial redraw must match the full gameplay base layer exactly:
            // void cells reveal the background, while floor/goal cells replace it.
            TileKind::Void => {
                self.restore_background_rect(
                    frame,
                    frame_width,
                    frame_height,
                    ScreenRect {
                        x: rect_x.max(0) as u32,
                        y: rect_y.max(0) as u32,
                        w: rect_w,
                        h: rect_h,
                    },
                );
                return;
            }
            TileKind::Floor => (self.theme.white, self.theme.gray_1, 1),
            TileKind::Goal => (self.theme.gray_2, self.theme.white, 1),
        };
        fill_rect(
            frame,
            frame_width,
            frame_height,
            rect_x,
            rect_y,
            rect_w,
            rect_h,
            fill,
        );
        draw_tile_edges_once(
            frame,
            frame_width,
            frame_height,
            board,
            cell,
            rect_x,
            rect_y,
            rect_w,
            rect_h,
            stroke,
            stroke_width,
            border_policy,
        );
    }

    pub(crate) fn draw_floor_tiles(
        &self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        border_policy: TileBorderPolicy,
    ) {
        for cell in board.cells() {
            let tile = board.tile(cell);
            if tile == TileKind::Void {
                continue;
            }
            let (rect_x, rect_y, rect_w, rect_h) = viewport.cell_to_screen_rect(cell);
            let (fill, stroke, stroke_width) = match tile {
                TileKind::Floor => (self.theme.white, self.theme.gray_1, 1),
                TileKind::Goal => (self.theme.gray_2, self.theme.white, 1),
                TileKind::Void => continue,
            };
            fill_rect(
                frame,
                frame_width,
                frame_height,
                rect_x,
                rect_y,
                rect_w,
                rect_h,
                fill,
            );
            draw_tile_edges_once(
                frame,
                frame_width,
                frame_height,
                board,
                cell,
                rect_x,
                rect_y,
                rect_w,
                rect_h,
                stroke,
                stroke_width,
                border_policy,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_tile_edges_once(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    board: &BoardView,
    tile: BoardCell,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: u8,
    stroke_width: u32,
    border_policy: TileBorderPolicy,
) {
    if w == 0 || h == 0 {
        return;
    }
    let stroke_width = stroke_width.max(1).min(w).min(h);
    if should_draw_tile_edge(board, tile, TileEdge::Left, border_policy) {
        fill_rect(
            frame,
            frame_width,
            frame_height,
            x,
            y,
            stroke_width,
            h,
            color,
        );
    }
    if should_draw_tile_edge(board, tile, TileEdge::Top, border_policy) {
        fill_rect(
            frame,
            frame_width,
            frame_height,
            x,
            y,
            w,
            stroke_width,
            color,
        );
    }
    if should_draw_tile_edge(board, tile, TileEdge::Right, border_policy) {
        fill_rect(
            frame,
            frame_width,
            frame_height,
            x + w as i32 - stroke_width as i32,
            y,
            stroke_width,
            h,
            color,
        );
    }
    if should_draw_tile_edge(board, tile, TileEdge::Bottom, border_policy) {
        fill_rect(
            frame,
            frame_width,
            frame_height,
            x,
            y + h as i32 - stroke_width as i32,
            w,
            stroke_width,
            color,
        );
    }
}

#[derive(Clone, Copy)]
enum TileEdge {
    Left,
    Top,
    Right,
    Bottom,
}

fn should_draw_tile_edge(
    board: &BoardView,
    tile: BoardCell,
    edge: TileEdge,
    border_policy: TileBorderPolicy,
) -> bool {
    let current = board.tile(tile);
    if current == TileKind::Void {
        return false;
    }
    if matches!(border_policy, TileBorderPolicy::EditorDraw) {
        return true;
    }

    let Some(neighbor) = neighbor_cell(board, tile, edge) else {
        return false;
    };
    board.tile(neighbor) == current
}

fn neighbor_cell(board: &BoardView, tile: BoardCell, edge: TileEdge) -> Option<BoardCell> {
    match edge {
        TileEdge::Left => (tile.x > 0).then(|| BoardCell::new(tile.x - 1, tile.y)),
        TileEdge::Top => (tile.y > 0).then(|| BoardCell::new(tile.x, tile.y - 1)),
        TileEdge::Right => {
            let x = tile.x.checked_add(1)?;
            (x < board.width()).then(|| BoardCell::new(x, tile.y))
        }
        TileEdge::Bottom => {
            let y = tile.y.checked_add(1)?;
            (y < board.height()).then(|| BoardCell::new(tile.x, y))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Renderer, TileBorderPolicy};
    use crate::layout::BoardViewport;
    use sokobanitron_gameplay::{BoardSolveState, BoardView, TileKind};

    fn board(tiles: Vec<TileKind>) -> BoardView {
        let len = tiles.len();
        BoardView::new(
            tiles.len() as u32,
            1,
            tiles,
            vec![false; len],
            None,
            None,
            BoardSolveState::Unsolved,
        )
    }

    fn viewport(width: u32) -> BoardViewport {
        BoardViewport {
            origin_x: 0,
            origin_y: 0,
            cell_size: 4,
            board_pixel_width: width * 4,
            board_pixel_height: 4,
            outer_margin_tiles: 0,
        }
    }

    #[test]
    fn presentation_borders_only_between_matching_non_void_tiles() {
        let board = board(vec![TileKind::Floor, TileKind::Floor, TileKind::Goal]);
        let renderer = Renderer::new();
        let mut frame = vec![0; 12 * 4];

        renderer.draw_floor_tiles(
            &mut frame,
            12,
            4,
            &board,
            &viewport(3),
            TileBorderPolicy::Presentation,
        );

        assert_eq!(frame[0], renderer.theme.white);
        assert_eq!(frame[3], renderer.theme.gray_1);
        assert_eq!(frame[4], renderer.theme.gray_1);
        assert_eq!(frame[7], renderer.theme.white);
        assert_eq!(frame[8], renderer.theme.gray_2);
    }

    #[test]
    fn editor_draw_borders_all_non_void_tile_edges() {
        let board = board(vec![TileKind::Floor, TileKind::Goal]);
        let renderer = Renderer::new();
        let mut frame = vec![0; 8 * 4];

        renderer.draw_floor_tiles(
            &mut frame,
            8,
            4,
            &board,
            &viewport(2),
            TileBorderPolicy::EditorDraw,
        );

        assert_eq!(frame[8], renderer.theme.gray_1);
        assert_eq!(frame[11], renderer.theme.gray_1);
        assert_eq!(frame[12], renderer.theme.white);
        assert_eq!(frame[13], renderer.theme.gray_2);
    }
}
