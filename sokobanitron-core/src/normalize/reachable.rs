use crate::grid::Grid;

pub(crate) fn mask_to_player_reachable_in_place(grid: &mut Grid) {
    let height = grid.height();
    let width = grid.width();

    let cells = grid.cells_mut();

    // Find the first player tile in row-major order: '@' or '+'.
    let player_index = cells
        .iter()
        .position(|&ch| ch == b'@' || ch == b'+')
        .expect("grid invariant violated: player not found");

    // Flood fill reachable tiles where tile != '#'.
    // Mark visited tiles in-place using the high bit (grid chars are ASCII).
    const VISITED: u8 = 0x80;
    #[inline]
    fn try_push(cells: &mut [u8], stack: &mut Vec<usize>, neighbor_index: usize, visited: u8) {
        let neighbor = cells[neighbor_index];
        if (neighbor & visited) == 0 && neighbor != b'#' {
            cells[neighbor_index] = neighbor | visited;
            stack.push(neighbor_index);
        }
    }
    let mut stack: Vec<usize> = Vec::with_capacity(16); // small DFS frontier
    cells[player_index] |= VISITED;
    stack.push(player_index);

    let grid_len = width * height;

    while let Some(cell_index) = stack.pop() {
        let row_start = cell_index - (cell_index % width);
        let row_end = row_start + width;

        // up
        if cell_index >= width {
            let neighbor_index = cell_index - width;
            try_push(cells, &mut stack, neighbor_index, VISITED);
        }

        // down
        if cell_index + width < grid_len {
            let neighbor_index = cell_index + width;
            try_push(cells, &mut stack, neighbor_index, VISITED);
        }

        // left
        if cell_index > row_start {
            let neighbor_index = cell_index - 1;
            try_push(cells, &mut stack, neighbor_index, VISITED);
        }

        // right
        if cell_index + 1 < row_end {
            let neighbor_index = cell_index + 1;
            try_push(cells, &mut stack, neighbor_index, VISITED);
        }
    }

    // Mask unreachable non-walls to '#', and clear visited marker from reachable tiles.
    for ch in cells.iter_mut() {
        let current = *ch;
        if (current & VISITED) != 0 {
            *ch = current & !VISITED;
        } else if current != b'#' {
            *ch = b'#';
        }
    }

    grid.debug_assert_invariants();
}

pub fn mask_to_player_reachable(lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let mut grid = Grid::from_lines(lines);
    mask_to_player_reachable_in_place(&mut grid);
    grid.into_lines()
}

#[cfg(test)]
mod tests {
    use super::mask_to_player_reachable;

    fn lines(grid: &str) -> Vec<String> {
        grid.trim_matches('\n')
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect()
    }

    #[test]
    fn masks_disconnected_interior_island() {
        let grid = "
#######
#.@   #
# ### #
# #*# #
# ### #
#   $ #
#######
";

        let result = mask_to_player_reachable(lines(grid));

        assert_eq!(
            result,
            vec![
                "#######".to_string(),
                "#.@   #".to_string(),
                "# ### #".to_string(),
                "# ### #".to_string(),
                "# ### #".to_string(),
                "#   $ #".to_string(),
                "#######".to_string(),
            ]
        );
    }
}
