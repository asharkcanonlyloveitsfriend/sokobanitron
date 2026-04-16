use crate::layout::{BoardViewport, ScreenRect};
use sokobanitron_gameplay::{BoardCell, BoardView, TileKind};

use super::{Renderer, WHITE, pixels::fill_rect};

impl Renderer {
    pub(crate) fn draw_floor_tile_cell(
        &mut self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
        cell: BoardCell,
    ) {
        let (rect_x, rect_y, rect_w, rect_h) = viewport.cell_to_screen_rect(cell);
        let (fill, stroke) = match board.tile(cell) {
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
            TileKind::Floor => (WHITE, self.theme.light_1),
            TileKind::Goal => (self.theme.light_2, WHITE),
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
        );
    }

    pub(crate) fn draw_floor_tiles(
        &self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
    ) {
        for cell in board.cells() {
            let tile = board.tile(cell);
            if tile == TileKind::Void {
                continue;
            }
            let (rect_x, rect_y, rect_w, rect_h) = viewport.cell_to_screen_rect(cell);
            let (fill, stroke) = match tile {
                TileKind::Floor => (WHITE, self.theme.light_1),
                TileKind::Goal => (self.theme.light_2, WHITE),
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
    color: [u8; 4],
) {
    if w == 0 || h == 0 {
        return;
    }
    let left_is_void =
        tile.x == 0 || board.tile(BoardCell::new(tile.x - 1, tile.y)) == TileKind::Void;
    let top_is_void =
        tile.y == 0 || board.tile(BoardCell::new(tile.x, tile.y - 1)) == TileKind::Void;

    if left_is_void {
        fill_rect(frame, frame_width, frame_height, x, y, 1, h, color);
    }
    if top_is_void {
        fill_rect(frame, frame_width, frame_height, x, y, w, 1, color);
    }
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x + w as i32 - 1,
        y,
        1,
        h,
        color,
    );
    fill_rect(
        frame,
        frame_width,
        frame_height,
        x,
        y + h as i32 - 1,
        w,
        1,
        color,
    );
}
