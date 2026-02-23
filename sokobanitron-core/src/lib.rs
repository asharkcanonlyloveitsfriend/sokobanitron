mod canonical_hash;
mod grid;
mod mask_to_player_reachable;
mod normalize_leading_indentation_with_walls;
mod normalize_to_walkable_region;
mod prune_dead_end_floors;
mod prune_immovable_boxes_on_goals;
mod rectangularize_with_walls;
mod stage_profile;
mod trim_outer_walls;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    PlayerStartNotFound,
}

pub use canonical_hash::*;
pub use grid::*;
pub use mask_to_player_reachable::*;
pub use normalize_leading_indentation_with_walls::*;
pub use normalize_to_walkable_region::*;
pub use prune_dead_end_floors::*;
pub use prune_immovable_boxes_on_goals::*;
pub use rectangularize_with_walls::*;
pub use trim_outer_walls::*;
