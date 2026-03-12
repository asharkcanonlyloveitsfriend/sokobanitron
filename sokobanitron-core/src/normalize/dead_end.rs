use crate::grid::Grid;

pub(crate) fn prune_dead_end_floors_in_place(grid: &mut Grid) {
    let height = grid.height();
    let width = grid.width();

    let cells = grid.cells_mut();

    // degree[i] is the non-wall neighbor count for floor tiles; non-floor tiles stay -1.
    let mut degree: Vec<i8> = vec![-1; height * width];
    let mut dead_end_stack: Vec<usize> = Vec::new();

    for row in 0..height {
        let row_start = row * width;
        for col in 0..width {
            let idx = row_start + col;
            if cells[idx] != b' ' {
                continue;
            }

            let mut degree_count: i8 = 0;

            // up
            if row > 0 {
                let neighbor_idx = idx - width;
                if cells[neighbor_idx] != b'#' {
                    degree_count += 1;
                }
            }
            // down
            if row + 1 < height {
                let neighbor_idx = idx + width;
                if cells[neighbor_idx] != b'#' {
                    degree_count += 1;
                }
            }
            // left
            if col > 0 {
                let neighbor_idx = idx - 1;
                if cells[neighbor_idx] != b'#' {
                    degree_count += 1;
                }
            }
            // right
            if col + 1 < width {
                let neighbor_idx = idx + 1;
                if cells[neighbor_idx] != b'#' {
                    degree_count += 1;
                }
            }

            degree[idx] = degree_count;
            if degree_count <= 1 {
                dead_end_stack.push(idx);
            }
        }
    }

    while let Some(idx) = dead_end_stack.pop() {
        if cells[idx] != b' ' {
            continue;
        }

        cells[idx] = b'#';

        let row = idx / width;
        let col = idx - (row * width);

        // Decrement neighboring floor degrees and enqueue newly created dead ends.
        // up
        if row > 0 {
            let neighbor_idx = idx - width;
            if cells[neighbor_idx] == b' ' {
                let new_degree = degree[neighbor_idx] - 1;
                degree[neighbor_idx] = new_degree;
                if new_degree <= 1 {
                    dead_end_stack.push(neighbor_idx);
                }
            }
        }
        // down
        if row + 1 < height {
            let neighbor_idx = idx + width;
            if cells[neighbor_idx] == b' ' {
                let new_degree = degree[neighbor_idx] - 1;
                degree[neighbor_idx] = new_degree;
                if new_degree <= 1 {
                    dead_end_stack.push(neighbor_idx);
                }
            }
        }
        // left
        if col > 0 {
            let neighbor_idx = idx - 1;
            if cells[neighbor_idx] == b' ' {
                let new_degree = degree[neighbor_idx] - 1;
                degree[neighbor_idx] = new_degree;
                if new_degree <= 1 {
                    dead_end_stack.push(neighbor_idx);
                }
            }
        }
        // right
        if col + 1 < width {
            let neighbor_idx = idx + 1;
            if cells[neighbor_idx] == b' ' {
                let new_degree = degree[neighbor_idx] - 1;
                degree[neighbor_idx] = new_degree;
                if new_degree <= 1 {
                    dead_end_stack.push(neighbor_idx);
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

#[cfg(test)]
mod tests {
    use super::prune_dead_end_floors;

    fn lines(grid: &str) -> Vec<String> {
        grid.trim_matches('\n')
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect()
    }

    #[test]
    fn prunes_dead_end_floor_tiles() {
        let grid = "
#####
#@$.#
# # #
#####
";

        let result = prune_dead_end_floors(lines(grid));

        assert_eq!(
            result,
            vec![
                "#####".to_string(),
                "#@$.#".to_string(),
                "#####".to_string(),
                "#####".to_string(),
            ]
        );
    }
}
