use crate::grid::Grid;

pub fn trim_outer_walls_in_place(grid: &mut Grid) {
    if grid.h == 0 || grid.w == 0 {
        return;
    }

    let mut top = 0usize;
    while top < grid.h {
        let mut all_wall = true;
        for c in 0..grid.w {
            if grid.cells[grid.idx(top, c)] != b'#' {
                all_wall = false;
                break;
            }
        }
        if !all_wall {
            break;
        }
        top += 1;
    }

    if top == grid.h {
        grid.h = 0;
        grid.w = 0;
        grid.cells.clear();
        return;
    }

    let mut bottom = grid.h - 1;
    while bottom > top {
        let mut all_wall = true;
        for c in 0..grid.w {
            if grid.cells[grid.idx(bottom, c)] != b'#' {
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
    while left < grid.w {
        let mut all_wall = true;
        for r in top..=bottom {
            if grid.cells[grid.idx(r, left)] != b'#' {
                all_wall = false;
                break;
            }
        }
        if !all_wall {
            break;
        }
        left += 1;
    }

    let mut right = grid.w - 1;
    while right >= left {
        let mut all_wall = true;
        for r in top..=bottom {
            if grid.cells[grid.idx(r, right)] != b'#' {
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
        new_cells.extend_from_slice(&grid.cells[start..end]);
    }

    grid.h = new_h;
    grid.w = new_w;
    grid.cells = new_cells;
}

pub fn trim_outer_walls(lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let mut grid = Grid::from_lines(lines);
    trim_outer_walls_in_place(&mut grid);
    grid.into_lines()
}
