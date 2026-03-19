use pixels::{Pixels, SurfaceTexture};
use renderer::{
    BoardViewport, ControlsButtonAction, Renderer, controls_button_action_at, draw_controls_ui,
    fit_board_viewport_for_controls,
};
use sokobanitron_gameplay::{
    BoxMovedTrailPresentation, BoxRemovedPresentation, GameplayController,
    GameplayControllerChanges, GameplayKey, GameplayPreferences, GameplayPresentMode,
    GameplayTapPresentationPlan, GameplayTapPresentationStep, GameplayTapPresentationStyle,
    OrientationPolicy, build_tap_presentation_plan, load_levels_from_default_locations,
};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes},
};

const INITIAL_WIDTH: u32 = 670;
const INITIAL_HEIGHT: u32 = 891;
const DEFAULT_LEVEL_LINES: [&str; 4] = ["    ###   ", " $$     #@", " $ #...   ", "   #######"];
const ANIMATION_TICK_MS: u64 = 50;
const BOX_PATH_SPEED_SCALE: f32 = 1.3;
const BOX_PATH_SPEED_EXPONENT: f32 = 0.5;
const BLINK_ON_MS: u64 = 120;
const PREFERENCES_PATH: &str = "desktop-client-preferences.json";
const DESKTOP_TAP_STYLE: GameplayTapPresentationStyle = GameplayTapPresentationStyle {
    box_removed_presentation: BoxRemovedPresentation::RenderThenBlink,
    long_box_path_presentation: BoxMovedTrailPresentation::AnimatePathDisappear,
    delayed_win_present_mode: GameplayPresentMode::Full,
};

fn initial_levels() -> Vec<String> {
    let fallback = DEFAULT_LEVEL_LINES.join("\n");
    let levels =
        load_levels_from_default_locations(OrientationPolicy::RotateWideToPortrait, &fallback);
    if levels.is_empty() {
        vec![fallback]
    } else {
        levels
    }
}

struct App {
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    renderer: Renderer,
    controller: GameplayController,
    preferences: GameplayPreferences,
    board_viewport: BoardViewport,
    cursor_position: Option<(f64, f64)>,
    surface_width: u32,
    surface_height: u32,
}

impl App {
    fn new() -> Self {
        let preferences = GameplayPreferences::load(PREFERENCES_PATH);
        let levels = initial_levels();
        let controller =
            GameplayController::new(levels.clone(), preferences.level_index(levels.len()));
        let board_viewport =
            Self::compute_viewport(INITIAL_WIDTH, INITIAL_HEIGHT, controller.board());
        Self {
            window: None,
            pixels: None,
            renderer: Renderer::new(),
            controller,
            preferences,
            board_viewport,
            cursor_position: None,
            surface_width: INITIAL_WIDTH,
            surface_height: INITIAL_HEIGHT,
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

    fn render_with_options(
        &mut self,
        box_trail: Option<&[(u32, u32)]>,
        draw_player: bool,
        show_win_overlay: bool,
    ) {
        if let Some(pixels) = &mut self.pixels {
            let frame = pixels.frame_mut();
            self.renderer.draw_with_box_trail_options(
                frame,
                self.surface_width,
                self.surface_height,
                self.controller.board(),
                &self.board_viewport,
                box_trail,
                draw_player,
                show_win_overlay,
            );
            draw_controls_ui(frame, self.surface_width, self.surface_height);
            pixels.render().expect("render");
        }
    }

    fn animate_player_blink(&mut self) {
        if let Some(pixels) = &mut self.pixels {
            let frame = pixels.frame_mut();
            self.renderer.draw_with_box_trail_progress_effects(
                frame,
                self.surface_width,
                self.surface_height,
                self.controller.board(),
                &self.board_viewport,
                None,
                None,
                true,
                true,
                false,
            );
            draw_controls_ui(frame, self.surface_width, self.surface_height);
            pixels.render().expect("render");
        }
        thread::sleep(Duration::from_millis(BLINK_ON_MS));
        self.render_with_options(None, true, true);
    }

    fn animate_box_path_disappear(&mut self, path: &[(u32, u32)], show_win_overlay: bool) {
        if path.len() <= 2 {
            self.render_with_options(None, true, show_win_overlay);
            return;
        }

        let total_segments = (path.len() - 1) as f32;
        let speed_per_tick = BOX_PATH_SPEED_SCALE * total_segments.powf(BOX_PATH_SPEED_EXPONENT);
        let mut consumed = 0.0f32;
        while consumed < total_segments {
            if let Some(pixels) = &mut self.pixels {
                let frame = pixels.frame_mut();
                self.renderer.draw_with_box_trail_progress_options(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    self.controller.board(),
                    &self.board_viewport,
                    Some(path),
                    Some(consumed),
                    false,
                    show_win_overlay,
                );
                draw_controls_ui(frame, self.surface_width, self.surface_height);
                pixels.render().expect("render");
            }
            thread::sleep(Duration::from_millis(ANIMATION_TICK_MS));
            consumed += speed_per_tick;
        }

        self.render_with_options(None, true, show_win_overlay);
    }

    fn execute_tap_presentation_plan(&mut self, plan: GameplayTapPresentationPlan) {
        for step in plan.steps {
            match step {
                GameplayTapPresentationStep::Render {
                    box_trail,
                    draw_player,
                    show_win_overlay,
                    ..
                } => {
                    self.render_with_options(box_trail.as_deref(), draw_player, show_win_overlay);
                }
                GameplayTapPresentationStep::AnimatePlayerBlink => self.animate_player_blink(),
                GameplayTapPresentationStep::AnimateBoxVanish { .. } => {}
                GameplayTapPresentationStep::AnimateBoxPathDisappear {
                    path,
                    show_win_overlay,
                } => self.animate_box_path_disappear(&path, show_win_overlay),
            }
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
                    if let Some(action) = controls_button_action_at(
                        cursor_x,
                        cursor_y,
                        self.surface_width,
                        self.surface_height,
                    ) {
                        match action {
                            ControlsButtonAction::Restart => {
                                let changes = self.controller.restart_with_changes();
                                self.handle_gameplay_changes(changes);
                                self.render_with_options(None, true, true);
                            }
                            ControlsButtonAction::Undo => {
                                let changes = self.controller.undo_with_changes();
                                self.handle_gameplay_changes(changes);
                                self.render_with_options(None, true, true);
                            }
                            ControlsButtonAction::ShowMenu => {}
                        }
                        return;
                    }

                    if self.controller.board().is_won() {
                        if let Some(next) = self.controller.peek_level(1) {
                            let changes = self.controller.advance_after_win(next);
                            self.handle_gameplay_changes(changes);
                            self.render_with_options(None, true, true);
                        }
                        return;
                    }

                    if let Some((x, y)) = self.board_viewport.screen_to_cell(
                        cursor_x,
                        cursor_y,
                        self.controller.board(),
                    ) {
                        let tap_outcome = self.controller.click_cell_with_outcome(x, y);
                        self.handle_gameplay_changes(tap_outcome.changes);
                        let plan = build_tap_presentation_plan(
                            &tap_outcome,
                            self.preferences.show_box_path,
                            DESKTOP_TAP_STYLE,
                        );
                        self.execute_tap_presentation_plan(plan);
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
                let changes = self.controller.on_key_with_changes(key);
                self.handle_gameplay_changes(changes);
            }
            WindowEvent::RedrawRequested => {
                self.render_with_options(None, true, true);
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
