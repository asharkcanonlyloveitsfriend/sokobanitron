use crate::app_driver::App;
use sokobanitron_app::app::{FrameRequest, FrameSink};

impl App {
    pub(crate) fn render_current(&mut self) {
        let request = self.runtime.current_frame_request();
        let _ = self.render_request(&request);
    }

    pub(crate) fn render_pending_visible_presentation(&mut self) {
        if let Some(pixels) = &mut self.pixels {
            self.runtime.draw_pending_visible_presentation();
            copy_gray_to_sepia_rgba(self.runtime.gray_frame(), pixels.frame_mut());
            pixels.render().expect("render");
        }
    }

    fn render_request(&mut self, request: &FrameRequest) -> Result<(), ()> {
        if let Some(pixels) = &mut self.pixels {
            self.runtime.draw_frame_request(request);
            copy_gray_to_sepia_rgba(self.runtime.gray_frame(), pixels.frame_mut());
            pixels.render().expect("render");
        }
        Ok(())
    }
}

fn copy_gray_to_sepia_rgba(gray: &[u8], rgba: &mut [u8]) {
    for (src, dst) in gray.iter().zip(rgba.chunks_exact_mut(4)) {
        let [red, green, blue] = sepia_from_gray(*src);
        dst[0] = red;
        dst[1] = green;
        dst[2] = blue;
        dst[3] = 255;
    }
}

fn sepia_from_gray(gray: u8) -> [u8; 3] {
    [
        scale_sepia_channel(gray, 1220),
        scale_sepia_channel(gray, 900),
        scale_sepia_channel(gray, 680),
    ]
}

fn scale_sepia_channel(gray: u8, scale_per_thousand: u16) -> u8 {
    ((u32::from(gray) * u32::from(scale_per_thousand)) / 1000).min(255) as u8
}

impl FrameSink for App {
    type Error = ();

    fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error> {
        self.render_request(request)
    }
}

#[cfg(test)]
mod tests {
    use super::{copy_gray_to_sepia_rgba, sepia_from_gray};

    #[test]
    fn sepia_conversion_tints_gray_values() {
        assert_eq!(sepia_from_gray(0), [0, 0, 0]);
        assert_eq!(sepia_from_gray(128), [156, 115, 87]);
        assert_eq!(sepia_from_gray(255), [255, 229, 173]);
    }

    #[test]
    fn copy_gray_to_sepia_rgba_writes_opaque_pixels() {
        let gray = [0, 128, 255];
        let mut rgba = [0; 12];

        copy_gray_to_sepia_rgba(&gray, &mut rgba);

        assert_eq!(rgba, [0, 0, 0, 255, 156, 115, 87, 255, 255, 229, 173, 255]);
    }
}
