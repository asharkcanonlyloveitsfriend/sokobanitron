use pixels::{Pixels, SurfaceTexture};
use presentation::{GameplayPresentationState, Renderer};
use sokobanitron_app::{
    app::{
        AppDriverContext, AppInput, AppInteractionMode, AppRuntimeMut, AppScreen, AppState,
        AppliedUpdate, apply_input_and_render_in_context,
    },
    editor::{
        editor_cursor_moved, editor_mouse_pressed, editor_mouse_released, editor_touch,
        reset_editor_interaction_state, resize_editor_surface,
    },
    gameplay::{
        interpret_gameplay_pointer_event, interpret_gameplay_pointer_tap, resize_gameplay_surface,
        set_gameplay_level_sets,
    },
    level_bootstrap::load_initial_levels_for_app,
    persistence::LevelPersistence,
    shared::PointerPhase,
};
use sokobanitron_gameplay::{BoardView, GameplayController};
use sokobanitron_level_editor::LevelEditor;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, MouseButton, TouchPhase, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes},
};

const INITIAL_WIDTH: u32 = 670;
const INITIAL_HEIGHT: u32 = 891;
const LEVEL_SETS_ROOT: &str = "tmp/level_sets";

pub struct App {
    window: Option<Arc<Window>>,
    pub(crate) pixels: Option<Pixels<'static>>,
    pub(crate) renderer: Renderer,
    pub(crate) gameplay_presentation: GameplayPresentationState,
    pub(crate) preview_boards: Vec<BoardView>,
    pub(crate) controller: GameplayController,
    pub(crate) app_state: AppState,
    level_persistence: LevelPersistence,
    cursor_position: Option<(f64, f64)>,
    pub(crate) surface_width: u32,
    pub(crate) surface_height: u32,
    pub(crate) editor: LevelEditor,
}

