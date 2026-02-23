use crate::grid::Grid;

pub(crate) fn mask_to_player_reachable_in_place(grid: &mut Grid) {
    if grid.h == 0 || grid.w == 0 {
        return;
    }

    // Find the first player tile in row-major order: '@' or '+'.
    let mut start: Option<usize> = None;
    for (i, &ch) in grid.cells.iter().enumerate() {
        if ch == b'@' || ch == b'+' {
            start = Some(i);
            break;
        }
    }

    // If no player found, match Python behavior by returning unchanged.
    let Some(start) = start else {
        return;
    };

    // Flood fill reachable tiles where tile != '#'.
    // Mark visited tiles in-place using the high bit (grid chars are ASCII).
    const VISITED: u8 = 0x80;
    let mut stack: Vec<usize> = Vec::with_capacity(16);
    grid.cells[start] |= VISITED;
    stack.push(start);

    while let Some(i) = stack.pop() {
        let r = i / grid.w;
        let c = i - (r * grid.w);

        // up
        if r > 0 {
            let ni = i - grid.w;
            let ch = grid.cells[ni];
            if (ch & VISITED) == 0 && ch != b'#' {
                grid.cells[ni] = ch | VISITED;
                stack.push(ni);
            }
        }
        // down
        if r + 1 < grid.h {
            let ni = i + grid.w;
            let ch = grid.cells[ni];
            if (ch & VISITED) == 0 && ch != b'#' {
                grid.cells[ni] = ch | VISITED;
                stack.push(ni);
            }
        }
        // left
        if c > 0 {
            let ni = i - 1;
            let ch = grid.cells[ni];
            if (ch & VISITED) == 0 && ch != b'#' {
                grid.cells[ni] = ch | VISITED;
                stack.push(ni);
            }
        }
        // right
        if c + 1 < grid.w {
            let ni = i + 1;
            let ch = grid.cells[ni];
            if (ch & VISITED) == 0 && ch != b'#' {
                grid.cells[ni] = ch | VISITED;
                stack.push(ni);
            }
        }
    }

    // Mask unreachable non-walls to '#', and clear visited marker from reachable tiles.
    for i in 0..(grid.h * grid.w) {
        let ch = grid.cells[i];
        if (ch & VISITED) != 0 {
            grid.cells[i] = ch & !VISITED;
        } else if ch != b'#' {
            grid.cells[i] = b'#';
        }
    }
}

pub fn mask_to_player_reachable(lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let mut grid = Grid::from_lines(lines);
    mask_to_player_reachable_in_place(&mut grid);
    grid.into_lines()
}
