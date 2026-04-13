use resvg::{render, tiny_skia, usvg};

/// Rasterizes SVG assets into premultiplied RGBA pixels, matching `tiny-skia`'s native pixmap
/// format. Callers should composite these pixels with a premultiplied-aware blitter.
pub(crate) fn rasterize_svg(svg: &str, size: u32) -> Vec<u8> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &options).expect("failed to parse svg");
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("failed to allocate pixmap");
    render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.data().to_vec()
}

#[cfg(test)]
mod tests {
    use super::rasterize_svg;

    #[test]
    fn rasterize_svg_returns_premultiplied_pixels() {
        let svg = "<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1' viewBox='0 0 1 1'>\
             <rect x='0' y='0' width='1' height='1' fill='rgba(255,0,0,0.5)'/>\
             </svg>";

        let pixels = rasterize_svg(svg, 1);

        assert_eq!(pixels, vec![128, 0, 0, 128]);
    }
}
