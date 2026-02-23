#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    pub h: usize,
    pub w: usize,
    pub cells: Vec<u8>,
}

impl Grid {
    pub fn from_lines(lines: Vec<String>) -> Self {
        let h = lines.len();
        let w = if h == 0 { 0 } else { lines[0].len() };
        let mut cells = Vec::with_capacity(h * w);
        for line in lines {
            cells.extend_from_slice(line.as_bytes());
        }
        Self { h, w, cells }
    }

    #[inline]
    pub fn idx(&self, r: usize, c: usize) -> usize {
        r * self.w + c
    }

    pub fn into_lines(self) -> Vec<String> {
        let mut out: Vec<String> = Vec::with_capacity(self.h);
        for r in 0..self.h {
            let start = r * self.w;
            let end = start + self.w;
            out.push(String::from_utf8(self.cells[start..end].to_vec()).expect("grid must contain valid ASCII"));
        }
        out
    }
}
