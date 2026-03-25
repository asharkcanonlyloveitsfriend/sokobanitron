use crate::{config, platform};
use presentation::layout::{BoardViewport, fit_board_viewport_for_controls};
use presentation::renderer::Renderer;
use sokobanitron_app::{
    AppAction, AppDriverContext, AppInput, AppState, GameplayInputContext,
    apply_action_and_present_in_context, gameplay_pointer_tap, interpret_input,
    is_gameplay_menu_open, is_gameplay_screen, is_level_select_open, is_overlay_open,
    level_select_page_start, load_initial_levels_for_app,
};
use sokobanitron_gameplay::{
    BoardView, GameplayController, GameplayControllerChanges, GameplayPreferences,
};
use std::io::Result;

pub struct KindleApp {
    pub(crate) renderer: Renderer,
    pub(crate) rgba_frame: Vec<u8>,
    levels: Vec<String>,
    pub(crate) preview_boards: Vec<BoardView>,
    pub(crate) controller: GameplayController,
    pub(crate) app_state: AppState,
    preferences: GameplayPreferences,
    pub(crate) viewport: BoardViewport,
    pub(crate) display: platform::Display,
}

impl KindleApp {
    pub fn new() -> Result<Self> {
        let initial_levels = load_initial_levels_for_app();
        let levels = initial_levels.levels;
        let preview_boards = initial_levels.preview_boards;
        let preferences = GameplayPreferences::load(config::PREFERENCES_PATH);
        let last_attempted_level = preferences.level_index(levels.len());
        let controller = GameplayController::new(levels.clone(), last_attempted_level);
        let viewport = Self::compute_viewport(controller.board());
        Ok(Self {
            renderer: Self::build_renderer(),
            rgba_frame: vec![0; config::WIDTH * config::HEIGHT * 4],
            levels,
            preview_boards,
            controller,
            app_state: AppState::default(),
            preferences,
            viewport,
            display: platform::Display::new()?,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.render()?;

        let mut touch = platform::TouchReader::new()?;
        loop {
            match touch.next_input_event()? {
                platform::AppInputEvent::Tap(raw_x, raw_y) => self.on_tap(raw_x, raw_y)?,
                platform::AppInputEvent::PowerShortPress => {
                    self.display.force_full_refresh_next();
                    self.render()?;
                }
                platform::AppInputEvent::PowerLongPress => {
                    if let Err(err) = platform::start_lab126_gui() {
                        eprintln!("warning: failed to restart lab126_gui: {err}");
                    }
                    return Ok(());
                }
            }
        }
    }

    fn update_viewport(&mut self) {
        self.viewport = Self::compute_viewport(self.controller.board());
    }

    fn handle_gameplay_changes(&mut self, changes: GameplayControllerChanges) {
        if let Some(index) = changes.last_attempted_level_changed {
            self.preferences.last_started_level = Some(index + 1);
            if let Err(err) = self.preferences.save(config::PREFERENCES_PATH) {
                eprintln!("warning: failed to persist preferences: {err}");
            }
        }
        if changes.level_changed.is_some() {
            self.update_viewport();
        }
    }

    fn apply_app_action(&mut self, action: AppAction) -> Result<()> {
        let applied = apply_action_and_present_in_context(self, action)?;
        self.handle_gameplay_changes(applied.changes);
        Ok(())
    }

    fn apply_app_input(&mut self, input: AppInput) -> Result<()> {
        let action = interpret_input(&self.app_state, input);
        self.apply_app_action(action)
    }

    fn compute_viewport(board: &sokobanitron_gameplay::BoardView) -> BoardViewport {
        fit_board_viewport_for_controls(config::WIDTH as u32, config::HEIGHT as u32, board)
    }

    fn on_gameplay_tap(&mut self, screen_x: f64, screen_y: f64) -> Result<()> {
        let context = GameplayInputContext {
            allow_enter_editor: self.app_state.editor_available,
            is_gameplay_screen: is_gameplay_screen(&self.app_state),
            is_gameplay_menu_open: is_gameplay_menu_open(&self.app_state),
            is_level_select_open: is_level_select_open(&self.app_state),
            is_overlay_open: is_overlay_open(&self.app_state),
            surface_width: config::WIDTH as u32,
            surface_height: config::HEIGHT as u32,
            level_count: self.levels.len(),
            current_level: self.controller.current_level(),
            current_level_select_page_start: level_select_page_start(&self.app_state).unwrap_or(0),
            can_undo: self.controller.can_undo(),
            can_restart: self.controller.can_restart(),
            is_solved: self.controller.board().is_solved(),
            board_viewport: self.viewport,
            board: self.controller.board(),
        };
        let input = gameplay_pointer_tap(&mut self.app_state.gameplay, context, screen_x, screen_y);

        match input {
            AppInput::NoOp => Ok(()),
            AppInput::BoardTap { .. } => self.apply_app_input(input),
            _ => {
                self.apply_app_input(input)?;
                self.render()
            }
        }
    }

    fn on_tap(&mut self, raw_x: i32, raw_y: i32) -> Result<()> {
        let (screen_x, screen_y) = platform::map_touch_to_screen(raw_x, raw_y)?;
        self.on_gameplay_tap(screen_x as f64, screen_y as f64)
    }
}

impl AppDriverContext for KindleApp {
    fn controller_and_app_state_mut(&mut self) -> (&mut GameplayController, &mut AppState) {
        (&mut self.controller, &mut self.app_state)
    }
}
