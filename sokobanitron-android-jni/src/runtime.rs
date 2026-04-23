use crate::native_window::NativeWindow;
use sokobanitron_app::{
    app::{
        AppDriverContext, AppFrameRenderer, AppPointerInput, AppRuntimeMut, AppState,
        EditorAppRuntimeMut, FrameDamage, FrameRequest, FrameSink, GameplayAnimationPolicy,
        RendererOverrides, build_current_app_screen_frame_request,
        continue_pending_render_work_and_render_in_context,
        handle_pointer_input_and_render_in_context, has_pending_render_work_in_context,
    },
    editor::{resize_editor_surface, set_editor_double_tap_window, set_editor_touch_slop},
    gameplay::{resize_gameplay_surface, set_gameplay_level_sets, set_gameplay_touch_slop},
    level_bootstrap::load_initial_levels_for_app,
    persistence::LevelPersistence,
    shared::PointerPhase,
};
use sokobanitron_gameplay::{BoardView, GameplayController};
use sokobanitron_level_editor::LevelEditor;
use std::io;
use std::path::Path;
use std::time::Duration;

const ANDROID_GAMEPLAY_TAP_SLOP_PX: i32 = 24;
const ANDROID_EDITOR_TAP_SLOP_PX: i32 = 24;
const ANDROID_EDITOR_DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(750);

pub struct AndroidApp {
    frame_renderer: AppFrameRenderer,
    gray_frame: Vec<u8>,
    current_request: FrameRequest,
    pending_present_damage: FrameDamage,
    preview_boards: Vec<BoardView>,
    controller: GameplayController,
    app_state: AppState,
    level_persistence: LevelPersistence,
    surface_width: u32,
    surface_height: u32,
    editor: LevelEditor,
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
        let levels = initial_levels.levels;
        let preview_boards = initial_levels.preview_boards;
        let controller = GameplayController::new_at_level(
            levels.clone(),
            initial_levels.initial_level_index,
            initial_levels.persisted_resume_level_index,
        );
        let mut app_state = AppState {
            editor_available: true,
            supports_multi_touch: true,
            ..AppState::default()
        };
        resize_gameplay_surface(&mut app_state.gameplay, surface_width, surface_height);
        resize_editor_surface(&mut app_state, surface_width, surface_height);
        set_gameplay_touch_slop(&mut app_state.gameplay, ANDROID_GAMEPLAY_TAP_SLOP_PX);
        set_editor_touch_slop(&mut app_state, ANDROID_EDITOR_TAP_SLOP_PX);
        set_editor_double_tap_window(&mut app_state, ANDROID_EDITOR_DOUBLE_TAP_WINDOW);
        set_gameplay_level_sets(
            &mut app_state.gameplay,
            initial_levels.level_set_catalog,
            Some(initial_levels.active_level_set_index),
        );
        let editor = LevelEditor::new();
        let current_request =
            build_current_app_screen_frame_request(&controller, &app_state, &editor);

