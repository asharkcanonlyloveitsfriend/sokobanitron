#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelCell {
    Void,
    Floor,
}

#[derive(Debug, Clone)]
pub struct ParsedLevel {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<LevelCell>,
    pub goals: Vec<bool>,
}

impl ParsedLevel {
    pub fn cell(&self, x: u32, y: u32) -> LevelCell {
        self.cells[(y * self.width + x) as usize]
    }

    pub fn is_goal(&self, x: u32, y: u32) -> bool {
        self.goals[(y * self.width + x) as usize]
    }
}

pub fn parse_level_ascii(ascii: &str) -> ParsedLevel {
    let lines = ascii.lines().collect::<Vec<_>>();
    let height = lines.len() as u32;
    let width = lines
        .iter()
        .map(|line| line.chars().count() as u32)
        .max()
        .unwrap_or(0);

    let mut cells = vec![LevelCell::Void; (width * height) as usize];
    let mut goals = vec![false; (width * height) as usize];

    for (y, line) in lines.iter().enumerate() {
        for (x, ch) in line.chars().enumerate() {
            let idx = y * (width as usize) + x;
            cells[idx] = if ch == '#' {
                LevelCell::Void
            } else {
                LevelCell::Floor
            };
            goals[idx] = matches!(ch, '.' | '+' | '*');
        }
    }

    ParsedLevel {
        width,
        height,
        cells,
        goals,
    }
}
