use pixels::{Pixels, SurfaceTexture};
use presentation::layout::{BoardViewport, fit_board_viewport_for_controls};
use presentation::renderer::Renderer;
use sokobanitron_app::{
    app::{
        AppAction, AppDriverContext, AppInput, AppState, apply_action_and_present_in_context,
        interpret_input,
    },
    editor::{
        editor_cursor_moved, editor_mouse_pressed, editor_mouse_released, editor_touch,
        reset_editor_interaction_state, resize_editor_surface,
    },
    gameplay::{
        build_gameplay_policy_context, build_gameplay_surface_model, gameplay_pointer_event,
        gameplay_pointer_tap,
    },
    level_bootstrap::load_initial_levels_for_app,
    shared::PointerPhase,
};
use sokobanitron_gameplay::{
    BoardView, GameplayController, GameplayControllerChanges, GameplayPreferences,
};
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
const PREFERENCES_PATH: &str = "desktop-client-preferences.json";

pub struct App {
    window: Option<Arc<Window>>,
    pub(crate) pixels: Option<Pixels<'static>>,
    pub(crate) renderer: Renderer,
    pub(crate) preview_boards: Vec<BoardView>,
    pub(crate) controller: GameplayController,
    pub(crate) app_state: AppState,
    preferences: GameplayPreferences,
    pub(crate) board_viewport: BoardViewport,
    cursor_position: Option<(f64, f64)>,
    pub(crate) surface_width: u32,
    pub(crate) surface_height: u32,
    pub(crate) editor: LevelEditor,
}

impl App {
    pub fn new() -> Self {
        let preferences = GameplayPreferences::load(PREFERENCES_PATH);
        let initial_levels = load_initial_levels_for_app();
        let levels = initial_levels.levels;
        let preview_boards = initial_levels.preview_boards;
        let controller =
            GameplayController::new(levels.clone(), preferences.level_index(levels.len()));
        let board_viewport =
            Self::compute_viewport(INITIAL_WIDTH, INITIAL_HEIGHT, controller.board());
        let mut app_state = AppState::default();
        app_state.editor_available = true;
        Self {
            window: None,
            pixels: None,
            renderer: Renderer::new(),
            preview_boards,
            controller,
            app_state,
            preferences,
            board_viewport,
            cursor_position: None,
            surface_width: INITIAL_WIDTH,
            surface_height: INITIAL_HEIGHT,
            editor: LevelEditor::new(),
        }
    }

    fn compute_viewport(
        width: u32,
        height: u32,
        board: &sokobanitron_gameplay::BoardView,
    ) -> BoardViewport {
        fit_board_viewport_for_controls(width, height, board)
    }

    fn update_viewport(&mut self) {
        self.board_viewport = Self::compute_viewport(
            self.surface_width,
            self.surface_height,
            self.controller.board(),
        );
    }

    fn handle_gameplay_changes(&mut self, changes: GameplayControllerChanges) {
        if let Some(index) = changes.last_attempted_level_changed {
            self.preferences.last_started_level = Some(index + 1);
            if let Err(err) = self.preferences.save(PREFERENCES_PATH) {
                eprintln!("warning: failed to persist preferences: {err}");
            }
        }
        if changes.level_changed.is_some() {
            self.update_viewport();
        }
    }

    fn apply_app_action(&mut self, action: AppAction) {
        if let Ok(applied) = apply_action_and_present_in_context(self, action) {
            self.handle_gameplay_changes(applied.changes);
        }
    }

    fn apply_app_input(&mut self, input: AppInput) {
        let action = interpret_input(&self.app_state, input);
        self.apply_app_action(action);
    }

    fn enter_editor_mode(&mut self) {
        self.apply_app_input(AppInput::EnterEditorMode);
        reset_editor_interaction_state(&mut self.app_state);
    }

    fn handle_gameplay_input(&mut self, input: AppInput) {
        match input {
            AppInput::NoOp => {}
            AppInput::EnterEditorMode => {
                self.enter_editor_mode();
                self.render_current();
            }
            AppInput::BoardTap { .. } => self.apply_app_input(input),
            _ => {
                self.apply_app_input(input);
                self.render_active_gameplay_screen();
            }
        }
    }

