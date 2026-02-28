use crate::grid::Grid;

pub(crate) fn trim_outer_walls_in_place(grid: &mut Grid) {
    let height = grid.height();
    let width = grid.width();
    if height == 0 || width == 0 {
        return;
    }

    let cells = grid.cells();
    let mut top = 0usize;
    while top < height {
        let mut all_wall = true;
        for c in 0..width {
            if cells[grid.idx(top, c)] != b'#' {
                all_wall = false;
                break;
            }
        }
        if !all_wall {
            break;
        }
        top += 1;
    }

    if top == height {
        grid.clear();
        return;
    }

    let mut bottom = height - 1;
    while bottom > top {
        let mut all_wall = true;
        for c in 0..width {
            if cells[grid.idx(bottom, c)] != b'#' {
                all_wall = false;
                break;
            }
        }
        if !all_wall {
            break;
        }
        bottom -= 1;
    }

    let mut left = 0usize;
    while left < width {
        let mut all_wall = true;
        for r in top..=bottom {
            if cells[grid.idx(r, left)] != b'#' {
                all_wall = false;
                break;
            }
        }
        if !all_wall {
            break;
        }
        left += 1;
    }

    let mut right = width - 1;
    while right >= left {
        let mut all_wall = true;
        for r in top..=bottom {
            if cells[grid.idx(r, right)] != b'#' {
                all_wall = false;
                break;
            }
        }
        if !all_wall {
            break;
        }
        if right == 0 {
            break;
        }
        right -= 1;
    }

    let new_h = bottom - top + 1;
    let new_w = right - left + 1;
    let mut new_cells: Vec<u8> = Vec::with_capacity(new_h * new_w);
    for r in top..=bottom {
        let start = grid.idx(r, left);
        let end = start + new_w;
        new_cells.extend_from_slice(&cells[start..end]);
    }

    grid.set_shape_and_cells(new_w, new_h, new_cells);
}

pub fn trim_outer_walls(lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let mut grid = Grid::from_lines(lines);
    trim_outer_walls_in_place(&mut grid);
    grid.into_lines()
}
