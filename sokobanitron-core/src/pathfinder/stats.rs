#[cfg(feature = "stats")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PathfinderStats {
    pub player_nodes_pushed: u64,
    pub player_nodes_expanded: u64,
    pub states_pushed: u64,
    pub states_expanded: u64,
    pub push_attempts: u64,
    pub player_pathfinder_calls: u64,
    pub player_pathfinder_successes: u64,
}

#[cfg(not(feature = "stats"))]
#[derive(Debug, Default, Clone, Copy)]
pub struct PathfinderStats;

#[cfg(feature = "stats")]
#[macro_export]
macro_rules! stat {
    ($receiver:ident, $field:ident += $value:expr) => {
        $receiver.stats.$field += $value;
    };
}

#[cfg(not(feature = "stats"))]
#[macro_export]
macro_rules! stat {
    ($receiver:ident, $field:ident += $value:expr) => {};
}
