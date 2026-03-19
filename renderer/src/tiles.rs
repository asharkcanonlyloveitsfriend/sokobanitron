use crate::{BoardViewport, Renderer, pixels::fill_rect};
use sokobanitron_gameplay::{BoardView, TileKind};

impl Renderer {
    pub(crate) fn draw_floor_tiles(
        &self,
        frame: &mut [u8],
        frame_width: u32,
        frame_height: u32,
        board: &BoardView,
        viewport: &BoardViewport,
    ) {
        for y in 0..board.height() {
            for x in 0..board.width() {
                let tile = board.tile(x, y);
                if tile == TileKind::Void {
                    continue;
                }
                let (rect_x, rect_y, rect_w, rect_h) = viewport.cell_to_screen_rect(x, y);
                let (fill, stroke) = match tile {
                    TileKind::Floor => (self.theme.floor_fill, self.theme.floor_stroke),
                    TileKind::Goal => (self.theme.target_fill, [255, 255, 255, 255]),
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
                    x,
                    y,
                    rect_x,
                    rect_y,
                    rect_w,
                    rect_h,
                    stroke,
                );
            }
        }
    }
}

fn draw_tile_edges_once(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    board: &BoardView,
    tile_x: u32,
    tile_y: u32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: [u8; 4],
) {
    if w == 0 || h == 0 {
        return;
    }
    let left_is_void = tile_x == 0 || board.tile(tile_x - 1, tile_y) == TileKind::Void;
    let top_is_void = tile_y == 0 || board.tile(tile_x, tile_y - 1) == TileKind::Void;

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
