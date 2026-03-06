use crate::error::CoreError;

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

pub(crate) fn canonicalize_player_start_in_place(
    cells: &mut [u8],
    h: usize,
    w: usize,
) -> Result<(), CoreError> {
    let (sr, sc) = find_player_start(cells, h, w).ok_or(CoreError::PlayerStartNotFound)?;

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

    let (cr, cc) = canonical_pos.ok_or(CoreError::PlayerStartNotFound)?;

    let si = sr * w + sc;
    cells[si] = if cells[si] == b'+' { b'.' } else { b' ' };
    let ci = cr * w + cc;
    cells[ci] = if cells[ci] == b'.' { b'+' } else { b'@' };
    Ok(())
}
