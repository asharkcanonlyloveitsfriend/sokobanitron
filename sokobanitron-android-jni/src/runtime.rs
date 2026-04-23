use crate::native_window::NativeWindow;
use sokobanitron_app::{
    app::{
        AppFramePresenter, AppFrameRenderer, AppPointerInput, AppState, FrameDamage,
        GameplayAnimationPolicy, RendererOverrides, SharedAppRuntime,
    },
    editor::{set_editor_double_tap_window, set_editor_touch_slop},
    gameplay::set_gameplay_touch_slop,
    level_bootstrap::load_initial_levels_for_app,
    shared::PointerPhase,
};
use std::io;
use std::path::Path;
use std::time::Duration;

const ANDROID_GAMEPLAY_TAP_SLOP_PX: i32 = 24;
const ANDROID_EDITOR_TAP_SLOP_PX: i32 = 24;
const ANDROID_EDITOR_DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(750);

pub struct AndroidApp {
    runtime: SharedAppRuntime,
    pending_present_damage: FrameDamage,
    native_window: Option<NativeWindow>,
}

impl AndroidApp {
    pub fn new(
        level_sets_root: &Path,
        surface_width: u32,
        surface_height: u32,
    ) -> io::Result<Self> {
        let surface_width = surface_width.max(1);
        let surface_height = surface_height.max(1);
        let initial_levels = load_initial_levels_for_app(level_sets_root)?;
        let app_state = AppState {
            editor_available: true,
            supports_multi_touch: true,
            ..AppState::default()
        };
        let mut runtime = SharedAppRuntime::new(
            initial_levels,
            app_state,
            surface_width,
            surface_height,
            AppFrameRenderer::with_renderer_overrides_and_gameplay_animation_policy(
                android_renderer_overrides(),
                GameplayAnimationPolicy::Full,
            ),
        );
        set_gameplay_touch_slop(
            &mut runtime.app_state_mut().gameplay,
            ANDROID_GAMEPLAY_TAP_SLOP_PX,
        );
        set_editor_touch_slop(runtime.app_state_mut(), ANDROID_EDITOR_TAP_SLOP_PX);
        set_editor_double_tap_window(runtime.app_state_mut(), ANDROID_EDITOR_DOUBLE_TAP_WINDOW);
        let mut app = Self {
            runtime,
            pending_present_damage: FrameDamage::Noop,
            native_window: None,
        };
        app.render_full_current_request();
        Ok(app)
    }

    pub fn resize(&mut self, surface_width: u32, surface_height: u32) {
        self.runtime.resize_surface(surface_width, surface_height);
        self.configure_native_window();
        self.render_full_current_request();
    }

    pub fn handle_pointer_event(&mut self, id: u64, phase: PointerPhase, x: f64, y: f64) -> bool {
        let _ = self.handle_pointer_input_and_render(AppPointerInput::Pointer { id, phase, x, y });
        self.has_pending_render_work()
    }

    pub fn set_native_window(&mut self, native_window: Option<NativeWindow>) {
        self.native_window = native_window;
        self.configure_native_window();
        if self.native_window.is_some() {
            self.mark_frame_damage(FrameDamage::Full);
        }
    }

    pub fn present_frame(&mut self) -> bool {
        if matches!(self.pending_present_damage, FrameDamage::Noop)
            && self.runtime.has_pending_render_work()
        {
            let _ = self.continue_pending_render_work_and_render();
        }

        if !self.has_pending_render_work() {
            return true;
        }
        let Some(mut window) = self.native_window.take() else {
            return false;
        };
        let surface_width = self.runtime.surface_width();
        let surface_height = self.runtime.surface_height();
        let damage = self.pending_present_damage;
        let presented = match damage {
            FrameDamage::Full => {
                window.present_gray(self.runtime.gray_frame(), surface_width, surface_height)
            }
            FrameDamage::Noop => true,
            FrameDamage::Region(region) => window.present_gray_region(
                self.runtime.gray_frame(),
                surface_width,
                surface_height,
                region,
            ),
        };
        self.native_window = Some(window);
        if presented {
            self.pending_present_damage = FrameDamage::Noop;
        }
        presented
    }

    pub fn has_pending_render_work(&mut self) -> bool {
        !matches!(self.pending_present_damage, FrameDamage::Noop)
            || self.runtime.has_pending_render_work()
    }

    fn render_full_current_request(&mut self) {
        let mut presenter = AndroidFramePresenter {
            pending_present_damage: &mut self.pending_present_damage,
        };
        let _ = self.runtime.render_full_current_frame(&mut presenter);
    }

    fn mark_frame_damage(&mut self, damage: FrameDamage) {
        self.pending_present_damage = self.pending_present_damage.merge(damage);
    }

    fn configure_native_window(&mut self) {
        if let Some(window) = self.native_window.as_mut()
            && !window.configure(self.runtime.surface_width(), self.runtime.surface_height())
        {
            #[cfg(debug_assertions)]
            eprintln!(
                "warning: failed to configure Android native window for {}x{}",
                self.runtime.surface_width(),
                self.runtime.surface_height()
            );
        }
    }

    fn handle_pointer_input_and_render(&mut self, input: AppPointerInput) -> Result<(), ()> {
        let mut presenter = AndroidFramePresenter {
            pending_present_damage: &mut self.pending_present_damage,
        };
        self.runtime
            .handle_pointer_input_and_render(input, &mut presenter)
            .map(|_| ())
    }

    fn continue_pending_render_work_and_render(&mut self) -> Result<(), ()> {
        let mut presenter = AndroidFramePresenter {
            pending_present_damage: &mut self.pending_present_damage,
        };
        self.runtime
            .continue_pending_render_work_and_render(&mut presenter)
            .map(|_| ())
    }
}

struct AndroidFramePresenter<'a> {
    pending_present_damage: &'a mut FrameDamage,
}

impl AppFramePresenter for AndroidFramePresenter<'_> {
    type Error = ();

    fn present_frame(
        &mut self,
        damage: FrameDamage,
        _gray_frame: &[u8],
        _width: u32,
        _height: u32,
    ) -> Result<(), Self::Error> {
        *self.pending_present_damage = (*self.pending_present_damage).merge(damage);
        Ok(())
    }
}

fn android_renderer_overrides() -> RendererOverrides {
    RendererOverrides {
        gray_1: Some(236),
        gray_2: Some(224),
        gray_3: Some(212),
        gray_4: Some(200),
        gray_5: Some(190),
        gray_6: Some(180),
        gray_7: Some(170),
        gray_8: Some(158),
        gray_9: Some(146),
        gray_10: Some(134),
        gray_11: Some(122),
        gray_12: Some(112),
        gray_13: Some(102),
        gray_14: Some(90),
    }
}
