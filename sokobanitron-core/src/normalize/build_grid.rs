use crate::grid::Grid;
use crate::grid::validation::{assert_has_goal, assert_single_player, validate_characters};

pub(crate) fn build_rectangular_grid(lines: &[String]) -> Grid {
    validate_characters(lines);
    assert_single_player(lines);
    assert_has_goal(lines);

    let height = lines.len();
    let width = lines.iter().map(String::len).max().unwrap();

    let mut cells = vec![b'#'; height * width];
    let mut row_start = 0;
    for line in lines {
        let bytes = line.as_bytes();
        let len = bytes.len();

        cells[row_start..row_start + len].copy_from_slice(bytes);
        row_start += width;
    }

    Grid::from_shape_and_cells(width, height, cells)
}

#[cfg(test)]
mod tests {
    use super::build_rectangular_grid;

    fn lines(grid: &str) -> Vec<String> {
        grid.trim_matches('\n')
            .lines()
            .map(|l| l.trim_end().to_string())
            .collect()
    }

    #[test]
    fn preserves_leading_spaces_and_keeps_rectangle() {
        // Leading spaces are preserved; the function only guarantees rectangular shape.
        let grid = "
   #####
   #@$.#
   #####
";

        let lines = lines(grid);
        let g = build_rectangular_grid(&lines);

        assert_eq!(
            g.into_lines(),
            vec![
                "   #####".to_string(),
                "   #@$.#".to_string(),
                "   #####".to_string(),
            ]
        );
    }

    #[test]
    fn handles_irregular_line_lengths_and_keeps_interior_spaces() {
        let grid = "
#######
#@ $ .#
#####
";

        let lines = lines(grid);
        let g = build_rectangular_grid(&lines);

        assert_eq!(
            g.into_lines(),
            vec![
                "#######".to_string(),
                "#@ $ .#".to_string(),
                "#######".to_string(),
            ]
        );
    }

    #[test]
    fn copies_row_without_walls_verbatim() {
        let grid = "
@$.
";

        let lines = lines(grid);
        let g = build_rectangular_grid(&lines);

        assert_eq!(g.into_lines(), vec!["@$.".to_string(),]);
    }
}
