use pixels::{Pixels, SurfaceTexture};
use sokobanitron_app::{
    app::{AppInput, AppPointerInput, AppliedUpdate, SharedAppRuntime, SharedAppRuntimeConfig},
    level_bootstrap::load_initial_levels_for_app,
    shared::PointerPhase,
};
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
    pub(crate) runtime: SharedAppRuntime,
    cursor_position: Option<(f64, f64)>,
}

impl App {
    pub fn new() -> io::Result<Self> {
        let initial_levels = load_initial_levels_for_app(std::path::Path::new(LEVEL_SETS_ROOT))?;
        Ok(Self {
            window: None,
            pixels: None,
            runtime: SharedAppRuntime::new(
                initial_levels,
                INITIAL_WIDTH,
                INITIAL_HEIGHT,
                desktop_runtime_config(),
            ),
            cursor_position: None,
        })
    }

    fn apply_app_input(&mut self, input: AppInput) -> Option<AppliedUpdate> {
        let applied = self.apply_input_and_render(input).ok();
        if applied
            .as_ref()
            .is_some_and(|update| update.render_work.needs_followup_wake)
        {
            self.request_window_redraw();
        }
        applied
    }

    fn handle_pointer_input(&mut self, input: AppPointerInput) {
        let followup = self
            .handle_pointer_input_and_render(input)
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

fn desktop_runtime_config() -> SharedAppRuntimeConfig {
    SharedAppRuntimeConfig {
        editor_available: true,
        supports_multi_touch: false,
        ..SharedAppRuntimeConfig::default()
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = WindowAttributes::default()
            .with_title("Sokobanitron Desktop")
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT));

        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let size = window.inner_size();
        self.runtime.resize_surface(size.width, size.height);

        let surface_texture = SurfaceTexture::new(
            self.runtime.surface_width(),
            self.runtime.surface_height(),
            window.clone(),
        );
        let pixels = Pixels::new(
            self.runtime.surface_width(),
            self.runtime.surface_height(),
            surface_texture,
        )
        .expect("pixels");

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
                self.runtime.resize_surface(width, height);
                if let Some(pixels) = &mut self.pixels {
                    pixels
                        .resize_surface(self.runtime.surface_width(), self.runtime.surface_height())
                        .expect("resize surface");
                    pixels
                        .resize_buffer(self.runtime.surface_width(), self.runtime.surface_height())
                        .expect("resize buffer");
                }
                self.render_current();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let (Some(window), Some(pixels)) = (&self.window, &mut self.pixels) {
                    let size = window.inner_size();
                    self.runtime.resize_surface(size.width, size.height);
                    pixels
                        .resize_surface(self.runtime.surface_width(), self.runtime.surface_height())
                        .expect("resize surface");
                    pixels
                        .resize_buffer(self.runtime.surface_width(), self.runtime.surface_height())
                        .expect("resize buffer");
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
                let work = self
                    .continue_pending_render_work_and_render()
                    .ok()
                    .unwrap_or_default();
                if !work.frame_changed && !work.needs_followup_wake {
                    self.render_current();
                }
                if work.needs_followup_wake {
                    self.request_window_redraw();
                }
            }
            _ => {}
        }
    }
}
