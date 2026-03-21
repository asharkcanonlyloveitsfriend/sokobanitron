use crate::constants::{INITIAL_HEIGHT, INITIAL_WIDTH};
use crate::session::{LevelCreatorSession, TouchInputPhase};
use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, MouseButton, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes},
};

pub struct LevelCreatorApp {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    surface_width: u32,
    surface_height: u32,
    session: LevelCreatorSession,
}

impl LevelCreatorApp {
    pub fn new() -> Self {
        Self {
            window: None,
            pixels: None,
            surface_width: INITIAL_WIDTH,
            surface_height: INITIAL_HEIGHT,
            session: LevelCreatorSession::new(),
        }
    }

    fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_width = width.max(1);
        self.surface_height = height.max(1);
        self.session
            .resize_surface(self.surface_width, self.surface_height);

        if let Some(pixels) = &mut self.pixels {
            pixels
                .resize_surface(self.surface_width, self.surface_height)
                .expect("resize surface");
            pixels
                .resize_buffer(self.surface_width, self.surface_height)
                .expect("resize buffer");
        }
    }

    fn render(&mut self) {
        if let Some(pixels) = &mut self.pixels {
            let frame = pixels.frame_mut();
            self.session
                .render(frame, self.surface_width, self.surface_height);
            pixels.render().expect("render");
        }
    }

    fn map_touch_phase(phase: TouchPhase) -> TouchInputPhase {
        match phase {
            TouchPhase::Started => TouchInputPhase::Started,
            TouchPhase::Moved => TouchInputPhase::Moved,
            TouchPhase::Ended => TouchInputPhase::Ended,
            TouchPhase::Cancelled => TouchInputPhase::Cancelled,
        }
    }
}

impl Default for LevelCreatorApp {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler for LevelCreatorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = WindowAttributes::default()
            .with_title("Sokobanitron Level Creator")
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT));

        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));
        let size = window.inner_size();
        let surface_width = size.width.max(1);
        let surface_height = size.height.max(1);
        let surface_texture = SurfaceTexture::new(surface_width, surface_height, window.clone());
        let pixels = Pixels::new(surface_width, surface_height, surface_texture).expect("pixels");

        self.window = Some(window);
        self.pixels = Some(pixels);
        self.resize_surface(surface_width, surface_height);
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
                self.resize_surface(width, height);
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    let size = window.inner_size();
                    self.resize_surface(size.width, size.height);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.session.cursor_moved(position.x, position.y);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.session.mouse_pressed_left();
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.session.mouse_released_left();
            }
            WindowEvent::Touch(touch) => {
                self.session.touch(
                    touch.id,
                    Self::map_touch_phase(touch.phase),
                    touch.location.x,
                    touch.location.y,
                );
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed
                    && !event.repeat
                    && matches!(event.logical_key, Key::Named(NamedKey::Escape))
                {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
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

pub fn run() {
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = LevelCreatorApp::new();
    event_loop.run_app(&mut app).expect("run app");
}
