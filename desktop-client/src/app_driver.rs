use pixels::{Pixels, SurfaceTexture};
use presentation::{GameplayPresentationState, Renderer};
use sokobanitron_app::{
    app::{
        AppDriverContext, AppInput, AppPointerInput, AppRuntimeMut, AppScreen, AppState,
        AppliedUpdate, EditorAppRuntimeMut, apply_input_and_render_in_context,
        continue_pending_render_work_and_render_in_context,
        handle_pointer_input_and_render_in_context,
    },
    editor::resize_editor_surface,
    gameplay::{resize_gameplay_surface, set_gameplay_level_sets},
    level_bootstrap::load_initial_levels_for_app,
    persistence::LevelPersistence,
    shared::PointerPhase,
};
use sokobanitron_gameplay::{BoardView, GameplayController};
use sokobanitron_level_editor::LevelEditor;
use std::io;
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
    pub(crate) gray_frame: Vec<u8>,
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
    pub fn new() -> io::Result<Self> {
        let initial_levels = load_initial_levels_for_app(std::path::Path::new(LEVEL_SETS_ROOT))?;
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
            Some(initial_levels.active_level_set_index),
        );
        Ok(Self {
            window: None,
            pixels: None,
            gray_frame: vec![0; (INITIAL_WIDTH * INITIAL_HEIGHT) as usize],
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
        })
    }

    fn apply_app_input(&mut self, input: AppInput) -> Option<AppliedUpdate> {
        let applied = apply_input_and_render_in_context(self, input).ok();
        if applied
            .as_ref()
            .is_some_and(|update| update.render_work.needs_followup_wake)
        {
            self.request_window_redraw();
        }
        applied
    }

    fn handle_pointer_input(&mut self, input: AppPointerInput) {
        let followup = handle_pointer_input_and_render_in_context(self, input)
            .map(|work| work.needs_followup_wake)
            .unwrap_or(false);
        if followup {
            self.request_window_redraw();
        }
    }

    pub(crate) fn request_window_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
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
        self.gray_frame = vec![0; (self.surface_width as usize) * (self.surface_height as usize)];
        self.gameplay_presentation.clear();
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
                self.gray_frame =
                    vec![0; (self.surface_width as usize) * (self.surface_height as usize)];
                self.gameplay_presentation.clear();
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
                    self.gray_frame =
                        vec![0; (self.surface_width as usize) * (self.surface_height as usize)];
                    self.gameplay_presentation.clear();
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
                self.handle_pointer_input(AppPointerInput::CursorMoved {
                    x: position.x,
                    y: position.y,
                });
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some((cursor_x, cursor_y)) = self.cursor_position {
                    self.handle_pointer_input(AppPointerInput::MousePressed {
                        x: cursor_x,
                        y: cursor_y,
                    });
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.handle_pointer_input(AppPointerInput::MouseReleased);
            }
            WindowEvent::Touch(touch) => {
                let phase = match touch.phase {
                    TouchPhase::Started => PointerPhase::Started,
                    TouchPhase::Moved => PointerPhase::Moved,
                    TouchPhase::Ended => PointerPhase::Ended,
                    TouchPhase::Cancelled => PointerPhase::Cancelled,
                };
                self.handle_pointer_input(AppPointerInput::Pointer {
                    id: touch.id,
                    phase,
                    x: touch.location.x,
                    y: touch.location.y,
                });
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed || event.repeat {
                    return;
                }
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        let _ = self.apply_app_input(AppInput::Restart);
                    }
                    Key::Named(NamedKey::Backspace) => {
                        let _ = self.apply_app_input(AppInput::Undo);
                    }
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => {
                if self.app_state.active_screen() == AppScreen::Gameplay
                    && self.gameplay_presentation.has_pending_presentation()
                {
                    self.render_active_gameplay_presentation();
                } else {
                    let work = continue_pending_render_work_and_render_in_context(self)
                        .ok()
                        .unwrap_or_default();
                    if !work.frame_changed {
                        self.render_current();
                    }
                    if work.needs_followup_wake {
                        self.request_window_redraw();
                    }
                }
            }
            _ => {}
        }
    }
}
