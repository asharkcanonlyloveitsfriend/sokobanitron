use resvg::{render, tiny_skia, usvg};

use crate::renderer::rgba_to_gray;

/// Rasterizes SVG assets into premultiplied gray+alpha pixels for the grayscale renderer.
///
/// Returned pixels are byte pairs: `[premultiplied_gray, alpha]`. `resvg`/`tiny-skia` provide
/// premultiplied RGBA, and converting those premultiplied RGB channels with `rgba_to_gray` keeps
/// the gray channel premultiplied.
pub(crate) fn rasterize_svg(svg: &str, size: u32) -> Vec<u8> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &options).expect("failed to parse svg");
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("failed to allocate pixmap");
    render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap
        .data()
        .chunks_exact(4)
        .flat_map(|pixel| {
            let gray = rgba_to_gray([pixel[0], pixel[1], pixel[2], pixel[3]]);
            [gray, pixel[3]]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::rasterize_svg;
    use crate::renderer::blit_premultiplied_gray_alpha;

    #[test]
    fn rasterize_svg_returns_premultiplied_gray_alpha_pixels() {
        let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1' viewBox='0 0 1 1'>\
             <rect x='0' y='0' width='1' height='1' fill='rgba(255,0,0,0.5)'/>\
             </svg>";

        let pixels = rasterize_svg(svg, 1);

        assert_eq!(pixels, vec![38, 128]);
    }

    #[test]
    fn rasterized_svg_blits_as_premultiplied_gray_alpha() {
        let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1' viewBox='0 0 1 1'>\
             <rect x='0' y='0' width='1' height='1' fill='rgba(255,0,0,0.5)'/>\
             </svg>";
        let pixels = rasterize_svg(svg, 1);
        let mut frame = vec![100];

        blit_premultiplied_gray_alpha(&mut frame, 1, 1, &pixels, 1, 1, 0, 0);

        assert_eq!(frame, vec![87]);
    }
}