impl App {
    pub fn new() -> Self {
        let initial_levels = load_initial_levels_for_app(std::path::Path::new(LEVEL_SETS_ROOT));
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
        resize_gameplay_surface(&mut app_state.gameplay, INITIAL_WIDTH, INITIAL_HEIGHT);
        set_gameplay_level_sets(
            &mut app_state.gameplay,
            initial_levels.level_set_catalog.clone(),
            initial_levels.active_level_set_index,
        );
        Self {
            window: None,
            pixels: None,
            renderer: Renderer::new(),
            gameplay_presentation: GameplayPresentationState::new(),
            preview_boards,
            controller,
            app_state,
            level_persistence: initial_levels.persistence,
            cursor_position: None,
            surface_width: INITIAL_WIDTH,
            surface_height: INITIAL_HEIGHT,
            editor: LevelEditor::new(),
        }
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

    fn on_gameplay_tap(&mut self, x: f64, y: f64) {
        let input = interpret_gameplay_pointer_tap(&mut self.app_state, &self.controller, x, y);
        self.handle_gameplay_input(input);
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

    fn on_editor_mouse_pressed(&mut self, x: f64, y: f64) {
        editor_mouse_pressed(&mut self.app_state, &mut self.editor, x, y);
        self.render_current();
    }

    fn on_editor_touch(&mut self, id: u64, phase: PointerPhase, x: f64, y: f64) {
        editor_touch(&mut self.app_state, &mut self.editor, id, phase, x, y);
        self.render_current();
    }

    fn on_overlay_tap(&mut self, x: f64, y: f64) {
        let AppInteractionMode::Overlay(overlay) = self.app_state.interaction_mode() else {
            return;
        };
        match overlay.owning_screen() {
            AppScreen::Gameplay => self.on_gameplay_tap(x, y),
            AppScreen::Editor => self.on_editor_mouse_pressed(x, y),
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
}

impl AppDriverContext for App {
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

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = WindowAttributes::default()
            .with_title("Sokobanitron Desktop")
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT));

        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let size = window.inner_size();
        self.surface_width = size.width.max(1);
        self.surface_height = size.height.max(1);
        resize_gameplay_surface(
            &mut self.app_state.gameplay,
            self.surface_width,
            self.surface_height,
        );
        resize_editor_surface(&mut self.app_state, self.surface_width, self.surface_height);

        let surface_texture =
            SurfaceTexture::new(self.surface_width, self.surface_height, window.clone());
        let pixels =
            Pixels::new(self.surface_width, self.surface_height, surface_texture).expect("pixels");

        self.window = Some(window);
        self.pixels = Some(pixels);
        self.render_current();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(PhysicalSize { width, height }) => {
                self.surface_width = width.max(1);
                self.surface_height = height.max(1);
                if let Some(pixels) = &mut self.pixels {
                    pixels
                        .resize_surface(self.surface_width, self.surface_height)
                        .expect("resize surface");
                    pixels
                        .resize_buffer(self.surface_width, self.surface_height)
                        .expect("resize buffer");
                }
                resize_gameplay_surface(
                    &mut self.app_state.gameplay,
                    self.surface_width,
                    self.surface_height,
                );
                resize_editor_surface(&mut self.app_state, self.surface_width, self.surface_height);
                self.render_current();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let (Some(window), Some(pixels)) = (&self.window, &mut self.pixels) {
                    let size = window.inner_size();
                    self.surface_width = size.width.max(1);
                    self.surface_height = size.height.max(1);
                    pixels
                        .resize_surface(self.surface_width, self.surface_height)
                        .expect("resize surface");
                    pixels
                        .resize_buffer(self.surface_width, self.surface_height)
                        .expect("resize buffer");
                    resize_gameplay_surface(
                        &mut self.app_state.gameplay,
                        self.surface_width,
                        self.surface_height,
                    );
                    resize_editor_surface(
                        &mut self.app_state,
                        self.surface_width,
                        self.surface_height,
                    );
                    self.render_current();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some((position.x, position.y));
                if matches!(
                    self.app_state.interaction_mode(),
                    AppInteractionMode::Editor
                ) {
                    editor_cursor_moved(
                        &mut self.app_state,
                        &mut self.editor,
                        position.x,
                        position.y,
                    );
                    self.render_current();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some((cursor_x, cursor_y)) = self.cursor_position {
                    match self.app_state.interaction_mode() {
                        AppInteractionMode::Gameplay => self.on_gameplay_tap(cursor_x, cursor_y),
                        AppInteractionMode::Editor => {
                            self.on_editor_mouse_pressed(cursor_x, cursor_y)
                        }
                        AppInteractionMode::Overlay(_) => self.on_overlay_tap(cursor_x, cursor_y),
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                if self.app_state.active_screen() == AppScreen::Editor {
                    editor_mouse_released(&mut self.app_state);
                }
            }
            WindowEvent::Touch(touch) => {
                let phase = match touch.phase {
                    TouchPhase::Started => PointerPhase::Started,
                    TouchPhase::Moved => PointerPhase::Moved,
                    TouchPhase::Ended => PointerPhase::Ended,
                    TouchPhase::Cancelled => PointerPhase::Cancelled,
                };
                match self.app_state.interaction_mode() {
                    AppInteractionMode::Gameplay => {
                        self.on_gameplay_pointer_event(
                            touch.id,
                            phase,
                            touch.location.x,
                            touch.location.y,
                        );
                    }
                    AppInteractionMode::Editor => {
                        self.on_editor_touch(touch.id, phase, touch.location.x, touch.location.y);
                    }
                    AppInteractionMode::Overlay(_) => {
                        self.on_overlay_pointer_event(
                            touch.id,
                            phase,
                            touch.location.x,
                            touch.location.y,
                        );
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if !matches!(
                    self.app_state.interaction_mode(),
                    AppInteractionMode::Gameplay
                ) || event.state != ElementState::Pressed
                    || event.repeat
                {
                    return;
                }
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        let _ = self.apply_app_input(AppInput::KeyRestart);
                    }
                    Key::Named(NamedKey::Backspace) => {
                        let _ = self.apply_app_input(AppInput::KeyUndo);
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_current();
            }
            _ => {}
        }
    }
}