        let mut app = Self {
            frame_renderer: AppFrameRenderer::with_renderer_overrides_and_gameplay_animation_policy(
                android_renderer_overrides(),
                GameplayAnimationPolicy::Full,
            ),
            gray_frame: allocate_gray_frame(surface_width, surface_height),
            current_request,
            pending_present_damage: FrameDamage::Noop,
            preview_boards,
            controller,
            app_state,
            level_persistence: initial_levels.persistence,
            surface_width,
            surface_height,
            editor,
            native_window: None,
        };
        app.render_full_current_request();
        Ok(app)
    }

    pub fn resize(&mut self, surface_width: u32, surface_height: u32) {
        self.surface_width = surface_width.max(1);
        self.surface_height = surface_height.max(1);
        self.gray_frame = allocate_gray_frame(self.surface_width, self.surface_height);
        resize_gameplay_surface(
            &mut self.app_state.gameplay,
            self.surface_width,
            self.surface_height,
        );
        resize_editor_surface(&mut self.app_state, self.surface_width, self.surface_height);
        self.configure_native_window();
        self.render_full_current_request();
    }

    pub fn handle_pointer_event(&mut self, id: u64, phase: PointerPhase, x: f64, y: f64) -> bool {
        let _ = handle_pointer_input_and_render_in_context(
            self,
            AppPointerInput::Pointer { id, phase, x, y },
        );
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
            && has_pending_render_work_in_context(self)
        {
            let _ = continue_pending_render_work_and_render_in_context(self);
        }

        if !self.has_pending_render_work() {
            return true;
        }
        let Some(mut window) = self.native_window.take() else {
            return false;
        };
        let surface_width = self.surface_width;
        let surface_height = self.surface_height;
        let damage = self.pending_present_damage;
        let presented = match damage {
            FrameDamage::Full => {
                window.present_gray(&self.gray_frame, surface_width, surface_height)
            }
            FrameDamage::Noop => true,
            FrameDamage::Region(region) => {
                window.present_gray_region(&self.gray_frame, surface_width, surface_height, region)
            }
        };
        self.native_window = Some(window);
        if presented {
            self.pending_present_damage = FrameDamage::Noop;
        }
        presented
    }

    pub fn has_pending_render_work(&mut self) -> bool {
        !matches!(self.pending_present_damage, FrameDamage::Noop)
            || has_pending_render_work_in_context(self)
    }

    fn build_current_request(&self) -> FrameRequest {
        build_current_app_screen_frame_request(&self.controller, &self.app_state, &self.editor)
    }

    fn render_full_current_request(&mut self) {
        let request = self.build_current_request();
        self.render_full_request(request);
    }

    fn render_presentation_request(&mut self, request: FrameRequest) {
        let damage = self.render_request_into_frame(&request);
        self.current_request = request;
        self.mark_frame_damage(damage);
    }

    fn render_full_request(&mut self, request: FrameRequest) {
        self.draw_full_request_into_frame(&request);
        self.current_request = request;
        self.mark_frame_damage(FrameDamage::Full);
    }

    fn mark_frame_damage(&mut self, damage: FrameDamage) {
        self.pending_present_damage = self.pending_present_damage.merge(damage);
    }

    fn configure_native_window(&mut self) {
        if let Some(window) = self.native_window.as_mut()
            && !window.configure(self.surface_width, self.surface_height)
        {
            #[cfg(debug_assertions)]
            eprintln!(
                "warning: failed to configure Android native window for {}x{}",
                self.surface_width, self.surface_height
            );
        }
    }

    fn render_request_into_frame(&mut self, request: &FrameRequest) -> FrameDamage {
        self.frame_renderer.draw_frame_request(
            &mut self.gray_frame,
            self.surface_width,
            self.surface_height,
            request,
            &self.preview_boards,
        )
    }

    fn render_pending_visible_presentation_into_frame(&mut self) -> FrameDamage {
        self.frame_renderer.draw_pending_visible_presentation(
            &self.app_state,
            &mut self.gray_frame,
            self.surface_width,
            self.surface_height,
        )
    }

    fn draw_full_request_into_frame(&mut self, request: &FrameRequest) {
        self.frame_renderer.draw_full_frame_request(
            &mut self.gray_frame,
            self.surface_width,
            self.surface_height,
            request,
            &self.preview_boards,
        );
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

impl AppDriverContext for AndroidApp {
    type Error = ();

    fn app_runtime_mut(&mut self) -> AppRuntimeMut<'_> {
        AppRuntimeMut {
            controller: &mut self.controller,
            app_state: &mut self.app_state,
            level_persistence: &mut self.level_persistence,
            preview_boards: &mut self.preview_boards,
        }
    }

    fn editor_runtime_mut(&mut self) -> Option<EditorAppRuntimeMut<'_>> {
        Some(
            AppRuntimeMut {
                controller: &mut self.controller,
                app_state: &mut self.app_state,
                level_persistence: &mut self.level_persistence,
                preview_boards: &mut self.preview_boards,
            }
            .with_editor(&mut self.editor),
        )
    }

    fn has_pending_frame_presentation(&mut self) -> bool {
        self.frame_renderer
            .has_pending_visible_presentation(&self.app_state)
    }

    fn continue_frame_presentation_and_render(&mut self) -> Result<bool, Self::Error> {
        if !self
            .frame_renderer
            .has_pending_visible_presentation(&self.app_state)
        {
            return Ok(false);
        }
        let damage = self.render_pending_visible_presentation_into_frame();
        let frame_changed = !matches!(damage, FrameDamage::Noop);
        self.mark_frame_damage(damage);
        Ok(frame_changed)
    }
}

impl FrameSink for AndroidApp {
    type Error = ();

    fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error> {
        self.render_presentation_request(request.clone());
        Ok(())
    }
}

fn allocate_gray_frame(surface_width: u32, surface_height: u32) -> Vec<u8> {
    vec![0; frame_len(surface_width, surface_height)]
}

fn frame_len(surface_width: u32, surface_height: u32) -> usize {
    usize::try_from(surface_width)
        .expect("surface width should fit usize")
        .saturating_mul(usize::try_from(surface_height).expect("surface height should fit usize"))
}
