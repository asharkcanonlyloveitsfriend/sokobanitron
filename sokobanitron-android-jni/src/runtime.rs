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
    argb_frame: Vec<i32>,
    preview_boards: Vec<BoardView>,
    controller: GameplayController,
    app_state: AppState,
    level_persistence: LevelPersistence,
    surface_width: u32,
    surface_height: u32,
    editor: LevelEditor,
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

        let mut app = Self {
            renderer: Renderer::new(),
            gameplay_presentation: GameplayPresentationState::new(),
            rgba_frame: allocate_rgba_frame(surface_width, surface_height),
            argb_frame: allocate_argb_frame(surface_width, surface_height),
            preview_boards,
            controller,
            app_state,
            level_persistence: initial_levels.persistence,
            surface_width,
            surface_height,
            editor: LevelEditor::new(),
        };
        app.render_current();
        Ok(app)
    }

    pub fn resize(&mut self, surface_width: u32, surface_height: u32) {
        self.surface_width = surface_width.max(1);
        self.surface_height = surface_height.max(1);
        self.rgba_frame = allocate_rgba_frame(self.surface_width, self.surface_height);
        self.argb_frame = allocate_argb_frame(self.surface_width, self.surface_height);
        resize_gameplay_surface(
            &mut self.app_state.gameplay,
            self.surface_width,
            self.surface_height,
        );
        resize_editor_surface(&mut self.app_state, self.surface_width, self.surface_height);
        self.render_current();
    }

    pub fn handle_pointer_event(&mut self, id: u64, phase: PointerPhase, x: f64, y: f64) {
        match self.app_state.interaction_mode() {
            AppInteractionMode::Gameplay => self.on_gameplay_pointer_event(id, phase, x, y),
            AppInteractionMode::Editor => self.on_editor_touch(id, phase, x, y),
            AppInteractionMode::Overlay(_) => self.on_overlay_pointer_event(id, phase, x, y),
        }
    }

    pub fn frame_pixels(&self) -> &[i32] {
        &self.argb_frame
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
        let action = editor_touch(&mut self.app_state, &mut self.editor, id, phase, x, y);
        let runtime = AppRuntimeMut {
            controller: &mut self.controller,
            app_state: &mut self.app_state,
            level_persistence: &mut self.level_persistence,
            preview_boards: &mut self.preview_boards,
        };
        apply_editor_ui_action(action, runtime.with_editor(&mut self.editor));
        self.render_current();
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
        let request = match self.app_state.active_screen() {
            AppScreen::Gameplay => build_current_frame_request(&self.controller, &self.app_state),
            AppScreen::Editor => build_current_editor_frame_request(&self.app_state, &self.editor),
        };
        let _ = self.render_request(&request);
    }

    fn render_active_gameplay_screen(&mut self) {
        let request = build_current_frame_request(&self.controller, &self.app_state);
        let _ = self.render_request(&request);
    }

    fn render_request(&mut self, request: &FrameRequest) -> Result<(), ()> {
        match request {
            FrameRequest::Gameplay { screen, .. } => {
                self.gameplay_presentation.replace_scene(screen.clone());
                self.gameplay_presentation.draw(
                    &mut self.renderer,
                    &mut self.rgba_frame,
                    self.surface_width,
                    self.surface_height,
                );
            }
            FrameRequest::GameplayMenu { screen } => {
                self.renderer.draw_background_only(
                    &mut self.rgba_frame,
                    self.surface_width,
                    self.surface_height,
                );
                draw_top_menu_toggle(
                    &mut self.rgba_frame,
                    self.surface_width,
                    self.surface_height,
                    true,
                );
                if screen.show_change_level_set {
                    draw_gameplay_menu_level_set_button(
                        &mut self.rgba_frame,
                        self.surface_width,
                        self.surface_height,
                    );
                }
                if let Some(icon) = screen.primary_action_icon {
                    draw_overlay_primary_action_button(
                        &mut self.rgba_frame,
                        self.surface_width,
                        self.surface_height,
                        icon,
                        BUTTON_TEXT_COLOR,
                    );
                }
            }
            FrameRequest::LevelSelect { screen, .. } => {
                self.renderer.draw_background_only(
                    &mut self.rgba_frame,
                    self.surface_width,
                    self.surface_height,
                );
                self.renderer.draw_level_select_menu_contents(
                    &mut self.rgba_frame,
                    self.surface_width,
                    self.surface_height,
                    &self.preview_boards,
                    screen.resume_level,
                    screen.page_start,
                );
                draw_controls_ui(
                    &mut self.rgba_frame,
                    self.surface_width,
                    self.surface_height,
                    ControlsUiMode::MenuOpen,
                    false,
                    false,
                );
            }
            FrameRequest::LevelSetSelect { screen, .. } => {
                self.renderer.draw_background_only(
                    &mut self.rgba_frame,
                    self.surface_width,
                    self.surface_height,
                );
                self.renderer.draw_level_set_select_menu_contents(
                    &mut self.rgba_frame,
                    self.surface_width,
                    self.surface_height,
                    screen,
                );
                draw_controls_ui(
                    &mut self.rgba_frame,
                    self.surface_width,
                    self.surface_height,
                    ControlsUiMode::MenuOpen,
                    false,
                    false,
                );
            }
            FrameRequest::Editor { screen } => {
                self.renderer.draw_editor_screen(
                    &mut self.rgba_frame,
                    self.surface_width,
                    self.surface_height,
                    screen,
                );
            }
            FrameRequest::EditorMenu { screen } => {
                self.renderer.draw_editor_menu(
                    &mut self.rgba_frame,
                    self.surface_width,
                    self.surface_height,
                    screen,
                );
            }
        }
        self.sync_argb_frame();
        Ok(())
    }

    fn sync_argb_frame(&mut self) {
        for (argb, rgba) in self
            .argb_frame
            .iter_mut()
            .zip(self.rgba_frame.chunks_exact(4))
        {
            let red = i32::from(rgba[0]);
            let green = i32::from(rgba[1]);
            let blue = i32::from(rgba[2]);
            let alpha = i32::from(rgba[3]);
            *argb = (alpha << 24) | (red << 16) | (green << 8) | blue;
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
        self.render_request(request)
    }
}

fn allocate_rgba_frame(surface_width: u32, surface_height: u32) -> Vec<u8> {
    vec![0; frame_len(surface_width, surface_height, 4)]
}

fn allocate_argb_frame(surface_width: u32, surface_height: u32) -> Vec<i32> {
    vec![0; frame_len(surface_width, surface_height, 1)]
}

fn frame_len(surface_width: u32, surface_height: u32, channels: usize) -> usize {
    usize::try_from(surface_width)
        .unwrap_or(1)
        .saturating_mul(usize::try_from(surface_height).unwrap_or(1))
        .saturating_mul(channels)
}
