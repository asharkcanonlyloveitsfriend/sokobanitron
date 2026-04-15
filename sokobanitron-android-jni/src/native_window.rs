use jni::JNIEnv;
use jni::objects::JObject;

#[cfg(target_os = "android")]
mod imp {
    use super::{JNIEnv, JObject};
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
                    i32::try_from(width).unwrap_or(1),
                    i32::try_from(height).unwrap_or(1),
                    WINDOW_FORMAT_RGBA_8888,
                ) == 0
            }
        }

        pub fn present_gray(&mut self, gray: &[u8], width: u32, height: u32) -> bool {
            let width_px = usize::try_from(width).unwrap_or(0);
            let height_px = usize::try_from(height).unwrap_or(0);
            let Some(expected_len) = width_px.checked_mul(height_px) else {
                return false;
            };
            if gray.len() < expected_len {
                return false;
            }
            self.present_staged(gray, width_px, height_px)
        }

        fn present_staged(
            &mut self,
            staging_gray: &[u8],
            width_px: usize,
            height_px: usize,
        ) -> bool {
            let Some((buffer, bits)) = self.lock_buffer() else {
                return false;
            };

            let stride_pixels = usize::try_from(buffer.stride).unwrap_or(0);
            let buffer_height = usize::try_from(buffer.height).unwrap_or(0);
            if buffer.format != WINDOW_FORMAT_RGBA_8888
                || stride_pixels < width_px
                || buffer_height < height_px
            {
                let _ = unsafe { ANativeWindow_unlockAndPost(self.raw.as_ptr()) };
                return false;
            }

            let stride_bytes = stride_pixels.saturating_mul(4);
            for row in 0..height_px {
                let src_offset = row.saturating_mul(width_px);
                let dst_offset = row.saturating_mul(stride_bytes);
                for col in 0..width_px {
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

        fn lock_buffer(&mut self) -> Option<(ANativeWindowBuffer, NonNull<u8>)> {
            let mut buffer = MaybeUninit::<ANativeWindowBuffer>::uninit();
            let lock_result = unsafe {
                ANativeWindow_lock(self.raw.as_ptr(), buffer.as_mut_ptr(), ptr::null_mut())
            };
            if lock_result != 0 {
                return None;
            }

            let buffer = unsafe { buffer.assume_init() };
            let Some(bits) = NonNull::new(buffer.bits.cast::<u8>()) else {
                let _ = unsafe { ANativeWindow_unlockAndPost(self.raw.as_ptr()) };
                return None;
            };
            Some((buffer, bits))
        }
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
    use super::{JNIEnv, JObject};

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
    }
}

pub use imp::NativeWindow;
