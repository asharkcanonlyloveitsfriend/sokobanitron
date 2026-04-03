use crate::native_window::NativeWindow;
use presentation::layout::ControlsUiMode;
use presentation::renderer::{
    Renderer, draw_controls_ui, draw_gameplay_menu_level_set_button,
    draw_overlay_primary_action_button, draw_top_menu_toggle,
};
use presentation::{GameplayPresentationState, Renderer as GameplayRenderer};
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
const BUTTON_TEXT_COLOR: [u8; 4] = [220, 220, 220, 255];

pub struct AndroidApp {
    renderer: GameplayRenderer,
    gameplay_presentation: GameplayPresentationState,
    rgba_frame: Vec<u8>,
    current_request: FrameRequest,
    frame_dirty: bool,
    preview_boards: Vec<BoardView>,
    controller: GameplayController,
    app_state: AppState,
    level_persistence: LevelPersistence,
    surface_width: u32,
    surface_height: u32,
    editor: LevelEditor,
    native_window: Option<NativeWindow>,
    // Tracks presentation-relevant invalidation observed by the Android host. Today we only bump
    // this when the built frame request changes, which is a deliberate current limitation rather
    // than a claim that request equality always implies identical pixels.
    presentation_generation: u64,
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

        Ok(Self {
            renderer: Renderer::new(),
            gameplay_presentation: GameplayPresentationState::new(),
            rgba_frame: allocate_rgba_frame(surface_width, surface_height),
            current_request,
            frame_dirty: true,
            preview_boards,
            controller,
            app_state,
            level_persistence: initial_levels.persistence,
            surface_width,
            surface_height,
            editor,
            native_window: None,
            presentation_generation: 0,
        })
    }

    pub fn resize(&mut self, surface_width: u32, surface_height: u32) {
        self.surface_width = surface_width.max(1);
        self.surface_height = surface_height.max(1);
        self.rgba_frame = allocate_rgba_frame(self.surface_width, self.surface_height);
        resize_gameplay_surface(
            &mut self.app_state.gameplay,
            self.surface_width,
            self.surface_height,
        );
        resize_editor_surface(&mut self.app_state, self.surface_width, self.surface_height);
        self.configure_native_window();
        self.render_current();
    }

    pub fn handle_pointer_event(&mut self, id: u64, phase: PointerPhase, x: f64, y: f64) -> bool {
        let presentation_generation_before = self.presentation_generation;
        match self.app_state.interaction_mode() {
            AppInteractionMode::Gameplay => self.on_gameplay_pointer_event(id, phase, x, y),
            AppInteractionMode::Editor => self.on_editor_touch(id, phase, x, y),
            AppInteractionMode::Overlay(_) => self.on_overlay_pointer_event(id, phase, x, y),
        }
        self.presentation_generation != presentation_generation_before
    }

    pub fn set_native_window(&mut self, native_window: Option<NativeWindow>) {
        self.native_window = native_window;
        self.configure_native_window();
        if self.native_window.is_some() {
            self.frame_dirty = true;
        }
    }

    pub fn present_frame(&mut self) -> bool {
        if !self.frame_dirty {
            return true;
        }
        let Some(mut window) = self.native_window.take() else {
            return false;
        };
        // Temporarily move the window out so we can render into app-owned buffers and then present
        // through the window without holding overlapping borrows on `self`.
        let request = self.current_request.clone();
        let surface_width = self.surface_width;
        let surface_height = self.surface_height;
        let mut frame = std::mem::take(&mut self.rgba_frame);
        self.render_request_into(&request, &mut frame, surface_width, surface_height);
        let presented = window.present_rgba(&frame, surface_width, surface_height);
        self.rgba_frame = frame;
        self.native_window = Some(window);
        if presented {
            self.frame_dirty = false;
        }
        presented
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
            AppInput::BoardTap { .. } => {
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
            self.queue_request(after_request);
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
        self.queue_request(request);
    }

    fn build_current_request(&self) -> FrameRequest {
        build_android_frame_request(&self.controller, &self.app_state, &self.editor)
    }

    fn render_active_gameplay_screen(&mut self) {
        let request = build_current_frame_request(&self.controller, &self.app_state);
        self.queue_request(request);
    }

    // Current invalidation policy: only queue a presentable update when the rebuilt frame request
    // changes. Future shared presentation work may need additional invalidation paths even when the
    // request shape stays the same.
    fn queue_request(&mut self, request: FrameRequest) {
        if self.current_request != request {
            self.current_request = request;
            self.frame_dirty = true;
            self.presentation_generation = self.presentation_generation.wrapping_add(1);
        }
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

    fn render_request_into(
        &mut self,
        request: &FrameRequest,
        frame: &mut [u8],
        surface_width: u32,
        surface_height: u32,
    ) {
        match request {
            FrameRequest::Gameplay { screen, .. } => {
                self.gameplay_presentation.replace_scene(screen.clone());
                self.gameplay_presentation.draw(
                    &mut self.renderer,
                    frame,
                    surface_width,
                    surface_height,
                );
            }
            FrameRequest::GameplayMenu { screen } => {
                self.renderer
                    .draw_background_only(frame, surface_width, surface_height);
                draw_top_menu_toggle(frame, surface_width, surface_height, true);
                if screen.show_change_level_set {
                    draw_gameplay_menu_level_set_button(frame, surface_width, surface_height);
                }
                if let Some(icon) = screen.primary_action_icon {
                    draw_overlay_primary_action_button(
                        frame,
                        surface_width,
                        surface_height,
                        icon,
                        BUTTON_TEXT_COLOR,
                    );
                }
            }
            FrameRequest::LevelSelect { screen, .. } => {
                self.renderer
                    .draw_background_only(frame, surface_width, surface_height);
                self.renderer.draw_level_select_menu_contents(
                    frame,
                    surface_width,
                    surface_height,
                    &self.preview_boards,
                    screen.resume_level,
                    screen.page_start,
                );
                draw_controls_ui(
                    frame,
                    surface_width,
                    surface_height,
                    ControlsUiMode::MenuOpen,
                    false,
                    false,
                );
            }
            FrameRequest::LevelSetSelect { screen, .. } => {
                self.renderer
                    .draw_background_only(frame, surface_width, surface_height);
                self.renderer.draw_level_set_select_menu_contents(
                    frame,
                    surface_width,
                    surface_height,
                    screen,
                );
                draw_controls_ui(
                    frame,
                    surface_width,
                    surface_height,
                    ControlsUiMode::MenuOpen,
                    false,
                    false,
                );
            }
            FrameRequest::Editor { screen } => {
                self.renderer
                    .draw_editor_screen(frame, surface_width, surface_height, screen);
            }
            FrameRequest::EditorMenu { screen } => {
                self.renderer
                    .draw_editor_menu(frame, surface_width, surface_height, screen);
            }
        }
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
        self.queue_request(request.clone());
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

fn allocate_rgba_frame(surface_width: u32, surface_height: u32) -> Vec<u8> {
    vec![0; frame_len(surface_width, surface_height, 4)]
}

fn frame_len(surface_width: u32, surface_height: u32, channels: usize) -> usize {
    usize::try_from(surface_width)
        .unwrap_or(1)
        .saturating_mul(usize::try_from(surface_height).unwrap_or(1))
        .saturating_mul(channels)
}
