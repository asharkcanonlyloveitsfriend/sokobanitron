#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Grid {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) cells: Vec<u8>,
}

impl Grid {
    pub(crate) fn from_lines(lines: Vec<String>) -> Self {
        let h = lines.len();
        let w = if h == 0 { 0 } else { lines[0].len() };
        let mut cells = Vec::with_capacity(h * w);
        for line in lines {
            cells.extend_from_slice(line.as_bytes());
        }
        let grid = Self {
            height: h,
            width: w,
            cells,
        };
        grid.debug_assert_invariants();
        grid
    }

    #[inline]
    pub(crate) fn width(&self) -> usize {
        self.width
    }

    #[inline]
    pub(crate) fn height(&self) -> usize {
        self.height
    }

    #[inline]
    pub(crate) fn cells(&self) -> &[u8] {
        &self.cells
    }

    #[inline]
    pub(crate) fn cells_mut(&mut self) -> &mut [u8] {
        &mut self.cells
    }

    #[inline]
    pub(crate) fn idx(&self, r: usize, c: usize) -> usize {
        r * self.width + c
    }

    pub(crate) fn set_shape_and_cells(&mut self, width: usize, height: usize, cells: Vec<u8>) {
        self.width = width;
        self.height = height;
        self.cells = cells;
        self.debug_assert_invariants();
    }

    pub(crate) fn from_shape_and_cells(width: usize, height: usize, cells: Vec<u8>) -> Self {
        let grid = Self {
            width,
            height,
            cells,
        };
        grid.debug_assert_invariants();
        grid
    }

    pub(crate) fn into_lines(self) -> Vec<String> {
        let mut out: Vec<String> = Vec::with_capacity(self.height);
        for r in 0..self.height {
            let start = r * self.width;
            let end = start + self.width;
            out.push(
                String::from_utf8(self.cells[start..end].to_vec())
                    .expect("grid must contain valid ASCII"),
            );
        }
        out
    }
}
