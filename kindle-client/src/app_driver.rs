use crate::{config, platform};
use presentation::{GameplayPresentationState, Renderer};
use sokobanitron_app::{
    app::{
        AppAction, AppDriverContext, AppInput, AppState, apply_action_in_context, interpret_input,
        render_presentation_plan,
    },
    gameplay::{
        build_gameplay_policy_context, build_gameplay_surface_model, gameplay_pointer_tap,
        resize_gameplay_surface,
    },
    level_bootstrap::load_initial_levels_for_app,
};
use sokobanitron_gameplay::{
    BoardView, GameplayController, GameplayControllerChanges, GameplayPreferences,
};
use std::io::Result;

pub struct KindleApp {
    pub(crate) renderer: Renderer,
    pub(crate) gameplay_presentation: GameplayPresentationState,
    pub(crate) rgba_frame: Vec<u8>,
    pub(crate) preview_boards: Vec<BoardView>,
    pub(crate) controller: GameplayController,
    pub(crate) app_state: AppState,
    preferences: GameplayPreferences,
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
        let mut app_state = AppState::default();
        resize_gameplay_surface(
            &mut app_state.gameplay,
            config::WIDTH as u32,
            config::HEIGHT as u32,
        );
        Ok(Self {
            renderer: Self::build_renderer(),
            gameplay_presentation: GameplayPresentationState::new(),
            rgba_frame: vec![0; config::WIDTH * config::HEIGHT * 4],
            preview_boards,
            controller,
            app_state,
            preferences,
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

    fn handle_gameplay_changes(&mut self, changes: GameplayControllerChanges) {
        if let Some(index) = changes.last_attempted_level_changed {
            self.preferences.last_started_level = Some(index + 1);
            if let Err(err) = self.preferences.save(config::PREFERENCES_PATH) {
                eprintln!("warning: failed to persist preferences: {err}");
            }
        }
    }

    fn apply_app_action(&mut self, action: AppAction) -> Result<()> {
        let applied = apply_action_in_context(self, action)?;
        self.handle_gameplay_changes(applied.changes);
        if let Some(plan) = applied.presentation_plan {
            render_presentation_plan(self, &plan)?;
        }
        Ok(())
    }

    fn apply_app_input(&mut self, input: AppInput) -> Result<()> {
        let action = interpret_input(&self.app_state, input);
        self.apply_app_action(action)
    }

    fn on_gameplay_tap(&mut self, screen_x: f64, screen_y: f64) -> Result<()> {
        let surface = build_gameplay_surface_model(&self.app_state, &self.controller);
        let policy = build_gameplay_policy_context(&self.app_state, &self.controller);
        let input = gameplay_pointer_tap(
            &mut self.app_state.gameplay,
            &surface,
            policy,
            screen_x,
            screen_y,
        );

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
    type Error = std::io::Error;

    fn controller_and_app_state_mut(&mut self) -> (&mut GameplayController, &mut AppState) {
        (&mut self.controller, &mut self.app_state)
    }
}
