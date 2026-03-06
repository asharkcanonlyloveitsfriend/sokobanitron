use crate::grid::Grid;

pub(crate) fn mask_to_player_reachable_in_place(grid: &mut Grid) {
    let height = grid.height();
    let width = grid.width();
    if height == 0 || width == 0 {
        return;
    }

    let cells = grid.cells_mut();

    // Find the first player tile in row-major order: '@' or '+'.
    let mut start: Option<usize> = None;
    for (i, &ch) in cells.iter().enumerate() {
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
    cells[start] |= VISITED;
    stack.push(start);

    while let Some(i) = stack.pop() {
        let r = i / width;
        let c = i - (r * width);

        // up
        if r > 0 {
            let ni = i - width;
            let ch = cells[ni];
            if (ch & VISITED) == 0 && ch != b'#' {
                cells[ni] = ch | VISITED;
                stack.push(ni);
            }
        }
        // down
        if r + 1 < height {
            let ni = i + width;
            let ch = cells[ni];
            if (ch & VISITED) == 0 && ch != b'#' {
                cells[ni] = ch | VISITED;
                stack.push(ni);
            }
        }
        // left
        if c > 0 {
            let ni = i - 1;
            let ch = cells[ni];
            if (ch & VISITED) == 0 && ch != b'#' {
                cells[ni] = ch | VISITED;
                stack.push(ni);
            }
        }
        // right
        if c + 1 < width {
            let ni = i + 1;
            let ch = cells[ni];
            if (ch & VISITED) == 0 && ch != b'#' {
                cells[ni] = ch | VISITED;
                stack.push(ni);
            }
        }
    }

    // Mask unreachable non-walls to '#', and clear visited marker from reachable tiles.
    for ch in cells.iter_mut().take(height * width) {
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
