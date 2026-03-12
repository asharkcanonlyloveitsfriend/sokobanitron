use std::time::Instant;

use crate::normalize::build_grid::build_rectangular_grid;
use crate::normalize::dead_end::prune_dead_end_floors_in_place;
use crate::normalize::immovable_boxes::prune_immovable_boxes_on_goals_in_place;
use crate::normalize::reachable::mask_to_player_reachable_in_place;
use crate::normalize::trim_outer::trim_outer_walls_in_place;
use crate::util::stage_profile;

pub fn normalize_to_walkable_region_lines(lines: Vec<String>) -> Vec<String> {
    let t0 = Instant::now();
    let mut grid = build_rectangular_grid(lines);
    stage_profile::record("normalize.build_grid", t0.elapsed());

    let t1 = Instant::now();
    mask_to_player_reachable_in_place(&mut grid);
    stage_profile::record("normalize.mask_reachable", t1.elapsed());

    let t2 = Instant::now();
    prune_immovable_boxes_on_goals_in_place(&mut grid);
    stage_profile::record("normalize.prune_immovable_boxes", t2.elapsed());

    let t3 = Instant::now();
    prune_dead_end_floors_in_place(&mut grid);
    stage_profile::record("normalize.prune_dead_ends", t3.elapsed());

    let t4 = Instant::now();
    trim_outer_walls_in_place(&mut grid);
    stage_profile::record("normalize.trim_outer_walls", t4.elapsed());

    let t5 = Instant::now();
    let out = grid.into_lines();
    stage_profile::record("normalize.into_lines", t5.elapsed());
    out
}

pub fn normalize_to_walkable_region(lines: Vec<String>) -> Vec<String> {
    normalize_to_walkable_region_lines(lines)
}

#[cfg(test)]
mod tests {
    use super::normalize_to_walkable_region_lines;

    fn normalize_grid(grid: &str) -> Vec<String> {
        let lines: Vec<String> = grid
            .trim_matches('\n')
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect();
        normalize_to_walkable_region_lines(lines)
    }

    #[test]
    fn normalize_to_walkable_region_basic_grid() {
        let grid = "
    #####
    #.$@#
    #####
    ";

        assert_eq!(normalize_grid(grid), vec![".$@"]);
    }

    #[test]
    fn normalize_to_walkable_region_complex_shape() {
        let grid = "
    #######
    #     #
######### ##
#   #      #
# @    $  ##
#####  ## #
    #   . #
    ### ###
      # #
      # ###
      #   #
      #####
    ";

        assert_eq!(
            normalize_grid(grid),
            vec![
                "   #     ".to_string(),
                " @    $  ".to_string(),
                "####  ## ".to_string(),
                "####   . ".to_string(),
            ]
        );
    }

    #[test]
    fn normalization_is_idempotent() {
        let grid = vec![
            ".@   ".to_string(),
            " ##  ".to_string(),
            " ##  ".to_string(),
            "   $ ".to_string(),
        ];

        let normalized = normalize_to_walkable_region_lines(grid.clone());

        assert_eq!(grid, normalized);
    }
}
