use crate::grid::validation::{assert_has_goal, assert_single_player, validate_characters};
use crate::grid::Grid;

pub(crate) fn build_grid_with_leading_and_rect(lines: Vec<String>) -> Grid {
    validate_characters(&lines);
    assert_single_player(&lines);
    assert_has_goal(&lines);

    let height = lines.len();
    if height == 0 {
        return Grid::from_shape_and_cells(0, 0, Vec::new());
    }

    let width = lines.iter().map(String::len).max().unwrap_or(0);

    let mut cells = vec![b'#'; height * width];
    for (r, line) in lines.into_iter().enumerate() {
        let bytes = line.into_bytes();
        let len = bytes.len();
        let row_start = r * width;

        if let Some(first_wall) = bytes.iter().position(|&b| b == b'#') {
            cells[row_start + first_wall..row_start + len].copy_from_slice(&bytes[first_wall..]);
        } else {
            cells[row_start..row_start + len].copy_from_slice(&bytes);
        }
    }

    Grid::from_shape_and_cells(width, height, cells)
}
