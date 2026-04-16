use jni::JNIEnv;
use jni::objects::JObject;
use presentation::ScreenRect;

#[cfg(target_os = "android")]
mod imp {
    use super::{JNIEnv, JObject, ScreenRect};
    use jni::sys::jobject;
    use std::ffi::c_void;
    use std::mem::MaybeUninit;
    use std::ptr::{self, NonNull};

    const WINDOW_FORMAT_RGBA_8888: i32 = 1;

    #[repr(C)]
    struct ANativeWindow {
        _private: [u8; 0],
    }

    #[repr(C)]
    struct ARect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    struct ANativeWindowBuffer {
        width: i32,
        height: i32,
        stride: i32,
        format: i32,
        bits: *mut c_void,
        reserved: [u32; 6],
    }

    #[link(name = "android")]
    unsafe extern "C" {
        fn ANativeWindow_fromSurface(
            env: *mut jni::sys::JNIEnv,
            surface: jobject,
        ) -> *mut ANativeWindow;
        fn ANativeWindow_release(window: *mut ANativeWindow);
        fn ANativeWindow_setBuffersGeometry(
            window: *mut ANativeWindow,
            width: i32,
            height: i32,
            format: i32,
        ) -> i32;
        fn ANativeWindow_lock(
            window: *mut ANativeWindow,
            out_buffer: *mut ANativeWindowBuffer,
            dirty_bounds: *mut ARect,
        ) -> i32;
        fn ANativeWindow_unlockAndPost(window: *mut ANativeWindow) -> i32;
    }

    pub struct NativeWindow {
        raw: NonNull<ANativeWindow>,
    }

    impl NativeWindow {
        pub fn from_surface(env: &JNIEnv, surface: &JObject) -> Option<Self> {
            let raw = unsafe { ANativeWindow_fromSurface(env.get_raw(), surface.as_raw()) };
            NonNull::new(raw).map(|raw| Self { raw })
        }

        pub fn configure(&mut self, width: u32, height: u32) -> bool {
            unsafe {
                ANativeWindow_setBuffersGeometry(
                    self.raw.as_ptr(),
                    i32::try_from(width).expect("surface width should fit i32"),
                    i32::try_from(height).expect("surface height should fit i32"),
                    WINDOW_FORMAT_RGBA_8888,
                ) == 0
            }
        }

        pub fn present_gray(&mut self, gray: &[u8], width: u32, height: u32) -> bool {
            self.present_gray_region_inner(gray, width, height, None)
        }

        pub fn present_gray_region(
            &mut self,
            gray: &[u8],
            width: u32,
            height: u32,
            region: ScreenRect,
        ) -> bool {
            self.present_gray_region_inner(gray, width, height, Some(region))
        }

        fn present_gray_region_inner(
            &mut self,
            gray: &[u8],
            width: u32,
            height: u32,
            dirty_region: Option<ScreenRect>,
        ) -> bool {
            let width_px = usize::try_from(width).expect("surface width should fit usize");
            let height_px = usize::try_from(height).expect("surface height should fit usize");
            let expected_len = width_px
                .checked_mul(height_px)
                .expect("surface dimensions should fit staging buffer");
            assert_eq!(
                gray.len(),
                expected_len,
                "staging gray frame should match the Android surface size",
            );
            self.present_staged(gray, width_px, height_px, dirty_region)
        }

