mod bootstrap;
mod import;
mod normalize;
mod schema;
mod slc;
mod solution;
mod store;

#[cfg(test)]
mod tests;

use std::io;

pub use bootstrap::BootstrappedLevelStore;
pub use store::{LevelPersistence, LevelSetCatalogEntry, LoadedLevelSetData};

fn sqlite_error(err: rusqlite::Error) -> io::Error {
    io::Error::other(format!("sqlite: {err}"))
}
