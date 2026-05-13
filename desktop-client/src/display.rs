use crate::app_driver::App;
use pixels::Pixels;
use sokobanitron_app::app::{
    AppFramePresenter, AppInput, AppPointerInput, AppliedUpdate, FrameDamage, RenderWorkResult,
};
use std::time::Instant;

impl App {
    pub(crate) fn apply_input_and_render(&mut self, input: AppInput) -> Result<AppliedUpdate, ()> {
        let mut presenter = DesktopFramePresenter {
            pixels: &mut self.pixels,
            presented_frame_time: None,
        };
        self.runtime.apply_input_and_render(input, &mut presenter)
    }

    pub(crate) fn handle_pointer_input_and_render(
        &mut self,
        input: AppPointerInput,
    ) -> Result<RenderWorkResult, ()> {
        let mut presenter = DesktopFramePresenter {
            pixels: &mut self.pixels,
            presented_frame_time: None,
        };
        self.runtime
            .handle_pointer_input_and_render(input, &mut presenter)
    }

    pub(crate) fn continue_pending_render_work_and_render(
        &mut self,
    ) -> Result<RenderWorkResult, ()> {
        let mut presenter = DesktopFramePresenter {
            pixels: &mut self.pixels,
            presented_frame_time: None,
        };
        self.runtime
            .continue_pending_render_work_and_render(&mut presenter)
    }

    pub(crate) fn render_current(&mut self) {
        let mut presenter = DesktopFramePresenter {
            pixels: &mut self.pixels,
            presented_frame_time: None,
        };
        let _ = self.runtime.render_current_frame(&mut presenter);
    }
}

struct DesktopFramePresenter<'a> {
    pixels: &'a mut Option<Pixels<'static>>,
    presented_frame_time: Option<Instant>,
}

impl AppFramePresenter for DesktopFramePresenter<'_> {
    type Error = ();

    fn present_frame(
        &mut self,
        _damage: FrameDamage,
        gray_frame: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<(), Self::Error> {
        if let Some(pixels) = self.pixels.as_mut() {
            copy_gray_to_sepia_rgba(gray_frame, pixels.frame_mut());
            pixels.render().expect("render");
            self.presented_frame_time = Some(Instant::now());
        }
        Ok(())
    }

    fn presented_frame_time(&self) -> Option<Instant> {
        self.presented_frame_time
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
