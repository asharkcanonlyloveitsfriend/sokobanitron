use crate::grid::Grid;

pub(crate) fn prune_immovable_boxes_on_goals_in_place(grid: &mut Grid) {
    let height = grid.height();
    let width = grid.width();
    if height == 0 || width == 0 {
        return;
    }

    // Immutable snapshot of the grid so decisions are based on the original state.
    let snapshot = grid.cells().to_vec();

    #[inline]
    fn snapshot_cell(snapshot: &[u8], w: usize, r: usize, c: usize) -> u8 {
        snapshot[r * w + c]
    }

    // Collect all goal boxes ('*')
    let mut goal_boxes: Vec<usize> = Vec::new();
    for i in 0..(height * width) {
        if grid.cells()[i] == b'*' {
            goal_boxes.push(i);
        }
    }

    let cells_mut = grid.cells_mut();

    // dependent_box -> (vertical_blockers, horizontal_blockers)
    use std::collections::HashMap;
    let mut box_dependencies: HashMap<usize, (Vec<usize>, Vec<usize>)> = HashMap::new();

    // Pass 1: detect wall‑locked boxes and collect dependency relationships.
    for &i in &goal_boxes {
        if cells_mut[i] != b'*' {
            continue;
        }

        let r = i / width;
        let c = i % width;

        let up_cell = snapshot_cell(&snapshot, width, r - 1, c);
        let down_cell = snapshot_cell(&snapshot, width, r + 1, c);
        let left_cell = snapshot_cell(&snapshot, width, r, c - 1);
        let right_cell = snapshot_cell(&snapshot, width, r, c + 1);

        let up_wall = up_cell == b'#';
        let down_wall = down_cell == b'#';
        let left_wall = left_cell == b'#';
        let right_wall = right_cell == b'#';

        let up_box = up_cell == b'*';
        let down_box = down_cell == b'*';
        let left_box = left_cell == b'*';
        let right_box = right_cell == b'*';

        let vert_free = !up_wall && !up_box && !down_wall && !down_box;
        let horiz_free = !left_wall && !left_box && !right_wall && !right_box;

        if vert_free || horiz_free {
            continue;
        }

        let vert_wall = up_wall || down_wall;
        let horiz_wall = left_wall || right_wall;

        // Box permanently trapped by walls.
        if vert_wall && horiz_wall {
            cells_mut[i] = b'#';
            continue;
        }

        let mut vertical_blockers: Vec<usize> = Vec::with_capacity(2);
        let mut horizontal_blockers: Vec<usize> = Vec::with_capacity(2);

        if !vert_wall {
            if up_box {
                vertical_blockers.push(i - width);
            }
            if down_box {
                vertical_blockers.push(i + width);
            }
        }

        if !horiz_wall {
            if left_box {
                horizontal_blockers.push(i - 1);
            }
            if right_box {
                horizontal_blockers.push(i + 1);
            }
        }

        if vertical_blockers.is_empty() && horizontal_blockers.is_empty() {
            cells_mut[i] = b'#';
        } else {
            box_dependencies.insert(i, (vertical_blockers, horizontal_blockers));
        }
    }

    // Remove dependencies that now point to non‑boxes.
    let mut to_remove: Vec<usize> = Vec::new();

    for (&dependent_box, (vertical, horizontal)) in box_dependencies.iter_mut() {
        vertical.retain(|&b| cells_mut[b] == b'*');
        horizontal.retain(|&b| cells_mut[b] == b'*');

        if vertical.is_empty() && horizontal.is_empty() {
            to_remove.push(dependent_box);
        }
    }

    for dependent_box in to_remove {
        box_dependencies.remove(&dependent_box);
        cells_mut[dependent_box] = b'#';
    }

    // Boxes with no dependencies are immediately movable.
    let mut immediately_movable: Vec<usize> = Vec::new();
    for &i in &goal_boxes {
        if cells_mut[i] == b'*' && !box_dependencies.contains_key(&i) {
            immediately_movable.push(i);
        }
    }

    // Proven movable boxes grows as dependencies resolve.
    use std::collections::HashSet;
    let mut proven_movable: HashSet<usize> = immediately_movable.iter().copied().collect();

    let mut changed = true;
    while changed {
        changed = false;

        for (&dependent_box, (vertical, horizontal)) in &box_dependencies {
            if proven_movable.contains(&dependent_box) {
                continue;
            }

            let vertical_present = !vertical.is_empty();
            let horizontal_present = !horizontal.is_empty();

            let vertical_resolved = vertical_present && vertical.iter().all(|b| proven_movable.contains(b));
            let horizontal_resolved = horizontal_present && horizontal.iter().all(|b| proven_movable.contains(b));

            if vertical_resolved || horizontal_resolved {
                proven_movable.insert(dependent_box);
                changed = true;
            }
        }
    }

    // Remaining dependency-only boxes are immovable.
    for &dependent_box in box_dependencies.keys() {
        if !proven_movable.contains(&dependent_box) {
            cells_mut[dependent_box] = b'#';
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

#[cfg(test)]
mod tests {
    use super::prune_immovable_boxes_on_goals;

    fn lines(grid: &str) -> Vec<String> {
        grid.trim_matches('\n')
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect()
    }

    #[test]
    fn removes_immovable_boxes_on_goals() {
        let grid = "
#######
#.@   #
# **  #
# **  #
#   $ #
#######
";

        let result = prune_immovable_boxes_on_goals(lines(grid));

        assert_eq!(
            result,
            vec![
                "#######".to_string(),
                "#.@   #".to_string(),
                "# ##  #".to_string(),
                "# ##  #".to_string(),
                "#   $ #".to_string(),
                "#######".to_string(),
            ]
        );
    }

    #[test]
    fn keeps_movable_boxes_on_goals() {
        let grid = "
#######
#.@   #
# *   #
# **  #
#   $ #
#######
";

        let result = prune_immovable_boxes_on_goals(lines(grid));

        assert_eq!(
            result,
            vec![
                "#######".to_string(),
                "#.@   #".to_string(),
                "# *   #".to_string(),
                "# **  #".to_string(),
                "#   $ #".to_string(),
                "#######".to_string(),
            ]
        );
    }

    #[test]
    fn does_not_treat_single_resolved_axis_neighbor_as_sufficient() {
        let grid = "
#########
#.@     #
#  **   #
#  **   #
#   **  #
#   $   #
#########
";

        let result = prune_immovable_boxes_on_goals(lines(grid));

        assert_eq!(
            result,
            vec![
                "#########".to_string(),
                "#.@     #".to_string(),
                "#  ##   #".to_string(),
                "#  ##   #".to_string(),
                "#   **  #".to_string(),
                "#   $   #".to_string(),
                "#########".to_string(),
            ]
        );
    }
}
