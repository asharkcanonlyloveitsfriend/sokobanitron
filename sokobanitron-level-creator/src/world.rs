use crate::constants::INITIAL_PATCH_SIZE;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditableTile {
    Void,
    Floor,
    Goal,
    Box,
    BoxOnGoal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonVoidBounds {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

pub struct EditableWorld {
    tiles: HashMap<(i32, i32), EditableTile>,
}

impl EditableWorld {
    pub fn new() -> Self {
        let mut tiles = HashMap::new();
        let start = -INITIAL_PATCH_SIZE / 2;
        let end = start + INITIAL_PATCH_SIZE;
        for y in start..end {
            for x in start..end {
                tiles.insert((x, y), EditableTile::Floor);
            }
        }
        Self { tiles }
    }

    pub fn tile(&self, world_x: i32, world_y: i32) -> EditableTile {
        self.tiles
            .get(&(world_x, world_y))
            .copied()
            .unwrap_or(EditableTile::Void)
    }

    pub fn set_tile(&mut self, world_x: i32, world_y: i32, tile: EditableTile) {
        let pos = (world_x, world_y);
        if tile == EditableTile::Void {
            self.tiles.remove(&pos);
        } else {
            self.tiles.insert(pos, tile);
        }
    }

    pub fn non_void_bounds(&self) -> Option<NonVoidBounds> {
        let mut iter = self.tiles.keys();
        let (first_x, first_y) = *iter.next()?;
        let mut bounds = NonVoidBounds {
            min_x: first_x,
            max_x: first_x,
            min_y: first_y,
            max_y: first_y,
        };
        for (x, y) in iter {
            bounds.min_x = bounds.min_x.min(*x);
            bounds.max_x = bounds.max_x.max(*x);
            bounds.min_y = bounds.min_y.min(*y);
            bounds.max_y = bounds.max_y.max(*y);
        }
        Some(bounds)
    }
}

impl Default for EditableWorld {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{EditableTile, EditableWorld, NonVoidBounds};

    #[test]
    fn seeded_world_starts_with_center_three_by_three_floor() {
        let world = EditableWorld::new();
        let mut floor_count = 0;
        for y in -1..=1 {
            for x in -1..=1 {
                assert_eq!(world.tile(x, y), EditableTile::Floor);
                floor_count += 1;
            }
        }
        assert_eq!(floor_count, 9);
        assert_eq!(world.tile(2, 0), EditableTile::Void);
    }

    #[test]
    fn non_void_bounds_track_insertions_and_removals() {
        let mut world = EditableWorld::new();
        world.set_tile(10, -4, EditableTile::Goal);
        world.set_tile(-8, 7, EditableTile::Box);

        let bounds = world.non_void_bounds().expect("bounds");
        assert_eq!(
            bounds,
            NonVoidBounds {
                min_x: -8,
                max_x: 10,
                min_y: -4,
                max_y: 7,
            }
        );

        world.set_tile(10, -4, EditableTile::Void);
        world.set_tile(-8, 7, EditableTile::Void);

        let bounds_after = world.non_void_bounds().expect("seed bounds");
        assert_eq!(bounds_after.min_x, -1);
        assert_eq!(bounds_after.max_x, 1);
        assert_eq!(bounds_after.min_y, -1);
        assert_eq!(bounds_after.max_y, 1);
    }
}