        fn present_staged(
            &mut self,
            staging_gray: &[u8],
            width_px: usize,
            height_px: usize,
            dirty_region: Option<ScreenRect>,
        ) -> bool {
            let Some((buffer, bits, locked_dirty_region)) = self.lock_buffer(dirty_region) else {
                return false;
            };

            let stride_pixels = usize::try_from(buffer.stride)
                .expect("native window stride should be non-negative");
            let buffer_height = usize::try_from(buffer.height)
                .expect("native window height should be non-negative");
            if buffer.format != WINDOW_FORMAT_RGBA_8888
                || stride_pixels < width_px
                || buffer_height < height_px
            {
                let _ = unsafe { ANativeWindow_unlockAndPost(self.raw.as_ptr()) };
                return false;
            }

            let copy_region = locked_dirty_region.unwrap_or(ScreenRect {
                x: 0,
                y: 0,
                w: width_px as u32,
                h: height_px as u32,
            });
            let left = copy_region.x.min(width_px as u32) as usize;
            let top = copy_region.y.min(height_px as u32) as usize;
            let right = copy_region
                .x
                .saturating_add(copy_region.w)
                .min(width_px as u32) as usize;
            let bottom = copy_region
                .y
                .saturating_add(copy_region.h)
                .min(height_px as u32) as usize;
            let stride_bytes = stride_pixels.saturating_mul(4);
            for row in top..bottom {
                let src_offset = row.saturating_mul(width_px);
                let dst_offset = row.saturating_mul(stride_bytes);
                for col in left..right {
                    let gray = staging_gray[src_offset + col];
                    let dst = dst_offset + col * 4;
                    unsafe {
                        let out = bits.as_ptr().add(dst);
                        *out = gray;
                        *out.add(1) = gray;
                        *out.add(2) = gray;
                        *out.add(3) = 255;
                    }
                }
            }

            unsafe { ANativeWindow_unlockAndPost(self.raw.as_ptr()) == 0 }
        }

        fn lock_buffer(
            &mut self,
            dirty_region: Option<ScreenRect>,
        ) -> Option<(ANativeWindowBuffer, NonNull<u8>, Option<ScreenRect>)> {
            let mut buffer = MaybeUninit::<ANativeWindowBuffer>::uninit();
            let mut dirty_rect = dirty_region.map(screen_rect_to_arect);
            let dirty_rect_ptr = dirty_rect
                .as_mut()
                .map_or(ptr::null_mut(), |rect| rect as *mut ARect);
            let lock_result = unsafe {
                ANativeWindow_lock(self.raw.as_ptr(), buffer.as_mut_ptr(), dirty_rect_ptr)
            };
            if lock_result != 0 {
                return None;
            }

            let buffer = unsafe { buffer.assume_init() };
            let Some(bits) = NonNull::new(buffer.bits.cast::<u8>()) else {
                let _ = unsafe { ANativeWindow_unlockAndPost(self.raw.as_ptr()) };
                return None;
            };
            Some((buffer, bits, dirty_rect.and_then(arect_to_screen_rect)))
        }
    }

    fn screen_rect_to_arect(rect: ScreenRect) -> ARect {
        ARect {
            left: rect.x.min(i32::MAX as u32) as i32,
            top: rect.y.min(i32::MAX as u32) as i32,
            right: rect.x.saturating_add(rect.w).min(i32::MAX as u32) as i32,
            bottom: rect.y.saturating_add(rect.h).min(i32::MAX as u32) as i32,
        }
    }

    fn arect_to_screen_rect(rect: ARect) -> Option<ScreenRect> {
        let left = rect.left.max(0) as u32;
        let top = rect.top.max(0) as u32;
        let right = rect.right.max(rect.left).max(0) as u32;
        let bottom = rect.bottom.max(rect.top).max(0) as u32;
        if left >= right || top >= bottom {
            return None;
        }
        Some(ScreenRect {
            x: left,
            y: top,
            w: right - left,
            h: bottom - top,
        })
    }

    impl Drop for NativeWindow {
        fn drop(&mut self) {
            unsafe {
                ANativeWindow_release(self.raw.as_ptr());
            }
        }
    }
}

#[cfg(not(target_os = "android"))]
mod imp {
    use super::{JNIEnv, JObject, ScreenRect};

    pub struct NativeWindow;

    impl NativeWindow {
        pub fn from_surface(_env: &JNIEnv, _surface: &JObject) -> Option<Self> {
            None
        }

        pub fn configure(&mut self, _width: u32, _height: u32) -> bool {
            false
        }

        pub fn present_gray(&mut self, _gray: &[u8], _width: u32, _height: u32) -> bool {
            false
        }

        pub fn present_gray_region(
            &mut self,
            _gray: &[u8],
            _width: u32,
            _height: u32,
            _region: ScreenRect,
        ) -> bool {
            false
        }
    }
}

pub use imp::NativeWindow;
