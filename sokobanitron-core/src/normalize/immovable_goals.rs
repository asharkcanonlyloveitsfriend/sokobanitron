use std::collections::{HashMap, HashSet};
use crate::grid::Grid;

pub(crate) fn prune_immovable_boxes_on_goals_in_place(grid: &mut Grid) {
    let height = grid.height();
    let width = grid.width();
    if height == 0 || width == 0 {
        return;
    }

    let view = grid.cells().to_vec();

    #[inline]
    fn at_view(view: &[u8], h: usize, w: usize, r: isize, c: isize) -> u8 {
        if r < 0 || c < 0 || r >= h as isize || c >= w as isize {
            b'#'
        } else {
            view[(r as usize) * w + (c as usize)]
        }
    }

    let mut stars: Vec<usize> = Vec::new();
    for i in 0..(height * width) {
        if grid.cells()[i] == b'*' {
            stars.push(i);
        }
    }

    let cells = grid.cells_mut();

    // a -> dependencies that can unblock a
    let mut deps: HashMap<usize, Vec<usize>> = HashMap::new();

    // Pass 1: wall-locked stars and dependency collection.
    for &i in &stars {
        if cells[i] != b'*' {
            continue;
        }

        let r = i / width;
        let c = i % width;

        let u = at_view(&view, height, width, r as isize - 1, c as isize);
        let d = at_view(&view, height, width, r as isize + 1, c as isize);
        let l = at_view(&view, height, width, r as isize, c as isize - 1);
        let rr = at_view(&view, height, width, r as isize, c as isize + 1);

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
            cells[i] = b'#';
            continue;
        }

        let mut dep_set: Vec<usize> = Vec::with_capacity(2);
        if !vert_hard {
            if u_star {
                dep_set.push(i - width);
            }
            if d_star {
                dep_set.push(i + width);
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
            cells[i] = b'#';
        } else {
            deps.insert(i, dep_set);
        }
    }

    // Filter dependencies that now point to non-stars.
    let mut to_remove: Vec<usize> = Vec::new();
    for (&a, dep_list) in deps.iter_mut() {
        dep_list.retain(|&b| cells[b] == b'*');
        if dep_list.is_empty() {
            to_remove.push(a);
        }
    }
    for a in to_remove {
        deps.remove(&a);
        cells[a] = b'#';
    }

    // Base movable stars are remaining '*' not in deps.
    let mut base_movable: Vec<usize> = Vec::new();
    for &i in &stars {
        if cells[i] == b'*' && !deps.contains_key(&i) {
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
            cells[a] = b'#';
        }
    }

    grid.debug_assert_invariants();
}

pub fn prune_immovable_boxes_on_goals(lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    let mut grid = Grid::from_lines(lines);
    prune_immovable_boxes_on_goals_in_place(&mut grid);
    grid.into_lines()
}
