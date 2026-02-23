use std::collections::{HashMap, HashSet};
use crate::grid::Grid;

pub fn prune_immovable_boxes_on_goals_in_place(grid: &mut Grid) {
    if grid.h == 0 || grid.w == 0 {
        return;
    }

    let view = grid.cells.clone();

    #[inline]
    fn at_view(view: &[u8], h: usize, w: usize, r: isize, c: isize) -> u8 {
        if r < 0 || c < 0 || r >= h as isize || c >= w as isize {
            b'#'
        } else {
            view[(r as usize) * w + (c as usize)]
        }
    }

    let mut stars: Vec<usize> = Vec::new();
    for i in 0..(grid.h * grid.w) {
        if grid.cells[i] == b'*' {
            stars.push(i);
        }
    }

    // a -> dependencies that can unblock a
    let mut deps: HashMap<usize, Vec<usize>> = HashMap::new();

    // Pass 1: wall-locked stars and dependency collection.
    for &i in &stars {
        if grid.cells[i] != b'*' {
            continue;
        }

        let r = i / grid.w;
        let c = i % grid.w;

        let u = at_view(&view, grid.h, grid.w, r as isize - 1, c as isize);
        let d = at_view(&view, grid.h, grid.w, r as isize + 1, c as isize);
        let l = at_view(&view, grid.h, grid.w, r as isize, c as isize - 1);
        let rr = at_view(&view, grid.h, grid.w, r as isize, c as isize + 1);

        let u_hard = u == b'#';
        let d_hard = d == b'#';
        let l_hard = l == b'#';
        let r_hard = rr == b'#';

        let u_star = u == b'*';
        let d_star = d == b'*';
        let l_star = l == b'*';
        let r_star = rr == b'*';

        let vert_free = !u_hard && !u_star && !d_hard && !d_star;
        let horiz_free = !l_hard && !l_star && !r_hard && !r_star;
        if vert_free || horiz_free {
            continue;
        }

        let vert_hard = u_hard || d_hard;
        let horiz_hard = l_hard || r_hard;
        if vert_hard && horiz_hard {
            grid.cells[i] = b'#';
            continue;
        }

        let mut dep_set: Vec<usize> = Vec::with_capacity(2);
        if !vert_hard {
            if u_star {
                dep_set.push(i - grid.w);
            }
            if d_star {
                dep_set.push(i + grid.w);
            }
        }
        if !horiz_hard {
            if l_star {
                dep_set.push(i - 1);
            }
            if r_star {
                dep_set.push(i + 1);
            }
        }

        if dep_set.is_empty() {
            grid.cells[i] = b'#';
        } else {
            deps.insert(i, dep_set);
        }
    }

    // Filter dependencies that now point to non-stars.
    let mut to_remove: Vec<usize> = Vec::new();
    for (&a, dep_list) in deps.iter_mut() {
        dep_list.retain(|&b| grid.cells[b] == b'*');
        if dep_list.is_empty() {
            to_remove.push(a);
        }
    }
    for a in to_remove {
        deps.remove(&a);
        grid.cells[a] = b'#';
    }

    // Base movable stars are remaining '*' not in deps.
    let mut base_movable: Vec<usize> = Vec::new();
    for &i in &stars {
        if grid.cells[i] == b'*' && !deps.contains_key(&i) {
            base_movable.push(i);
        }
    }

    // Reverse edges: dependency -> dependents.
    let mut rev: HashMap<usize, Vec<usize>> = HashMap::new();
    for (&a, dep_list) in &deps {
        for &b in dep_list {
            rev.entry(b).or_default().push(a);
        }
    }

    let mut movable: HashSet<usize> = base_movable.iter().copied().collect();
    let mut stack = base_movable;
    while let Some(b) = stack.pop() {
        if let Some(dependents) = rev.get(&b) {
            for &a in dependents {
                if movable.insert(a) {
                    stack.push(a);
                }
            }
        }
    }

    // Unresolved dependency-only stars are mutually dependent and immovable.
    for &a in deps.keys() {
        if !movable.contains(&a) {
            grid.cells[a] = b'#';
        }
    }
}

pub fn prune_immovable_boxes_on_goals(lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let mut grid = Grid::from_lines(lines);
    prune_immovable_boxes_on_goals_in_place(&mut grid);
    grid.into_lines()
}