    fn on_gameplay_tap(&mut self, x: f64, y: f64) {
        let surface = build_gameplay_surface_model(
            &self.app_state,
            &self.controller,
            self.surface_width,
            self.surface_height,
            self.board_viewport,
        );
        let policy = build_gameplay_policy_context(&self.app_state, &self.controller);
        let input = gameplay_pointer_tap(&mut self.app_state.gameplay, &surface, policy, x, y);
        self.handle_gameplay_input(input);
    }

    fn on_gameplay_pointer_event(&mut self, id: u64, phase: PointerPhase, x: f64, y: f64) {
        let surface = build_gameplay_surface_model(
            &self.app_state,
            &self.controller,
            self.surface_width,
            self.surface_height,
            self.board_viewport,
        );
        let policy = build_gameplay_policy_context(&self.app_state, &self.controller);
        let input = gameplay_pointer_event(
            &mut self.app_state.gameplay,
            &surface,
            policy,
            id,
            phase,
            x,
            y,
        );
        self.handle_gameplay_input(input);
    }
}

impl AppDriverContext for App {
    fn controller_and_app_state_mut(&mut self) -> (&mut GameplayController, &mut AppState) {
        (&mut self.controller, &mut self.app_state)
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
        self.update_viewport();
        resize_editor_surface(&mut self.app_state, self.surface_width, self.surface_height);

        let surface_texture =
            SurfaceTexture::new(self.surface_width, self.surface_height, window.clone());
        let pixels =
            Pixels::new(self.surface_width, self.surface_height, surface_texture).expect("pixels");

        self.window = Some(window);
        self.pixels = Some(pixels);
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
                self.update_viewport();
                resize_editor_surface(&mut self.app_state, self.surface_width, self.surface_height);
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
                    self.update_viewport();
                    resize_editor_surface(
                        &mut self.app_state,
                        self.surface_width,
                        self.surface_height,
                    );
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some((position.x, position.y));
                if self.app_state.is_editor_screen() && !self.app_state.is_editor_menu_open() {
                    editor_cursor_moved(
                        &mut self.app_state,
                        &mut self.editor,
                        position.x,
                        position.y,
                    );
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some((cursor_x, cursor_y)) = self.cursor_position {
                    if self.app_state.is_editor_screen() {
                        editor_mouse_pressed(
                            &mut self.app_state,
                            &mut self.editor,
                            cursor_x,
                            cursor_y,
                        );
                        self.render_current();
                        return;
                    }
                    if self.app_state.is_gameplay_screen() {
                        self.on_gameplay_tap(cursor_x, cursor_y);
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                if self.app_state.is_editor_screen() {
                    editor_mouse_released(&mut self.app_state);
                }
            }
            WindowEvent::Touch(touch) => {
                if self.app_state.is_editor_screen() {
                    let phase = match touch.phase {
                        TouchPhase::Started => PointerPhase::Started,
                        TouchPhase::Moved => PointerPhase::Moved,
                        TouchPhase::Ended => PointerPhase::Ended,
                        TouchPhase::Cancelled => PointerPhase::Cancelled,
                    };
                    editor_touch(
                        &mut self.app_state,
                        &mut self.editor,
                        touch.id,
                        phase,
                        touch.location.x,
                        touch.location.y,
                    );
                    self.render_current();
                } else if self.app_state.is_gameplay_screen() {
                    let phase = match touch.phase {
                        TouchPhase::Started => PointerPhase::Started,
                        TouchPhase::Moved => PointerPhase::Moved,
                        TouchPhase::Ended => PointerPhase::Ended,
                        TouchPhase::Cancelled => PointerPhase::Cancelled,
                    };
                    self.on_gameplay_pointer_event(
                        touch.id,
                        phase,
                        touch.location.x,
                        touch.location.y,
                    );
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if !self.app_state.is_gameplay_screen() {
                    return;
                }
                if event.state != ElementState::Pressed || event.repeat {
                    return;
                }
                if self.app_state.is_overlay_open() {
                    return;
                }
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => self.apply_app_input(AppInput::KeyRestart),
                    Key::Named(NamedKey::Backspace) => self.apply_app_input(AppInput::KeyUndo),
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                self.render_current();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
