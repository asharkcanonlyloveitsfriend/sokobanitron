use crate::canonical;
use crate::error::CoreError;

pub fn canonical_hash(grid: &str) -> Result<String, CoreError> {
    canonical::canonical_hash(grid)
}
