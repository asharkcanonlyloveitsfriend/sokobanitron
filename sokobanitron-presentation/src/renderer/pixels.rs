#[allow(clippy::too_many_arguments)]
pub(crate) fn fill_rect(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    color: u8,
) {
    let start_x = x.max(0) as u32;
    let start_y = y.max(0) as u32;
    let end_x = (x + w as i32).min(frame_width as i32).max(0) as u32;
    let end_y = (y + h as i32).min(frame_height as i32).max(0) as u32;
    if start_x >= end_x || start_y >= end_y {
        return;
    }
    for py in start_y..end_y {
        for px in start_x..end_x {
            let idx = (py * frame_width + px) as usize;
            frame[idx] = color;
        }
    }
}

/// Blits a gray+alpha source buffer into the 8-bit grayscale destination frame.
///
/// Source pixels are byte pairs: `[premultiplied_gray, alpha]`. The gray byte must already be
/// premultiplied by the alpha byte before calling this blitter. For straight gray+alpha data,
/// premultiply the source buffer first or use the straight-alpha per-pixel helpers when composing
/// individual pixels.
#[allow(clippy::too_many_arguments)]
pub(crate) fn blit_premultiplied_gray_alpha(
    frame: &mut [u8],
    frame_width: u32,
    frame_height: u32,
    src: &[u8],
    src_width: u32,
    src_height: u32,
    dst_x: i32,
    dst_y: i32,
) {
    for sy in 0..src_height {
        let dy = dst_y + sy as i32;
        if dy < 0 || dy >= frame_height as i32 {
            continue;
        }
        for sx in 0..src_width {
            let dx = dst_x + sx as i32;
            if dx < 0 || dx >= frame_width as i32 {
                continue;
            }

            let src_idx = ((sy * src_width + sx) * 2) as usize;
            let dst_idx = ((dy as u32) * frame_width + (dx as u32)) as usize;

            let src_gray = src[src_idx];
            let src_a = src[src_idx + 1];
            if src_a == 0 {
                continue;
            }
            frame[dst_idx] = composite_premultiplied_gray_over(frame[dst_idx], src_gray, src_a);
        }
    }
}

/// Converts RGB channels to an 8-bit grayscale/luma value and ignores alpha.
///
/// Alpha handling is always the caller's responsibility. If `color[0..3]` are straight RGB
/// channels, the result is straight grayscale. If those RGB channels are already premultiplied by
/// alpha, the result is premultiplied grayscale too, because this luma transform is linear in the
/// RGB channels.
#[inline]
pub(crate) fn rgba_to_gray(color: [u8; 4]) -> u8 {
    let r = color[0] as u16;
    let g = color[1] as u16;
    let b = color[2] as u16;
    ((77 * r + 150 * g + 29 * b) >> 8) as u8
}

/// Converts a straight grayscale source value plus alpha into premultiplied grayscale.
#[inline]
pub(crate) fn premultiply_straight_gray(gray: u8, alpha: u8) -> u8 {
    (u16::from(gray) * u16::from(alpha) / 255) as u8
}

/// Composites a premultiplied grayscale source over an opaque grayscale destination.
///
/// `src_gray_premultiplied` must already have `src_alpha` applied.
#[inline]
pub(crate) fn composite_premultiplied_gray_over(
    dst_gray: u8,
    src_gray_premultiplied: u8,
    src_alpha: u8,
) -> u8 {
    let inv_alpha = 255_u16 - u16::from(src_alpha);
    (u16::from(src_gray_premultiplied) + (u16::from(dst_gray) * inv_alpha) / 255).min(255) as u8
}

/// Composites a straight grayscale source plus alpha over an opaque grayscale destination.
///
/// This is the helper to use when the source gray value has not yet been premultiplied.
#[inline]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn composite_straight_gray_alpha_over(dst_gray: u8, src_gray: u8, src_alpha: u8) -> u8 {
    composite_premultiplied_gray_over(
        dst_gray,
        premultiply_straight_gray(src_gray, src_alpha),
        src_alpha,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        blit_premultiplied_gray_alpha, composite_premultiplied_gray_over,
        composite_straight_gray_alpha_over, fill_rect, premultiply_straight_gray,
    };

    #[test]
    fn blit_premultiplied_gray_alpha_composites_premultiplied_source_without_edge_darkening() {
        let mut frame = vec![162];
        let src = vec![81, 128];

        blit_premultiplied_gray_alpha(&mut frame, 1, 1, &src, 1, 1, 0, 0);

        assert_eq!(frame, vec![161]);
    }

    #[test]
    fn blit_premultiplied_gray_alpha_composites_semitransparent_source_over_destination() {
        let mut frame = vec![100];
        let src = vec![100, 128];

        blit_premultiplied_gray_alpha(&mut frame, 1, 1, &src, 1, 1, 0, 0);

        assert_eq!(frame, vec![149]);
    }

    #[test]
    fn fill_rect_replaces_destination_with_gray_color() {
        let mut frame = vec![100];

        fill_rect(&mut frame, 1, 1, 0, 0, 1, 1, 200);

        assert_eq!(frame, vec![200]);
    }

    #[test]
    fn gray_alpha_pixel_helper_matches_premultiplied_composite() {
        let src = premultiply_straight_gray(200, 128);

        assert_eq!(
            composite_straight_gray_alpha_over(100, 200, 128),
            composite_premultiplied_gray_over(100, src, 128)
        );
    }
}
