pub mod api;
pub mod canonical;
pub mod error;
pub mod normalize;
pub mod optimizer;
pub mod pathfinder;

mod grid;

pub use api::canonical_hash::canonical_hash;
pub use api::normalize::{normalize_to_walkable_region, normalize_to_walkable_region_lines};
pub use error::CoreError;
pub use normalize::dead_end::prune_dead_end_floors;
pub use normalize::immovable_boxes::prune_immovable_boxes_on_goals;
pub use normalize::reachable::mask_to_player_reachable;
pub use normalize::rectangularize::rectangularize_with_walls;
pub use normalize::rectangularize::rectangularize_with_walls_in_place;
pub use normalize::trim_outer::trim_outer_walls;
