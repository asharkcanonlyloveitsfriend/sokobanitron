use crate::{config, platform};
use presentation::{GameplayPresentationState, Renderer};
use sokobanitron_app::{
    AppPreferences,
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
use sokobanitron_gameplay::{BoardView, GameplayController, GameplayControllerChanges};
use std::io::Result;

pub struct KindleApp {
    pub(crate) renderer: Renderer,
    pub(crate) gameplay_presentation: GameplayPresentationState,
    pub(crate) rgba_frame: Vec<u8>,
    pub(crate) sleep_screen_active: bool,
    pub(crate) preview_boards: Vec<BoardView>,
    pub(crate) controller: GameplayController,
    pub(crate) app_state: AppState,
    preferences: AppPreferences,
    pub(crate) display: platform::Display,
}

impl KindleApp {
    pub fn new() -> Result<Self> {
        let initial_levels = load_initial_levels_for_app();
        let levels = initial_levels.levels;
        let preview_boards = initial_levels.preview_boards;
        let preferences =
            AppPreferences::load_and_sync(config::PREFERENCES_PATH).unwrap_or_else(|err| {
                eprintln!("warning: failed to sync preferences: {err}");
                AppPreferences::load(config::PREFERENCES_PATH)
            });
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
            sleep_screen_active: false,
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
            let event = touch.next_input_event(
                self.preferences
                    .use_app_sleep_screen
                    .then_some(config::SLEEP_STATE_POLL_TIMEOUT_MS),
            )?;
            let woke_from_sleep = self.sync_sleep_state()?;

            match event {
                platform::AppInputEvent::IdleTick => {}
                platform::AppInputEvent::Tap(raw_x, raw_y) => self.on_tap(raw_x, raw_y)?,
                platform::AppInputEvent::PowerShortPress => {
                    if woke_from_sleep {
                        continue;
                    }

                    if self.sleep_screen_active {
                        self.exit_sleep_screen()?;
                    } else {
                        self.enter_sleep_screen()?;
                    }
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

    fn sync_sleep_state(&mut self) -> Result<bool> {
        if !self.sleep_screen_active && !self.preferences.use_app_sleep_screen {
            return Ok(false);
        }

        match platform::read_powerd_state()? {
            platform::PowerdScreensaverState::Active => {
                if !self.sleep_screen_active {
                    return Ok(false);
                }
                self.restore_active_screen()?;
                Ok(true)
            }
            platform::PowerdScreensaverState::ScreenSaver => {
                if self.sleep_screen_active || !self.preferences.use_app_sleep_screen {
                    return Ok(false);
                }
                self.render_sleep_screen()?;
                self.sleep_screen_active = true;
                Ok(false)
            }
            platform::PowerdScreensaverState::Other => Ok(false),
        }
    }

    fn enter_sleep_screen(&mut self) -> Result<()> {
        let enter_sleep = if self.preferences.use_app_sleep_screen {
            self.render_sleep_screen()?;
            platform::enter_powerd_screensaver
        } else {
            platform::enter_system_screensaver
        };
        match enter_sleep() {
            Ok(()) => {
                self.sleep_screen_active = true;
            }
            Err(err) => {
                eprintln!("warning: failed to enter sleep: {err}");
                self.restore_active_screen()?;
            }
        }
        Ok(())
    }

    fn exit_sleep_screen(&mut self) -> Result<()> {
        if let Err(err) = platform::exit_powerd_screensaver() {
            eprintln!("warning: failed to exit powerd screensaver: {err}");
        }
        self.restore_active_screen()
    }

    fn restore_active_screen(&mut self) -> Result<()> {
        self.sleep_screen_active = false;
        self.display.force_full_refresh_next();
        self.render()
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
