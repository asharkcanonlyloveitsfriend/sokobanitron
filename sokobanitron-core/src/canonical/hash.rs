use std::cmp::Ordering;

use sha2::{Digest, Sha256};

use crate::canonical::player_canonicalization::canonicalize_player_start_in_place;
use crate::canonical::symmetry::{FlatGrid, VARIANTS, build_variant_grid, compare_variant_keys};
use crate::error::CoreError;
use crate::normalize::pipeline::normalize_to_walkable_region_lines;

pub fn canonical_hash(grid: &str) -> Result<String, CoreError> {
    canonical_hash_impl(grid)
}

fn split_grid_lines(grid: &str) -> Vec<String> {
    grid.trim_matches('\n')
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect()
}

fn canonical_hash_impl(grid: &str) -> Result<String, CoreError> {
    let split = split_grid_lines(grid);

    let normalized = normalize_to_walkable_region_lines(split);

    let h = normalized.len();
    let w = if h == 0 { 0 } else { normalized[0].len() };
    let mut flat = Vec::with_capacity(h * w);
    for row in normalized {
        flat.extend_from_slice(row.as_bytes());
    }
    let base = FlatGrid { h, w, cells: &flat };
    base.debug_assert_invariants();

    let mut best = VARIANTS[0];
    for &v in &VARIANTS[1..] {
        if compare_variant_keys(base, v, best) == Ordering::Less {
            best = v;
        }
    }

    let (vh, vw, mut canonical_cells) = build_variant_grid(base, best);
    debug_assert_eq!(canonical_cells.len(), vh * vw);

    canonicalize_player_start_in_place(&mut canonical_cells, vh, vw)?;

    let mut hasher = Sha256::new();
    for r in 0..vh {
        let start = r * vw;
        let end = start + vw;
        hasher.update(&canonical_cells[start..end]);
        if r + 1 < vh {
            hasher.update(b"\n");
        }
    }

    let digest = hasher.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out_bytes = [0u8; 64];
    for (i, b) in digest.iter().copied().enumerate() {
        out_bytes[i * 2] = HEX[(b >> 4) as usize];
        out_bytes[i * 2 + 1] = HEX[(b & 0x0f) as usize];
    }
    Ok(String::from_utf8(out_bytes.to_vec()).unwrap())
}

#[cfg(test)]
mod tests {
    use super::canonical_hash_impl;

    #[test]
    fn canonical_hash_is_deterministic() {
        let grid = "
    #####
    #.@ #
    # $ #
    # . #
    #####
    ";
        let h1 = canonical_hash_impl(grid).unwrap();
        let h2 = canonical_hash_impl(grid).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn canonical_hash_ignores_rotation() {
        let grid = "
    #####
    #.@ #
    # $ #
    # . #
    #####
    ";
        let rotated_grid = "
    #####
    # . #
    # $ #
    # @.#
    #####
    ";
        assert_eq!(
            canonical_hash_impl(grid).unwrap(),
            canonical_hash_impl(rotated_grid).unwrap()
        );
    }

    #[test]
    fn canonical_hash_ignores_mirror() {
        let grid = "
    #####
    #.@ #
    # $ #
    # . #
    #####
    ";
        let mirrored_grid = "
    #####
    # @.#
    # $ #
    # . #
    #####
    ";
        assert_eq!(
            canonical_hash_impl(grid).unwrap(),
            canonical_hash_impl(mirrored_grid).unwrap()
        );
    }

    #[test]
    fn canonical_hash_ignores_rotation_and_mirror_combined() {
        let grid = "
    #####
    #.@ #
    # $ #
    # . #
    #####
    ";
        let transformed = "
    #####
    # . #
    # $ #
    #.@ #
    #####
    ";
        assert_eq!(
            canonical_hash_impl(grid).unwrap(),
            canonical_hash_impl(transformed).unwrap()
        );
    }

    #[test]
    fn canonical_hash_ignores_reachable_player_position() {
        let a = "
    #######
    #.@   #
    # ### #
    #     #
    # ### #
    #   $ #
    #######
    ";
        let b = "
    #######
    #.    #
    # ### #
    #  @  #
    # ### #
    #   $ #
    #######
    ";
        assert_eq!(
            canonical_hash_impl(a).unwrap(),
            canonical_hash_impl(b).unwrap()
        );
    }

    #[test]
    fn canonical_hash_does_not_ignore_unreachable_player_position() {
        let a = "
    #######
    #.    #
    #.    #
    #$##$##
    #  @  #
    #     #
    #######
    ";
        let b = "
    #######
    #.    #
    #.  @ #
    #$##$##
    #     #
    #     #
    #######
    ";
        assert_ne!(
            canonical_hash_impl(a).unwrap(),
            canonical_hash_impl(b).unwrap()
        );
    }
}
