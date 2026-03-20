use pixels::{Pixels, SurfaceTexture};
use renderer::{
    BoardViewport, ControlsButtonAction, Renderer, controls_button_action_at,
    fit_board_viewport_for_controls, level_select_menu_nav_action_at,
    level_select_menu_start_for_nav, level_select_menu_start_index, level_select_menu_target_at,
};
use sokobanitron_app::{
    AppAction, AppDriverContext, AppInput, AppState, BoxPathStyle, BoxRemovedStyle, PresentMode,
    PresentationProfile, apply_action_and_present_in_context, interpret_input, is_menu_open,
    menu_page_start,
};
use sokobanitron_gameplay::{
    BoardView, GameplayController, GameplayControllerChanges, GameplayPreferences,
    OrientationPolicy, load_levels_from_default_locations,
};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowAttributes},
};

const INITIAL_WIDTH: u32 = 670;
const INITIAL_HEIGHT: u32 = 891;
const DEFAULT_LEVEL_LINES: [&str; 4] = ["    ###   ", " $$     #@", " $ #...   ", "   #######"];
const PREFERENCES_PATH: &str = "desktop-client-preferences.json";
const DESKTOP_PRESENTATION_PROFILE: PresentationProfile = PresentationProfile {
    box_removed_style: BoxRemovedStyle::RenderThenBlink,
    box_path_style: BoxPathStyle::AnimatePathDisappear,
    delayed_solved_present_mode: PresentMode::Full,
    allow_delays: true,
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

pub struct App {
    window: Option<Arc<Window>>,
    pub(crate) pixels: Option<Pixels<'static>>,
    pub(crate) renderer: Renderer,
    levels: Vec<String>,
    pub(crate) preview_boards: Vec<BoardView>,
    pub(crate) controller: GameplayController,
    pub(crate) app_state: AppState,
    preferences: GameplayPreferences,
    pub(crate) board_viewport: BoardViewport,
    cursor_position: Option<(f64, f64)>,
    pub(crate) surface_width: u32,
    pub(crate) surface_height: u32,
}

impl App {
    fn build_preview_board(level_ascii: &str) -> BoardView {
        GameplayController::new(vec![level_ascii.to_string()], None)
            .board()
            .clone()
    }

    pub fn new() -> Self {
        let preferences = GameplayPreferences::load(PREFERENCES_PATH);
        let levels = initial_levels();
        let preview_boards = levels
            .iter()
            .map(|level| Self::build_preview_board(level))
            .collect::<Vec<_>>();
        let controller =
            GameplayController::new(levels.clone(), preferences.level_index(levels.len()));
        let board_viewport =
            Self::compute_viewport(INITIAL_WIDTH, INITIAL_HEIGHT, controller.board());
        Self {
            window: None,
            pixels: None,
            renderer: Renderer::new(),
            levels,
            preview_boards,
            controller,
            app_state: AppState::default(),
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

    fn build_effective_presentation_profile(&self) -> PresentationProfile {
        PresentationProfile {
            box_removed_style: DESKTOP_PRESENTATION_PROFILE.box_removed_style,
            box_path_style: if self.preferences.show_box_path {
                DESKTOP_PRESENTATION_PROFILE.box_path_style
            } else {
                BoxPathStyle::Hidden
            },
            delayed_solved_present_mode: DESKTOP_PRESENTATION_PROFILE.delayed_solved_present_mode,
            allow_delays: DESKTOP_PRESENTATION_PROFILE.allow_delays,
        }
    }

    fn apply_app_action(&mut self, action: AppAction) {
        let profile = self.build_effective_presentation_profile();
        if let Ok(applied) = apply_action_and_present_in_context(self, action, &profile) {
            self.handle_gameplay_changes(applied.changes);
        }
    }

    fn apply_app_input(&mut self, input: AppInput) {
        let action = interpret_input(&self.app_state, input);
        self.apply_app_action(action);
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
                                if !is_menu_open(&self.app_state) {
                                    self.apply_app_input(AppInput::ControlRestart);
                                    self.render_with_options(None, true, true);
                                    return;
                                }
                            }
                            ControlsButtonAction::Undo => {
                                if !is_menu_open(&self.app_state) {
                                    self.apply_app_input(AppInput::ControlUndo);
                                    self.render_with_options(None, true, true);
                                    return;
                                }
                            }
                            ControlsButtonAction::ShowMenu => {
                                self.apply_app_input(AppInput::ControlToggleMenu);
                                if is_menu_open(&self.app_state) {
                                    let page_start = level_select_menu_start_index(
                                        self.levels.len(),
                                        self.controller.current_level(),
                                    );
                                    self.apply_app_input(AppInput::MenuNavigate { page_start });
                                }
                                self.render_with_options(None, true, true);
                                return;
                            }
                        }
                    }

                    if is_menu_open(&self.app_state) {
                        if let Some(nav_action) = level_select_menu_nav_action_at(
                            cursor_x,
                            cursor_y,
                            self.surface_width,
                            self.surface_height,
                        ) {
                            let menu_page_start = menu_page_start(&self.app_state).unwrap_or(0);
                            let page_start = level_select_menu_start_for_nav(
                                self.levels.len(),
                                self.controller.current_level(),
                                menu_page_start,
                                nav_action,
                            );
                            self.apply_app_input(AppInput::MenuNavigate { page_start });
                            self.render_with_options(None, true, true);
                            return;
                        }

                        let menu_page_start = menu_page_start(&self.app_state).unwrap_or(0);
                        if let Some(target) = level_select_menu_target_at(
                            cursor_x,
                            cursor_y,
                            self.surface_width,
                            self.surface_height,
                            self.levels.len(),
                            menu_page_start,
                        ) {
                            self.apply_app_input(AppInput::MenuSelectLevel(target));
                            self.render_with_options(None, true, true);
                        }
                        return;
                    }

                    if self.controller.board().is_won() {
                        self.apply_app_input(AppInput::SolvedAdvance);
                        self.render_with_options(None, true, true);
                        return;
                    }

                    if let Some((x, y)) = self.board_viewport.screen_to_cell(
                        cursor_x,
                        cursor_y,
                        self.controller.board(),
                    ) {
                        self.apply_app_input(AppInput::BoardTap { x, y });
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed || event.repeat {
                    return;
                }
                if is_menu_open(&self.app_state) {
                    return;
                }
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => self.apply_app_input(AppInput::KeyRestart),
                    Key::Named(NamedKey::Backspace) => self.apply_app_input(AppInput::KeyUndo),
                    _ => {}
                }
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
