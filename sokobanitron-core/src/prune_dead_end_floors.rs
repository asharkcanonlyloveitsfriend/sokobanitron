use crate::grid::Grid;

pub fn prune_dead_end_floors_in_place(grid: &mut Grid) {
    if grid.h == 0 || grid.w == 0 {
        return;
    }

    // Degree for ' ' tiles: number of non-wall neighbors (neighbor != '#').
    // Non-floor tiles have degree -1.
    let mut deg: Vec<i8> = vec![-1; grid.h * grid.w];
    let mut stack: Vec<usize> = Vec::new();

    for r in 0..grid.h {
        let row_base = r * grid.w;
        for c in 0..grid.w {
            let i = row_base + c;
            if grid.cells[i] != b' ' {
                continue;
            }

            let mut d: i8 = 0;

            // up
            if r > 0 {
                let ni = i - grid.w;
                if grid.cells[ni] != b'#' {
                    d += 1;
                }
            }
            // down
            if r + 1 < grid.h {
                let ni = i + grid.w;
                if grid.cells[ni] != b'#' {
                    d += 1;
                }
            }
            // left
            if c > 0 {
                let ni = i - 1;
                if grid.cells[ni] != b'#' {
                    d += 1;
                }
            }
            // right
            if c + 1 < grid.w {
                let ni = i + 1;
                if grid.cells[ni] != b'#' {
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
        if grid.cells[i] != b' ' {
            continue;
        }
        if deg[i] > 1 {
            continue;
        }

        // Remove dead-end floor.
        grid.cells[i] = b'#';

        let r = i / grid.w;
        let c = i - (r * grid.w);

        // For each neighboring floor tile, decrement its degree.
        // Seed it if it becomes a dead end.
        // up
        if r > 0 {
            let ni = i - grid.w;
            if grid.cells[ni] == b' ' {
                let nd = deg[ni] - 1;
                deg[ni] = nd;
                if nd <= 1 {
                    stack.push(ni);
                }
            }
        }
        // down
        if r + 1 < grid.h {
            let ni = i + grid.w;
            if grid.cells[ni] == b' ' {
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
            if grid.cells[ni] == b' ' {
                let nd = deg[ni] - 1;
                deg[ni] = nd;
                if nd <= 1 {
                    stack.push(ni);
                }
            }
        }
        // right
        if c + 1 < grid.w {
            let ni = i + 1;
            if grid.cells[ni] == b' ' {
                let nd = deg[ni] - 1;
                deg[ni] = nd;
                if nd <= 1 {
                    stack.push(ni);
                }
            }
        }
    }
}

pub fn prune_dead_end_floors(lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let mut grid = Grid::from_lines(lines);
    prune_dead_end_floors_in_place(&mut grid);
    grid.into_lines()
}
