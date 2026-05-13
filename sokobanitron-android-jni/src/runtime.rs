use crate::native_window::NativeWindow;
use sokobanitron_app::{
    app::{
        AppFramePresenter, AppPointerInput, FrameDamage, GameplayAnimationPolicy,
        RendererOverrides, SharedAppRendererConfig, SharedAppRuntime, SharedAppRuntimeConfig,
    },
    level_bootstrap::load_initial_levels_for_app,
    shared::PointerPhase,
};
use std::io;
use std::path::Path;
use std::time::{Duration, Instant};

const ANDROID_GAMEPLAY_TAP_SLOP_PX: i32 = 24;
const ANDROID_EDITOR_TAP_SLOP_PX: i32 = 24;
const ANDROID_EDITOR_DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(750);

pub struct AndroidApp {
    runtime: SharedAppRuntime,
    pending_present_damage: FrameDamage,
    pending_presentation_frame_for_present: bool,
    native_window: Option<NativeWindow>,
    frame_clock: AndroidFrameClock,
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
        let runtime = SharedAppRuntime::new(
            initial_levels,
            surface_width,
            surface_height,
            android_runtime_config(),
        );
        let mut app = Self {
            runtime,
            pending_present_damage: FrameDamage::Noop,
            pending_presentation_frame_for_present: false,
            native_window: None,
            frame_clock: AndroidFrameClock::default(),
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
        self.frame_clock.reset();
        self.native_window = native_window;
        self.configure_native_window();
        if self.native_window.is_some() {
            self.mark_frame_damage(FrameDamage::Full);
        }
    }

    pub fn present_frame(&mut self, frame_time_nanos: Option<u64>) -> bool {
        let presentation_now = self.frame_clock.instant_for_frame_time(frame_time_nanos);
        if matches!(self.pending_present_damage, FrameDamage::Noop)
            && !self.pending_presentation_frame_for_present
            && self.runtime.has_pending_render_work()
        {
            let _ = self.continue_pending_render_work_and_render_at(presentation_now);
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
        let presents_pending_presentation_frame = self.pending_presentation_frame_for_present;
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
            if presents_pending_presentation_frame {
                self.runtime
                    .mark_pending_presentation_frame_presented_at(presentation_now);
            }
            self.pending_present_damage = FrameDamage::Noop;
            self.pending_presentation_frame_for_present = false;
        }
        presented
    }

    pub fn has_pending_render_work(&mut self) -> bool {
        !matches!(self.pending_present_damage, FrameDamage::Noop)
            || self.pending_presentation_frame_for_present
            || self.runtime.has_pending_render_work()
    }

    fn render_full_current_request(&mut self) {
        let mut presenter = AndroidFramePresenter {
            pending_present_damage: &mut self.pending_present_damage,
            pending_presentation_frame_for_present: &mut self
                .pending_presentation_frame_for_present,
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
            pending_presentation_frame_for_present: &mut self
                .pending_presentation_frame_for_present,
        };
        self.runtime
            .handle_pointer_input_and_render(input, &mut presenter)
            .map(|_| ())
    }

    fn continue_pending_render_work_and_render_at(&mut self, now: Instant) -> Result<(), ()> {
        let mut presenter = AndroidFramePresenter {
            pending_present_damage: &mut self.pending_present_damage,
            pending_presentation_frame_for_present: &mut self
                .pending_presentation_frame_for_present,
        };
        self.runtime
            .continue_pending_render_work_and_render_at(&mut presenter, now)
            .map(|_| ())
    }
}

#[derive(Default)]
struct AndroidFrameClock {
    base_frame_time_nanos: Option<u64>,
    base_instant: Option<Instant>,
    last_instant: Option<Instant>,
}

impl AndroidFrameClock {
    fn reset(&mut self) {
        *self = Self::default();
    }

    fn instant_for_frame_time(&mut self, frame_time_nanos: Option<u64>) -> Instant {
        let Some(frame_time_nanos) = frame_time_nanos else {
            return self.monotonic_instant(Instant::now());
        };
        let base_frame_time_nanos = *self.base_frame_time_nanos.get_or_insert(frame_time_nanos);
        let base_instant = *self.base_instant.get_or_insert_with(Instant::now);
        let delta = frame_time_nanos.saturating_sub(base_frame_time_nanos);
        let frame_instant = base_instant + Duration::from_nanos(delta);
        self.monotonic_instant(frame_instant)
    }

    fn monotonic_instant(&mut self, instant: Instant) -> Instant {
        let monotonic_frame_instant = self
            .last_instant
            .map(|last| instant.max(last))
            .unwrap_or(instant);
        self.last_instant = Some(monotonic_frame_instant);
        monotonic_frame_instant
    }
}

fn android_runtime_config() -> SharedAppRuntimeConfig {
    SharedAppRuntimeConfig {
        supports_multi_touch: true,
        gameplay_touch_slop_px: Some(ANDROID_GAMEPLAY_TAP_SLOP_PX),
        editor_touch_slop_px: Some(ANDROID_EDITOR_TAP_SLOP_PX),
        editor_double_tap_window: Some(ANDROID_EDITOR_DOUBLE_TAP_WINDOW),
        renderer: SharedAppRendererConfig {
            renderer_overrides: android_renderer_overrides(),
            gameplay_animation_policy: GameplayAnimationPolicy::Full,
        },
        ..SharedAppRuntimeConfig::default()
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

struct AndroidFramePresenter<'a> {
    pending_present_damage: &'a mut FrameDamage,
    pending_presentation_frame_for_present: &'a mut bool,
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

    fn presented_frame_time(&self) -> Option<Instant> {
        None
    }

    fn note_pending_presentation_frame(&mut self) {
        *self.pending_presentation_frame_for_present = true;
    }
}

#[cfg(test)]
mod tests {
    use super::AndroidFrameClock;

    #[test]
    fn android_frame_clock_keeps_backwards_frame_times_monotonic() {
        let mut clock = AndroidFrameClock::default();

        let first = clock.instant_for_frame_time(Some(1_000));
        let second = clock.instant_for_frame_time(Some(500));

        assert!(second >= first);
    }

    #[test]
    fn android_frame_clock_fallback_does_not_poison_frame_time_base() {
        let mut clock = AndroidFrameClock::default();

        let fallback = clock.instant_for_frame_time(None);
        let choreographer_time = clock.instant_for_frame_time(Some(1_000));

        assert!(choreographer_time >= fallback);
        assert_eq!(clock.base_frame_time_nanos, Some(1_000));
    }
}
