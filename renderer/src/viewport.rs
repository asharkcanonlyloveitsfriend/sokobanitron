use sokobanitron_gameplay::BoardView;

#[derive(Debug, Clone, Copy)]
pub struct BoardViewport {
    pub origin_x: i32,
    pub origin_y: i32,
    pub cell_size: u32,
    pub board_pixel_width: u32,
    pub board_pixel_height: u32,
}

impl BoardViewport {
    pub fn fit_to_window(window_width: u32, window_height: u32, board: &BoardView) -> Self {
        let cols = board.width().saturating_add(2).max(1);
        let rows = board.height().saturating_add(2).max(1);
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
        }
    }

    pub fn cell_to_screen_rect(&self, x: u32, y: u32) -> (i32, i32, u32, u32) {
        let px = self.origin_x + ((x + 1) as i32 * self.cell_size as i32);
        let py = self.origin_y + ((y + 1) as i32 * self.cell_size as i32);
        (px, py, self.cell_size, self.cell_size)
    }

    pub fn screen_to_cell(
        &self,
        screen_x: f64,
        screen_y: f64,
        board: &BoardView,
    ) -> Option<(u32, u32)> {
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
        let inner_x = col - 1;
        let inner_y = row - 1;
        if inner_x >= 0
            && inner_y >= 0
            && (inner_x as u32) < board.width()
            && (inner_y as u32) < board.height()
        {
            Some((inner_x as u32, inner_y as u32))
        } else {
            None
        }
    }
}
