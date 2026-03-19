use pixels::{Pixels, SurfaceTexture};
use renderer::{BoardViewport, Renderer};
use sokobanitron_gameplay::{GameplayKey, GameplaySession};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes},
};

const INITIAL_WIDTH: u32 = 960;
const INITIAL_HEIGHT: u32 = 640;

struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    renderer: Renderer,
    session: GameplaySession,
    board_viewport: BoardViewport,
    cursor_position: Option<(f64, f64)>,
    surface_width: u32,
    surface_height: u32,
}

impl App {
    fn new() -> Self {
        let session = GameplaySession::new_default_level();
        let board_viewport =
            BoardViewport::fit_to_window(INITIAL_WIDTH, INITIAL_HEIGHT, session.board());
        Self {
            window: None,
            pixels: None,
            renderer: Renderer::new(),
            session,
            board_viewport,
            cursor_position: None,
            surface_width: INITIAL_WIDTH,
            surface_height: INITIAL_HEIGHT,
        }
    }

    fn update_viewport(&mut self) {
        self.board_viewport = BoardViewport::fit_to_window(
            self.surface_width,
            self.surface_height,
            self.session.board(),
        );
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
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some((position.x, position.y));
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some((cursor_x, cursor_y)) = self.cursor_position {
                    if let Some((x, y)) =
                        self.board_viewport
                            .screen_to_cell(cursor_x, cursor_y, self.session.board())
                    {
                        self.session.click_cell(x, y);
                        self.update_viewport();
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed || event.repeat {
                    return;
                }
                let key = match event.logical_key {
                    Key::Named(NamedKey::Escape) => GameplayKey::Escape,
                    Key::Named(NamedKey::Backspace) => GameplayKey::Backspace,
                    _ => GameplayKey::Other,
                };
                self.session.on_key(key);
                self.update_viewport();
            }
            WindowEvent::RedrawRequested => {
                if let Some(pixels) = &mut self.pixels {
                    let frame = pixels.frame_mut();
                    self.renderer.draw(
                        frame,
                        self.surface_width,
                        self.surface_height,
                        self.session.board(),
                        &self.board_viewport,
                    );
                    pixels.render().expect("render");
                }
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

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run app");
}
