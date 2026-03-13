use resvg::{render, tiny_skia, usvg};

pub(crate) fn rasterize_svg(svg: &str, size: u32) -> Vec<u8> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg, &options).expect("failed to parse svg");
    let mut pixmap = tiny_skia::Pixmap::new(size, size).expect("failed to allocate pixmap");
    render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.data().to_vec()
}
