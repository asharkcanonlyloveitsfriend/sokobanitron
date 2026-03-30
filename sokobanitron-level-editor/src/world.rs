use std::collections::{HashMap, HashSet};

const INITIAL_PATCH_SIZE: i32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Void,
    Floor,
    Goal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NonVoidBounds {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
}

#[derive(Clone)]
pub struct EditableWorld {
    tiles: HashMap<(i32, i32), Tile>,
    boxes: HashSet<(i32, i32)>,
    player: Option<(i32, i32)>,
}

impl EditableWorld {
    pub fn new() -> Self {
        let mut tiles = HashMap::new();
        let start = -INITIAL_PATCH_SIZE / 2;
        let end = start + INITIAL_PATCH_SIZE;
        for y in start..end {
            for x in start..end {
                tiles.insert((x, y), Tile::Floor);
            }
        }

        Self {
            tiles,
            boxes: HashSet::new(),
            player: None,
        }
    }

    pub fn tile(&self, world_x: i32, world_y: i32) -> Tile {
        self.tiles
            .get(&(world_x, world_y))
            .copied()
            .unwrap_or(Tile::Void)
    }

    pub fn set_tile(&mut self, world_x: i32, world_y: i32, tile: Tile) {
        let pos = (world_x, world_y);
        if tile == Tile::Void {
            self.tiles.remove(&pos);
            self.boxes.remove(&pos);
            if self.player == Some(pos) {
                self.player = None;
            }
        } else {
            self.tiles.insert(pos, tile);
        }
    }

    pub fn has_box(&self, world_x: i32, world_y: i32) -> bool {
        self.boxes.contains(&(world_x, world_y))
    }

    pub fn set_box(&mut self, world_x: i32, world_y: i32, has_box: bool) {
        let pos = (world_x, world_y);
        if has_box {
            assert_ne!(
                self.tile(world_x, world_y),
                Tile::Void,
                "cannot place a box on void"
            );
            self.boxes.insert(pos);
        } else {
            self.boxes.remove(&pos);
        }
    }

    pub fn player(&self) -> Option<(i32, i32)> {
        self.player
    }

    pub fn box_positions(&self) -> Vec<(i32, i32)> {
        let mut positions = self.boxes.iter().copied().collect::<Vec<_>>();
        positions.sort_unstable();
        positions
    }

    pub fn goal_positions(&self) -> Vec<(i32, i32)> {
        let mut positions = self
            .tiles
            .iter()
            .filter_map(|(pos, tile)| match tile {
                Tile::Goal => Some(*pos),
                _ => None,
            })
            .collect::<Vec<_>>();
        positions.sort_unstable();
        positions
    }

    pub fn set_player(&mut self, player: Option<(i32, i32)>) {
        if let Some((world_x, world_y)) = player {
            assert_ne!(
                self.tile(world_x, world_y),
                Tile::Void,
                "cannot place the player on void"
            );
            assert!(
                !self.has_box(world_x, world_y),
                "cannot place the player on a box"
            );
        }
        self.player = player;
    }

    pub fn non_void_bounds(&self) -> Option<NonVoidBounds> {
        let mut iter = self.tiles.keys();
        let (first_x, first_y) = if let Some((x, y)) = iter.next() {
            (*x, *y)
        } else if let Some((x, y)) = self.player {
            (x, y)
        } else {
            return None;
        };

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
        if let Some((x, y)) = self.player {
            bounds.min_x = bounds.min_x.min(x);
            bounds.max_x = bounds.max_x.max(x);
            bounds.min_y = bounds.min_y.min(y);
            bounds.max_y = bounds.max_y.max(y);
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
    use super::{EditableWorld, INITIAL_PATCH_SIZE, NonVoidBounds, Tile};

    #[test]
    fn seeded_world_starts_with_center_three_by_three_floor() {
        let world = EditableWorld::new();
        let start = -INITIAL_PATCH_SIZE / 2;
        let end = start + INITIAL_PATCH_SIZE;
        let mut floor_count = 0;
        for y in start..end {
            for x in start..end {
                assert_eq!(world.tile(x, y), Tile::Floor);
                floor_count += 1;
            }
        }
        assert_eq!(floor_count, INITIAL_PATCH_SIZE * INITIAL_PATCH_SIZE);
        assert_eq!(world.tile(end, 0), Tile::Void);
    }

    #[test]
    fn non_void_bounds_track_insertions_and_removals() {
        let mut world = EditableWorld::new();
        world.set_tile(10, -4, Tile::Goal);
        world.set_tile(-8, 7, Tile::Floor);
        world.set_box(-8, 7, true);

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

        world.set_tile(10, -4, Tile::Void);
        world.set_tile(-8, 7, Tile::Void);

        let bounds_after = world.non_void_bounds().expect("seed bounds");
        let start = -INITIAL_PATCH_SIZE / 2;
        let end_inclusive = start + INITIAL_PATCH_SIZE - 1;
        assert_eq!(bounds_after.min_x, start);
        assert_eq!(bounds_after.max_x, end_inclusive);
        assert_eq!(bounds_after.min_y, start);
        assert_eq!(bounds_after.max_y, end_inclusive);
    }

    #[test]
    fn clearing_a_tile_also_clears_its_box() {
        let mut world = EditableWorld::new();
        world.set_tile(4, 2, Tile::Floor);
        world.set_box(4, 2, true);

        world.set_tile(4, 2, Tile::Void);

        assert!(!world.has_box(4, 2));
    }
}
