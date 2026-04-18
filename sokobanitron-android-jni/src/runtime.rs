use crate::native_window::NativeWindow;
use presentation::renderer::{
    Renderer, RendererOverrides, draw_controls_ui, draw_gameplay_menu_level_set_button,
    draw_overlay_primary_action_button, draw_top_menu_toggle,
};
use presentation::{
    GameplayDamage, GameplayPresentationState, Renderer as GameplayRenderer, ScreenRect,
    gameplay_damage_union_rect,
};
use sokobanitron_app::{
    app::{
        AppDriverContext, AppInput, AppInteractionMode, AppRuntimeMut, AppScreen, AppState,
        AppliedUpdate, FrameRequest, FrameSink, apply_editor_ui_action,
        apply_input_and_render_in_context,
    },
    editor::{
        build_current_editor_frame_request, editor_touch, reset_editor_interaction_state,
        resize_editor_surface, set_editor_double_tap_window, set_editor_touch_slop,
    },
    gameplay::{
        build_current_frame_request, interpret_gameplay_pointer_event, resize_gameplay_surface,
        set_gameplay_level_sets, set_gameplay_touch_slop,
    },
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
    renderer: GameplayRenderer,
    gameplay_presentation: GameplayPresentationState,
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
        let current_request = build_android_frame_request(&controller, &app_state, &editor);

        let mut app = Self {
            renderer: Renderer::with_overrides(android_renderer_overrides()),
            gameplay_presentation: GameplayPresentationState::new(),
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
        match self.app_state.interaction_mode() {
            AppInteractionMode::Gameplay => self.on_gameplay_pointer_event(id, phase, x, y),
            AppInteractionMode::Editor => self.on_editor_touch(id, phase, x, y),
            AppInteractionMode::Overlay(_) => self.on_overlay_pointer_event(id, phase, x, y),
        }
        self.needs_present()
    }

    pub fn set_native_window(&mut self, native_window: Option<NativeWindow>) {
        self.native_window = native_window;
        self.configure_native_window();
        if self.native_window.is_some() {
            self.mark_frame_damage(FrameDamage::Full);
        }
    }

    pub fn present_frame(&mut self) -> bool {
        if !self.needs_present() {
            return true;
        }
        let Some(mut window) = self.native_window.take() else {
            return false;
        };
        if matches!(self.pending_present_damage, FrameDamage::Noop)
            && self.app_state.is_gameplay_screen()
            && self.gameplay_presentation.has_pending_presentation()
        {
            let damage = self.render_active_gameplay_presentation_into_frame();
            self.mark_frame_damage(damage);
        }
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

    pub fn has_pending_gameplay_presentation(&self) -> bool {
        self.app_state.is_gameplay_screen() && self.gameplay_presentation.has_pending_presentation()
    }

    fn apply_app_input(&mut self, input: AppInput) -> Option<AppliedUpdate> {
        apply_input_and_render_in_context(self, input).ok()
    }

    fn enter_editor_mode(&mut self) {
        let _ = self.apply_app_input(AppInput::EnterEditorMode);
        reset_editor_interaction_state(&mut self.app_state);
    }

    fn handle_gameplay_input(&mut self, input: AppInput) {
        match input {
            AppInput::NoOp => {}
            AppInput::EnterEditorMode => {
                self.enter_editor_mode();
                self.render_current();
            }
            AppInput::BoardTap(_) => {
                let _ = self.apply_app_input(input);
            }
            _ => {
                let Some(applied) = self.apply_app_input(input) else {
                    return;
                };
                if !applied.rendered_frame {
                    self.render_active_gameplay_screen();
                }
            }
        }
    }

    fn on_gameplay_pointer_event(&mut self, id: u64, phase: PointerPhase, x: f64, y: f64) {
        let input = interpret_gameplay_pointer_event(
            &mut self.app_state,
            &self.controller,
            id,
            phase,
            x,
            y,
        );
        self.handle_gameplay_input(input);
    }

    fn on_editor_touch(&mut self, id: u64, phase: PointerPhase, x: f64, y: f64) {
        let before_request = self.build_current_request();
        let action = editor_touch(&mut self.app_state, &mut self.editor, id, phase, x, y);
        let runtime = AppRuntimeMut {
            controller: &mut self.controller,
            app_state: &mut self.app_state,
            level_persistence: &mut self.level_persistence,
            preview_boards: &mut self.preview_boards,
        };
        apply_editor_ui_action(action, runtime.with_editor(&mut self.editor));
        let after_request = self.build_current_request();
        if after_request != before_request {
            self.render_changed_request(after_request);
        }
    }

    fn on_overlay_pointer_event(&mut self, id: u64, phase: PointerPhase, x: f64, y: f64) {
        let AppInteractionMode::Overlay(overlay) = self.app_state.interaction_mode() else {
            return;
        };
        match overlay.owning_screen() {
            AppScreen::Gameplay => self.on_gameplay_pointer_event(id, phase, x, y),
            AppScreen::Editor => self.on_editor_touch(id, phase, x, y),
        }
    }

    fn render_current(&mut self) {
        let request = self.build_current_request();
        self.render_changed_request(request);
    }

    fn build_current_request(&self) -> FrameRequest {
        build_android_frame_request(&self.controller, &self.app_state, &self.editor)
    }

    fn render_active_gameplay_screen(&mut self) {
        let request = build_current_frame_request(&self.controller, &self.app_state);
        self.render_changed_request(request);
    }

    fn render_changed_request(&mut self, request: FrameRequest) {
        if self.current_request != request {
            self.render_presentation_request(request);
        }
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

    fn needs_present(&self) -> bool {
        !matches!(self.pending_present_damage, FrameDamage::Noop)
            || self.has_pending_gameplay_presentation()
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
        match request {
            FrameRequest::Gameplay { update, .. } => {
                let result = self
                    .gameplay_presentation
                    .replace_update_with_damage(update.clone());
                self.gameplay_presentation.draw_damage(
                    &mut self.renderer,
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                    &result.damage,
                );
                FrameDamage::from_gameplay_damage(
                    &update.scene,
                    &result.damage,
                    self.surface_width,
                    self.surface_height,
                )
            }
            FrameRequest::GameplayMenu { screen } => {
                self.gameplay_presentation.clear();
                self.renderer.draw_background_only(
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                );
                draw_top_menu_toggle(
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                    true,
                    self.renderer.theme(),
                );
                if screen.show_change_level_set {
                    draw_gameplay_menu_level_set_button(
                        &mut self.gray_frame,
                        self.surface_width,
                        self.surface_height,
                        self.renderer.theme(),
                    );
                }
                if let Some(icon) = screen.primary_action_icon {
                    draw_overlay_primary_action_button(
                        &mut self.gray_frame,
                        self.surface_width,
                        self.surface_height,
                        icon,
                        self.renderer.theme().gray_2,
                    );
                }
                FrameDamage::Full
            }
            FrameRequest::LevelSelect { screen, .. } => {
                self.gameplay_presentation.clear();
                self.renderer.draw_background_only(
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                );
                self.renderer.draw_level_select_menu_contents(
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                    &self.preview_boards,
                    screen.resume_level,
                    screen.page_start,
                );
                draw_controls_ui(
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                    true,
                    self.renderer.theme(),
                );
                FrameDamage::Full
            }
            FrameRequest::LevelSetSelect { screen, .. } => {
                self.gameplay_presentation.clear();
                self.renderer.draw_background_only(
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                );
                self.renderer.draw_level_set_select_menu_contents(
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                    screen,
                );
                draw_controls_ui(
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                    true,
                    self.renderer.theme(),
                );
                FrameDamage::Full
            }
            FrameRequest::Editor { screen } => {
                self.gameplay_presentation.clear();
                self.renderer.draw_editor_screen(
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                    screen,
                );
                FrameDamage::Full
            }
            FrameRequest::EditorMenu { screen } => {
                self.gameplay_presentation.clear();
                self.renderer.draw_editor_menu(
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                    screen,
                );
                FrameDamage::Full
            }
        }
    }

    fn render_active_gameplay_presentation_into_frame(&mut self) -> FrameDamage {
        let Some(scene) = self.gameplay_presentation.current_scene().cloned() else {
            return FrameDamage::Noop;
        };
        let result = self
            .gameplay_presentation
            .advance_presentation_with_damage();
        self.gameplay_presentation.draw_damage(
            &mut self.renderer,
            &mut self.gray_frame,
            self.surface_width,
            self.surface_height,
            &result.damage,
        );
        FrameDamage::from_gameplay_damage(
            &scene,
            &result.damage,
            self.surface_width,
            self.surface_height,
        )
    }

    fn draw_full_request_into_frame(&mut self, request: &FrameRequest) {
        match request {
            FrameRequest::Gameplay { update, .. } => {
                self.gameplay_presentation.replace_update(update.clone());
                self.gameplay_presentation.draw(
                    &mut self.renderer,
                    &mut self.gray_frame,
                    self.surface_width,
                    self.surface_height,
                );
            }
            FrameRequest::GameplayMenu { .. }
            | FrameRequest::LevelSelect { .. }
            | FrameRequest::LevelSetSelect { .. }
            | FrameRequest::Editor { .. }
            | FrameRequest::EditorMenu { .. } => {
                let _ = self.render_request_into_frame(request);
            }
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameDamage {
    Full,
    Region(ScreenRect),
    Noop,
}

impl FrameDamage {
    fn from_gameplay_damage(
        scene: &presentation::GameplayScreenRequest,
        damage: &GameplayDamage,
        surface_width: u32,
        surface_height: u32,
    ) -> Self {
        match damage {
            GameplayDamage::Full => Self::Full,
            GameplayDamage::Cells(cells) if cells.is_empty() => Self::Noop,
            GameplayDamage::Cells(_) => Self::Region(
                gameplay_damage_union_rect(scene, damage, surface_width, surface_height)
                    .expect("non-empty gameplay damage should map to an Android screen rect"),
            ),
        }
    }

    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Full, _) | (_, Self::Full) => Self::Full,
            (Self::Noop, damage) | (damage, Self::Noop) => damage,
            (Self::Region(a), Self::Region(b)) => Self::Region(union_screen_rect(a, b)),
        }
    }
}

fn union_screen_rect(a: ScreenRect, b: ScreenRect) -> ScreenRect {
    let left = a.x.min(b.x);
    let top = a.y.min(b.y);
    let right = a.x.saturating_add(a.w).max(b.x.saturating_add(b.w));
    let bottom = a.y.saturating_add(a.h).max(b.y.saturating_add(b.h));
    ScreenRect {
        x: left,
        y: top,
        w: right - left,
        h: bottom - top,
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
}

impl FrameSink for AndroidApp {
    type Error = ();

    fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error> {
        if !self.app_state.is_gameplay_screen() {
            return Ok(());
        }
        self.render_presentation_request(request.clone());
        Ok(())
    }
}

fn build_android_frame_request(
    controller: &GameplayController,
    app_state: &AppState,
    editor: &LevelEditor,
) -> FrameRequest {
    match app_state.active_screen() {
        AppScreen::Gameplay => build_current_frame_request(controller, app_state),
        AppScreen::Editor => build_current_editor_frame_request(app_state, editor),
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
