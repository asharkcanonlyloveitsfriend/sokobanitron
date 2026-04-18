use super::Position;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldBounds {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

impl WorldBounds {
    pub fn from_points<I>(points: I) -> Option<Self>
    where
        I: IntoIterator<Item = (i32, i32)>,
    {
        let mut iter = points.into_iter();
        let (first_x, first_y) = iter.next()?;
        let mut bounds = Self {
            min_x: first_x,
            max_x: first_x,
            min_y: first_y,
            max_y: first_y,
        };
        for point in iter {
            bounds.include(point);
        }
        Some(bounds)
    }

    pub fn include(&mut self, (x, y): (i32, i32)) {
        self.min_x = self.min_x.min(x);
        self.max_x = self.max_x.max(x);
        self.min_y = self.min_y.min(y);
        self.max_y = self.max_y.max(y);
    }

    fn width(self) -> usize {
        span_len(self.min_x, self.max_x, "x")
    }

    fn height(self) -> usize {
        span_len(self.min_y, self.max_y, "y")
    }
}

fn span_len(min: i32, max: i32, axis: &str) -> usize {
    let span = i64::from(max) - i64::from(min) + 1;
    assert!(
        span > 0,
        "WorldBounds invariant violated: max_{axis} must be greater than or equal to min_{axis}"
    );
    usize::try_from(span)
        .expect("WorldBounds invariant violated: coordinate span must fit in usize")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGridOrigin {
    pub x: i32,
    pub y: i32,
}

impl WorldGridOrigin {
    pub fn to_world_position(self, grid: Position) -> (i32, i32) {
        let col = i64::try_from(grid.col)
            .expect("WorldGrid invariant violated: grid column must fit in i64");
        let row = i64::try_from(grid.row)
            .expect("WorldGrid invariant violated: grid row must fit in i64");
        let x = i64::from(self.x) + col;
        let y = i64::from(self.y) + row;
        (
            i32::try_from(x).expect("WorldGrid invariant violated: world x must fit in i32"),
            i32::try_from(y).expect("WorldGrid invariant violated: world y must fit in i32"),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldGrid {
    rows: Vec<Vec<bool>>,
    origin: WorldGridOrigin,
}

impl WorldGrid {
    pub fn from_bounds<F>(bounds: WorldBounds, mut is_walkable: F) -> Self
    where
        F: FnMut((i32, i32)) -> bool,
    {
        let width = bounds.width();
        let height = bounds.height();
        let mut rows = vec![vec![false; width]; height];
        for y in bounds.min_y..=bounds.max_y {
            for x in bounds.min_x..=bounds.max_x {
                let row = usize::try_from(i64::from(y) - i64::from(bounds.min_y)).expect(
                    "WorldBounds invariant violated: row offset must be non-negative and fit usize",
                );
                let col = usize::try_from(i64::from(x) - i64::from(bounds.min_x)).expect(
                    "WorldBounds invariant violated: column offset must be non-negative and fit usize",
                );
                rows[row][col] = is_walkable((x, y));
            }
        }

        Self {
            rows,
            origin: WorldGridOrigin {
                x: bounds.min_x,
                y: bounds.min_y,
            },
        }
    }

    pub fn origin(&self) -> WorldGridOrigin {
        self.origin
    }

    pub fn to_grid_position(&self, (x, y): (i32, i32)) -> Option<Position> {
        let col = i64::from(x) - i64::from(self.origin.x);
        let row = i64::from(y) - i64::from(self.origin.y);
        let row = usize::try_from(row).ok()?;
        let col = usize::try_from(col).ok()?;
        if row < self.rows.len() && col < self.rows.first().map_or(0, Vec::len) {
            Some(Position::new(row, col))
        } else {
            None
        }
    }

    pub fn into_rows(self) -> Vec<Vec<bool>> {
        self.rows
    }
}

#[cfg(test)]
mod tests {
    use super::{WorldBounds, WorldGrid};
    use crate::pathfinder::Position;

    #[test]
    fn bounds_expand_to_include_points() {
        let mut bounds = WorldBounds::from_points([(2, -1), (-4, 3)]).expect("bounds from points");

        bounds.include((8, -6));

        assert_eq!(bounds.min_x, -4);
        assert_eq!(bounds.max_x, 8);
        assert_eq!(bounds.min_y, -6);
        assert_eq!(bounds.max_y, 3);
    }

    #[test]
    fn grid_maps_world_positions_through_origin() {
        let bounds = WorldBounds {
            min_x: -2,
            max_x: 0,
            min_y: 4,
            max_y: 5,
        };
        let grid = WorldGrid::from_bounds(bounds, |(x, y)| x == -1 && y == 5);
        let origin = grid.origin();

        assert_eq!(grid.to_grid_position((-1, 5)), Some(Position::new(1, 1)));
        assert_eq!(origin.to_world_position(Position::new(1, 1)), (-1, 5));
        assert_eq!(
            grid.into_rows(),
            vec![vec![false, false, false], vec![false, true, false]]
        );
    }

    #[test]
    fn grid_round_trips_multiple_negative_world_positions() {
        let bounds = WorldBounds {
            min_x: -5,
            max_x: -1,
            min_y: -4,
            max_y: -2,
        };
        let grid = WorldGrid::from_bounds(bounds, |_| true);
        let origin = grid.origin();

        for world in [(-5, -4), (-3, -3), (-1, -2)] {
            let grid_position = grid.to_grid_position(world).expect("in-bounds position");
            assert_eq!(origin.to_world_position(grid_position), world);
        }
        assert_eq!(grid.to_grid_position((0, -2)), None);
    }
}
