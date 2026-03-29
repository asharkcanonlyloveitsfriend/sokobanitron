use sokobanitron_core::normalize_to_walkable_region_lines;
use sokobanitron_gameplay::OrientationPolicy;

pub(crate) fn normalize_and_orient_level(
    ascii: &str,
    orientation_policy: OrientationPolicy,
) -> String {
    let cleaned = ascii
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect::<Vec<_>>();
    let mut lines = normalize_to_walkable_region_lines(cleaned);
    if orientation_policy == OrientationPolicy::RotateWideToPortrait && is_wider_than_tall(&lines) {
        lines = rotate_clockwise_lines(&lines);
    }
    lines.join("\n")
}

fn is_wider_than_tall(lines: &[String]) -> bool {
    let height = lines.len();
    if height == 0 {
        return false;
    }
    let width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    width > height
}

fn rotate_clockwise_lines(lines: &[String]) -> Vec<String> {
    let height = lines.len();
    if height == 0 {
        return Vec::new();
    }
    let width = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    if width == 0 {
        return Vec::new();
    }

    let mut grid = vec![vec![' '; width]; height];
    for (row_index, line) in lines.iter().enumerate() {
        for (col_index, ch) in line.chars().enumerate() {
            grid[row_index][col_index] = ch;
        }
    }

    let mut rotated = vec![String::with_capacity(height); width];
    for col in 0..width {
        let mut row = String::with_capacity(height);
        for src_row in (0..height).rev() {
            row.push(grid[src_row][col]);
        }
        rotated[col] = row;
    }
    rotated
}
