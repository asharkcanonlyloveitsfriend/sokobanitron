use crate::grid::Grid;

pub(crate) fn prune_immovable_boxes_on_goals_in_place(grid: &mut Grid) {
    let height = grid.height();
    let width = grid.width();

    const MOVABLE_GOAL_BOX: u8 = b'm';

    // Collect all goal boxes ('*') from the original grid positions.
    let mut goal_boxes: Vec<usize> = Vec::new();
    for idx in 0..(height * width) {
        if grid.cells()[idx] == b'*' {
            goal_boxes.push(idx);
        }
    }

    if goal_boxes.is_empty() {
        return;
    }

    let cells_mut = grid.cells_mut();

    let mut changed = true;
    while changed {
        changed = false;

        for &idx in &goal_boxes {
            if cells_mut[idx] != b'*' {
                continue;
            }

            let up_cell = cells_mut[idx - width];
            let down_cell = cells_mut[idx + width];
            let left_cell = cells_mut[idx - 1];
            let right_cell = cells_mut[idx + 1];

            let up_wall = up_cell == b'#';
            let down_wall = down_cell == b'#';
            let left_wall = left_cell == b'#';
            let right_wall = right_cell == b'#';

            let vert_wall = up_wall || down_wall;
            let horiz_wall = left_wall || right_wall;

            if vert_wall && horiz_wall {
                cells_mut[idx] = b'#';
                changed = true;
                continue;
            }

            let up_box = up_cell == b'*';
            let down_box = down_cell == b'*';
            let left_box = left_cell == b'*';
            let right_box = right_cell == b'*';

            let vert_free = !vert_wall && !up_box && !down_box;
            let horiz_free = !horiz_wall && !left_box && !right_box;

            if vert_free || horiz_free {
                cells_mut[idx] = MOVABLE_GOAL_BOX;
                changed = true;
            }
        }
    }

    for &idx in &goal_boxes {
        if cells_mut[idx] == b'*' {
            cells_mut[idx] = b'#';
        } else if cells_mut[idx] == MOVABLE_GOAL_BOX {
            cells_mut[idx] = b'*';
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
