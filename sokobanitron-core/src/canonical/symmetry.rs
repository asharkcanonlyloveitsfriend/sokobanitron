use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub(crate) struct FlatGrid<'a> {
    pub(crate) h: usize,
    pub(crate) w: usize,
    pub(crate) cells: &'a [u8],
}

impl<'a> FlatGrid<'a> {
    #[inline]
    pub(crate) fn debug_assert_invariants(&self) {
        debug_assert_eq!(self.cells.len(), self.h * self.w);
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Variant {
    pub(crate) rot: u8,
    pub(crate) mirror: bool,
}

pub(crate) const VARIANTS: [Variant; 8] = [
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

pub(crate) fn compare_variant_keys(base: FlatGrid<'_>, a: Variant, b: Variant) -> Ordering {
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

pub(crate) fn build_variant_grid(base: FlatGrid<'_>, v: Variant) -> (usize, usize, Vec<u8>) {
    let (h, w) = variant_dims(base, v);
    let mut out = Vec::with_capacity(h * w);
    for r in 0..h {
        for c in 0..w {
            out.push(variant_at(base, v, r, c));
        }
    }
    (h, w, out)
}
