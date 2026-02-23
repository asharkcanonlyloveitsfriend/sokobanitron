use std::cmp::Ordering;
use std::time::Instant;

use sha2::{Digest, Sha256};

use crate::normalize_to_walkable_region::normalize_to_walkable_region_lines;
use crate::stage_profile;
use crate::CoreError;

#[derive(Clone, Copy)]
struct FlatGrid<'a> {
    h: usize,
    w: usize,
    cells: &'a [u8],
}

#[derive(Clone, Copy)]
struct Variant {
    rot: u8,     // 0..=3 clockwise rotations
    mirror: bool, // mirror horizontally after rotation
}

fn split_grid_lines(grid: &str) -> Vec<String> {
    grid.trim_matches('\n')
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect()
}

#[inline]
fn normalize_key_byte(ch: u8) -> u8 {
    match ch {
        b'@' => b' ',
        b'+' => b'.',
        _ => ch,
    }
}

#[inline]
fn variant_dims(base: FlatGrid<'_>, v: Variant) -> (usize, usize) {
    if v.rot % 2 == 0 {
        (base.h, base.w)
    } else {
        (base.w, base.h)
    }
}

#[inline]
fn base_at(base: FlatGrid<'_>, r: usize, c: usize) -> u8 {
    base.cells[r * base.w + c]
}

#[inline]
fn variant_at(base: FlatGrid<'_>, v: Variant, r: usize, c: usize) -> u8 {
    let (vh, vw) = variant_dims(base, v);
    debug_assert!(r < vh && c < vw);

    let c2 = if v.mirror { vw - 1 - c } else { c };
    let (br, bc) = match v.rot {
        0 => (r, c2),
        1 => (base.h - 1 - c2, r),
        2 => (base.h - 1 - r, base.w - 1 - c2),
        3 => (c2, base.w - 1 - r),
        _ => unreachable!(),
    };
    base_at(base, br, bc)
}

fn compare_variant_keys(base: FlatGrid<'_>, a: Variant, b: Variant) -> Ordering {
    let (ah, aw) = variant_dims(base, a);
    let (bh, bw) = variant_dims(base, b);

    let a_total = ah * aw + ah.saturating_sub(1);
    let b_total = bh * bw + bh.saturating_sub(1);
    let total = a_total.min(b_total);

    for i in 0..total {
        let a_is_newline = ah > 0 && i % (aw + 1) == aw;
        let b_is_newline = bh > 0 && i % (bw + 1) == bw;

        let ab = if a_is_newline {
            b'\n'
        } else {
            let r = i / (aw + 1);
            let c = i % (aw + 1);
            normalize_key_byte(variant_at(base, a, r, c))
        };
        let bb = if b_is_newline {
            b'\n'
        } else {
            let r = i / (bw + 1);
            let c = i % (bw + 1);
            normalize_key_byte(variant_at(base, b, r, c))
        };

        if ab != bb {
            return ab.cmp(&bb);
        }
    }

    a_total.cmp(&b_total)
}

fn build_variant_grid(base: FlatGrid<'_>, v: Variant) -> (usize, usize, Vec<u8>) {
    let (h, w) = variant_dims(base, v);
    let mut out = Vec::with_capacity(h * w);
    for r in 0..h {
        for c in 0..w {
            out.push(variant_at(base, v, r, c));
        }
    }
    (h, w, out)
}

fn find_player_start(cells: &[u8], h: usize, w: usize) -> Option<(usize, usize)> {
    for r in 0..h {
        let row_base = r * w;
        for c in 0..w {
            let ch = cells[row_base + c];
            if ch == b'@' || ch == b'+' {
                return Some((r, c));
            }
        }
    }
    None
}

