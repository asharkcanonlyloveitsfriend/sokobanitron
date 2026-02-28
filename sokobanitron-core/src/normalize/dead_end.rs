use crate::grid::Grid;

pub(crate) fn prune_dead_end_floors_in_place(grid: &mut Grid) {
    let height = grid.height();
    let width = grid.width();
    if height == 0 || width == 0 {
        return;
    }

    let cells = grid.cells_mut();

    // Degree for ' ' tiles: number of non-wall neighbors (neighbor != '#').
    // Non-floor tiles have degree -1.
    let mut deg: Vec<i8> = vec![-1; height * width];
    let mut stack: Vec<usize> = Vec::new();

    for r in 0..height {
        let row_base = r * width;
        for c in 0..width {
            let i = row_base + c;
            if cells[i] != b' ' {
                continue;
            }

            let mut d: i8 = 0;

            // up
            if r > 0 {
                let ni = i - width;
                if cells[ni] != b'#' {
                    d += 1;
                }
            }
            // down
            if r + 1 < height {
                let ni = i + width;
                if cells[ni] != b'#' {
                    d += 1;
                }
            }
            // left
            if c > 0 {
                let ni = i - 1;
                if cells[ni] != b'#' {
                    d += 1;
                }
            }
            // right
            if c + 1 < width {
                let ni = i + 1;
                if cells[ni] != b'#' {
                    d += 1;
                }
            }

            deg[i] = d;
            if d <= 1 {
                stack.push(i);
            }
        }
    }

    while let Some(i) = stack.pop() {
        if cells[i] != b' ' {
            continue;
        }
        if deg[i] > 1 {
            continue;
        }

        // Remove dead-end floor.
        cells[i] = b'#';

        let r = i / width;
        let c = i - (r * width);

        // For each neighboring floor tile, decrement its degree.
        // Seed it if it becomes a dead end.
        // up
        if r > 0 {
            let ni = i - width;
            if cells[ni] == b' ' {
                let nd = deg[ni] - 1;
                deg[ni] = nd;
                if nd <= 1 {
                    stack.push(ni);
                }
            }
        }
        // down
        if r + 1 < height {
            let ni = i + width;
            if cells[ni] == b' ' {
                let nd = deg[ni] - 1;
                deg[ni] = nd;
                if nd <= 1 {
                    stack.push(ni);
                }
            }
        }
        // left
        if c > 0 {
            let ni = i - 1;
            if cells[ni] == b' ' {
                let nd = deg[ni] - 1;
                deg[ni] = nd;
                if nd <= 1 {
                    stack.push(ni);
                }
            }
        }
        // right
        if c + 1 < width {
            let ni = i + 1;
            if cells[ni] == b' ' {
                let nd = deg[ni] - 1;
                deg[ni] = nd;
                if nd <= 1 {
                    stack.push(ni);
                }
            }
        }
    }

    grid.debug_assert_invariants();
}

pub fn prune_dead_end_floors(lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let mut grid = Grid::from_lines(lines);
    prune_dead_end_floors_in_place(&mut grid);
    grid.into_lines()
}
