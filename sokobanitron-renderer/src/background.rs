use crate::Renderer;
use image::{ImageBuffer, Rgba, RgbaImage, imageops};

impl Renderer {
    pub(crate) fn ensure_cached_background(&mut self, width: u32, height: u32) {
        if self.cached_width == width && self.cached_height == height {
            return;
        }

        let src_w = self.source_background.width();
        let src_h = self.source_background.height();
        let view_aspect = width as f32 / height as f32;
        let src_aspect = src_w as f32 / src_h as f32;

        let (crop_x, crop_y, crop_w, crop_h) = if src_aspect > view_aspect {
            let crop_w = ((src_h as f32) * view_aspect)
                .round()
                .clamp(1.0, src_w as f32) as u32;
            let crop_x = ((src_w - crop_w) as f32 / 2.0).round().max(0.0) as u32;
            (crop_x, 0, crop_w, src_h)
        } else {
            let crop_h = ((src_w as f32) / view_aspect)
                .round()
                .clamp(1.0, src_h as f32) as u32;
            let crop_y = ((src_h - crop_h) as f32 / 2.0).round().max(0.0) as u32;
            (0, crop_y, src_w, crop_h)
        };

        let cropped: RgbaImage =
            imageops::crop_imm(&self.source_background, crop_x, crop_y, crop_w, crop_h).to_image();
        let resized: ImageBuffer<Rgba<u8>, Vec<u8>> =
            imageops::resize(&cropped, width, height, imageops::FilterType::Triangle);

        self.cached_background = resized.into_raw();
        self.cached_width = width;
        self.cached_height = height;
    }
}