fn canonicalize_player_start_in_place(cells: &mut [u8], h: usize, w: usize) -> Result<(), CoreError> {
    let (sr, sc) = find_player_start(cells, h, w)
        .ok_or_else(|| CoreError::PlayerStartNotFound)?;

    let size = h * w;
    let mut seen = vec![0u8; size];
    let mut stack = vec![(sr, sc)];
    seen[sr * w + sc] = 1;

    while let Some((r, c)) = stack.pop() {
        if r > 0 {
            let nr = r - 1;
            let nc = c;
            let i = nr * w + nc;
            if seen[i] == 0 {
                let ch = cells[i];
                if ch != b'#' && ch != b'$' && ch != b'*' {
                    seen[i] = 1;
                    stack.push((nr, nc));
                }
            }
        }
        if r + 1 < h {
            let nr = r + 1;
            let nc = c;
            let i = nr * w + nc;
            if seen[i] == 0 {
                let ch = cells[i];
                if ch != b'#' && ch != b'$' && ch != b'*' {
                    seen[i] = 1;
                    stack.push((nr, nc));
                }
            }
        }
        if c > 0 {
            let nr = r;
            let nc = c - 1;
            let i = nr * w + nc;
            if seen[i] == 0 {
                let ch = cells[i];
                if ch != b'#' && ch != b'$' && ch != b'*' {
                    seen[i] = 1;
                    stack.push((nr, nc));
                }
            }
        }
        if c + 1 < w {
            let nr = r;
            let nc = c + 1;
            let i = nr * w + nc;
            if seen[i] == 0 {
                let ch = cells[i];
                if ch != b'#' && ch != b'$' && ch != b'*' {
                    seen[i] = 1;
                    stack.push((nr, nc));
                }
            }
        }
    }

    let mut canonical_pos: Option<(usize, usize)> = None;
    for r in 0..h {
        let row_base = r * w;
        for c in 0..w {
            if seen[row_base + c] != 0 {
                canonical_pos = Some((r, c));
                break;
            }
        }
        if canonical_pos.is_some() {
            break;
        }
    }

    let (cr, cc) =
        canonical_pos.ok_or_else(|| CoreError::PlayerStartNotFound)?;

    let si = sr * w + sc;
    cells[si] = if cells[si] == b'+' { b'.' } else { b' ' };
    let ci = cr * w + cc;
    cells[ci] = if cells[ci] == b'.' { b'+' } else { b'@' };
    Ok(())
}

fn canonical_hash_impl(grid: &str) -> Result<String, CoreError> {
    let t0 = Instant::now();
    let split = split_grid_lines(grid);
    stage_profile::record("canonical.split_lines", t0.elapsed());

    let t1 = Instant::now();
    let normalized = normalize_to_walkable_region_lines(split);
    stage_profile::record("canonical.normalize", t1.elapsed());

    let t2 = Instant::now();
    let h = normalized.len();
    let w = if h == 0 { 0 } else { normalized[0].len() };
    let mut flat = Vec::with_capacity(h * w);
    for row in normalized {
        flat.extend_from_slice(row.as_bytes());
    }
    let base = FlatGrid {
        h,
        w,
        cells: &flat,
    };
    stage_profile::record("canonical.flatten_normalized", t2.elapsed());

    let variants = [
        Variant {
            rot: 0,
            mirror: false,
        },
        Variant {
            rot: 0,
            mirror: true,
        },
        Variant {
            rot: 1,
            mirror: false,
        },
        Variant {
            rot: 1,
            mirror: true,
        },
        Variant {
            rot: 2,
            mirror: false,
        },
        Variant {
            rot: 2,
            mirror: true,
        },
        Variant {
            rot: 3,
            mirror: false,
        },
        Variant {
            rot: 3,
            mirror: true,
        },
    ];

    let t3 = Instant::now();
    let mut best = variants[0];
    for &v in &variants[1..] {
        if compare_variant_keys(base, v, best) == Ordering::Less {
            best = v;
        }
    }
    stage_profile::record("canonical.choose_variant", t3.elapsed());

    let t4 = Instant::now();
    let (vh, vw, mut canonical_cells) = build_variant_grid(base, best);
    stage_profile::record("canonical.materialize_variant", t4.elapsed());

    let t5 = Instant::now();
    canonicalize_player_start_in_place(&mut canonical_cells, vh, vw)?;
    stage_profile::record("canonical.canonicalize_player", t5.elapsed());

    let t6 = Instant::now();
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
    let out = String::from_utf8(out_bytes.to_vec()).unwrap();
    stage_profile::record("canonical.sha256_hex", t6.elapsed());
    Ok(out)
}

pub fn canonical_hash(grid: &str) -> Result<String, CoreError> {
    canonical_hash_impl(grid)
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
        assert_eq!(canonical_hash_impl(a).unwrap(), canonical_hash_impl(b).unwrap());
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
        assert_ne!(canonical_hash_impl(a).unwrap(), canonical_hash_impl(b).unwrap());
    }
}
