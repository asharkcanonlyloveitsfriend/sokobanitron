use super::controls::ScreenRect;
use sokobanitron_gameplay::{BoardCell, BoardView};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardViewport {
    pub origin_x: i32,
    pub origin_y: i32,
    pub cell_size: u32,
    pub board_pixel_width: u32,
    pub board_pixel_height: u32,
    pub outer_margin_tiles: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardViewportOptions {
    pub outer_margin_tiles: u32,
}

impl Default for BoardViewportOptions {
    fn default() -> Self {
        Self {
            outer_margin_tiles: 1,
        }
    }
}

pub(crate) fn board_cells_union_rect(
    viewport: &BoardViewport,
    cells: &[BoardCell],
    surface_width: u32,
    surface_height: u32,
) -> Option<ScreenRect> {
    let mut dirty = DamageRectUnion::default();
    for &cell in cells {
        let (x, y, w, h) = viewport.cell_to_screen_rect(cell);
        dirty.add_rect(x, y, w, h, surface_width, surface_height);
    }
    dirty.finish()
}

#[derive(Default)]
struct DamageRectUnion {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    found: bool,
}

impl DamageRectUnion {
    fn add_rect(
        &mut self,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        surface_width: u32,
        surface_height: u32,
    ) {
        if w == 0 || h == 0 || surface_width == 0 || surface_height == 0 {
            return;
        }
        let left = x.max(0) as u32;
        let top = y.max(0) as u32;
        let right = (x + w as i32).clamp(0, surface_width as i32) as u32;
        let bottom = (y + h as i32).clamp(0, surface_height as i32) as u32;
        if left >= right || top >= bottom {
            return;
        }
        if self.found {
            self.left = self.left.min(left);
            self.top = self.top.min(top);
            self.right = self.right.max(right);
            self.bottom = self.bottom.max(bottom);
        } else {
            self.left = left;
            self.top = top;
            self.right = right;
            self.bottom = bottom;
            self.found = true;
        }
    }

    fn finish(self) -> Option<ScreenRect> {
        self.found.then_some(ScreenRect {
            x: self.left,
            y: self.top,
            w: self.right - self.left,
            h: self.bottom - self.top,
        })
    }
}

impl BoardViewportOptions {
    pub fn fill_available_space() -> Self {
        Self {
            outer_margin_tiles: 0,
        }
    }
}

impl BoardViewport {
    pub fn fit_to_window(window_width: u32, window_height: u32, board: &BoardView) -> Self {
        Self::fit_to_window_with_options(
            window_width,
            window_height,
            board,
            BoardViewportOptions::default(),
        )
    }

    pub fn fit_to_window_with_options(
        window_width: u32,
        window_height: u32,
        board: &BoardView,
        options: BoardViewportOptions,
    ) -> Self {
        let margin = options.outer_margin_tiles;
        let cols = board
            .width()
            .saturating_add(margin.saturating_mul(2))
            .max(1);
        let rows = board
            .height()
            .saturating_add(margin.saturating_mul(2))
            .max(1);
        let max_cell_w = window_width / cols;
        let max_cell_h = window_height / rows;
        let cell_size = max_cell_w.min(max_cell_h).max(1);

        let board_pixel_width = cols * cell_size;
        let board_pixel_height = rows * cell_size;

        let origin_x = ((window_width as i32) - (board_pixel_width as i32)) / 2;
        let origin_y = ((window_height as i32) - (board_pixel_height as i32)) / 2;

        Self {
            origin_x,
            origin_y,
            cell_size,
            board_pixel_width,
            board_pixel_height,
            outer_margin_tiles: margin,
        }
    }

    pub fn cell_to_screen_rect(&self, cell: BoardCell) -> (i32, i32, u32, u32) {
        let px =
            self.origin_x + ((cell.x + self.outer_margin_tiles) as i32 * self.cell_size as i32);
        let py =
            self.origin_y + ((cell.y + self.outer_margin_tiles) as i32 * self.cell_size as i32);
        (px, py, self.cell_size, self.cell_size)
    }

    pub fn screen_to_cell(
        &self,
        screen_x: f64,
        screen_y: f64,
        board: &BoardView,
    ) -> Option<BoardCell> {
        let rel_x = screen_x - f64::from(self.origin_x);
        let rel_y = screen_y - f64::from(self.origin_y);
        if rel_x < 0.0 || rel_y < 0.0 {
            return None;
        }
        if rel_x >= f64::from(self.board_pixel_width) || rel_y >= f64::from(self.board_pixel_height)
        {
            return None;
        }

        let col = (rel_x / f64::from(self.cell_size)).floor() as i32;
        let row = (rel_y / f64::from(self.cell_size)).floor() as i32;
        let inner_x = col - self.outer_margin_tiles as i32;
        let inner_y = row - self.outer_margin_tiles as i32;
        if inner_x >= 0
            && inner_y >= 0
            && (inner_x as u32) < board.width()
            && (inner_y as u32) < board.height()
        {
            Some(BoardCell::new(inner_x as u32, inner_y as u32))
        } else {
            None
        }
    }
}
