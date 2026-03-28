use crate::{config, platform};
use presentation::{GameplayPresentationState, Renderer};
use sokobanitron_app::{
    AppPreferences,
    app::{
        AppDriverContext, AppInput, AppInteractionMode, AppPreferencesStore, AppState,
        apply_input_and_render_in_context,
    },
    gameplay::{
        interpret_gameplay_pointer_event, resize_gameplay_surface, set_gameplay_touch_slop,
    },
    level_bootstrap::load_initial_levels_for_app,
    shared::PointerPhase,
};
use sokobanitron_gameplay::{BoardView, GameplayController};
use std::io::Result;
use std::path::Path;

const TOUCH_POINTER_ID: u64 = 1;
const KINDLE_GAMEPLAY_TAP_SLOP_PX: i32 = 24;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppSleepState {
    Awake,
    AppSleepScreenVisible,
    SystemScreensaverActive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SleepSyncOutcome {
    NoChange,
    WokeAndRestored,
}

pub struct KindleApp {
    pub(crate) renderer: Renderer,
    pub(crate) gameplay_presentation: GameplayPresentationState,
    pub(crate) rgba_frame: Vec<u8>,
    sleep_state: AppSleepState,
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
        let preferences = AppPreferences::load_and_save_normalized(config::PREFERENCES_PATH)
            .unwrap_or_else(|err| {
                eprintln!("warning: failed to load or normalize preferences: {err}");
                AppPreferences::default()
            });
        let last_attempted_level = preferences.level_index(levels.len());
        let controller = GameplayController::new(levels.clone(), last_attempted_level);
        let mut app_state = AppState::default();
        resize_gameplay_surface(
            &mut app_state.gameplay,
            config::WIDTH as u32,
            config::HEIGHT as u32,
        );
        set_gameplay_touch_slop(&mut app_state.gameplay, KINDLE_GAMEPLAY_TAP_SLOP_PX);
        Ok(Self {
            renderer: Self::build_renderer(),
            gameplay_presentation: GameplayPresentationState::new(),
            rgba_frame: vec![0; config::WIDTH * config::HEIGHT * 4],
            sleep_state: AppSleepState::Awake,
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
                    .kindle
                    .use_app_sleep_screen
                    .then_some(config::SLEEP_STATE_POLL_TIMEOUT_MS),
            )?;
            let sync = self.sync_sleep_state()?;

            match event {
                platform::AppInputEvent::IdleTick => {}
                platform::AppInputEvent::Pointer {
                    phase,
                    raw_x,
                    raw_y,
                } => self.on_pointer(phase, raw_x, raw_y)?,
                platform::AppInputEvent::PowerShortPress => self.handle_power_short_press(sync)?,
                platform::AppInputEvent::PowerLongPress => {
                    if let Err(err) = platform::start_lab126_gui() {
                        eprintln!("warning: failed to restart lab126_gui: {err}");
                    }
                    return Ok(());
                }
            }
        }
    }

    fn sync_sleep_state(&mut self) -> Result<SleepSyncOutcome> {
        if self.sleep_state == AppSleepState::Awake && !self.preferences.kindle.use_app_sleep_screen
        {
            return Ok(SleepSyncOutcome::NoChange);
        }

        match platform::read_powerd_state()? {
            platform::PowerdScreensaverState::Active => {
                if self.sleep_state == AppSleepState::Awake {
                    return Ok(SleepSyncOutcome::NoChange);
                }
                self.restore_active_screen()?;
                Ok(SleepSyncOutcome::WokeAndRestored)
            }
            platform::PowerdScreensaverState::ScreenSaver => {
                if self.sleep_state != AppSleepState::Awake {
                    return Ok(SleepSyncOutcome::NoChange);
                }
                if self.preferences.kindle.use_app_sleep_screen {
                    self.render_sleep_screen()?;
                    self.sleep_state = AppSleepState::AppSleepScreenVisible;
                } else {
                    self.sleep_state = AppSleepState::SystemScreensaverActive;
                }
                Ok(SleepSyncOutcome::NoChange)
            }
            platform::PowerdScreensaverState::Other => Ok(SleepSyncOutcome::NoChange),
        }
    }

    fn enter_sleep_screen(&mut self) -> Result<()> {
        if self.preferences.kindle.use_app_sleep_screen {
            self.render_sleep_screen()?;
            match platform::enter_powerd_screensaver() {
                Ok(()) => {
                    self.sleep_state = AppSleepState::AppSleepScreenVisible;
                }
                Err(err) => {
                    eprintln!("warning: failed to enter sleep: {err}");
                    self.restore_active_screen()?;
                }
            }
        } else {
            match platform::enter_system_screensaver() {
                Ok(()) => {
                    self.sleep_state = AppSleepState::SystemScreensaverActive;
                }
                Err(err) => {
                    eprintln!("warning: failed to enter sleep: {err}");
                    self.restore_active_screen()?;
                }
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
        self.sleep_state = AppSleepState::Awake;
        self.display.force_full_refresh_next();
        self.render()
    }

    fn handle_power_short_press(&mut self, sync: SleepSyncOutcome) -> Result<()> {
        // The same physical power press can both wake powerd and still surface here as a
        // short-press input event. Once wake has already been observed and the active screen
        // restored, ignore that trailing press so we do not immediately re-enter sleep.
        if sync == SleepSyncOutcome::WokeAndRestored {
            return Ok(());
        }

        match self.sleep_state {
            AppSleepState::Awake => self.enter_sleep_screen(),
            AppSleepState::AppSleepScreenVisible | AppSleepState::SystemScreensaverActive => {
                self.exit_sleep_screen()
            }
        }
    }

    fn apply_app_input(&mut self, input: AppInput) -> Result<()> {
        apply_input_and_render_in_context(self, input).map(|_| ())
    }

    fn handle_gameplay_input(&mut self, input: AppInput) -> Result<()> {
        match input {
            AppInput::NoOp => Ok(()),
            AppInput::BoardTap { .. } => self.apply_app_input(input),
            _ => {
                self.apply_app_input(input)?;
                self.render()
            }
        }
    }

    fn on_gameplay_pointer_event(
        &mut self,
        phase: PointerPhase,
        screen_x: f64,
        screen_y: f64,
    ) -> Result<()> {
        let input = interpret_gameplay_pointer_event(
            &mut self.app_state,
            &self.controller,
            TOUCH_POINTER_ID,
            phase,
            screen_x,
            screen_y,
        );
        self.handle_gameplay_input(input)
    }

    fn on_pointer(&mut self, phase: PointerPhase, raw_x: i32, raw_y: i32) -> Result<()> {
        let (screen_x, screen_y) = platform::map_touch_to_screen(raw_x, raw_y)?;
        match self.app_state.interaction_mode() {
            AppInteractionMode::Gameplay => {
                self.on_gameplay_pointer_event(phase, screen_x as f64, screen_y as f64)
            }
            AppInteractionMode::Overlay(overlay)
                if matches!(
                    overlay.owning_screen(),
                    sokobanitron_app::app::AppScreen::Gameplay
                ) =>
            {
                self.on_gameplay_pointer_event(phase, screen_x as f64, screen_y as f64)
            }
            AppInteractionMode::Overlay(_) | AppInteractionMode::Editor => Ok(()),
        }
    }
}

impl AppDriverContext for KindleApp {
    type Error = std::io::Error;

    fn controller_and_app_state_mut(&mut self) -> (&mut GameplayController, &mut AppState) {
        (&mut self.controller, &mut self.app_state)
    }
}

impl AppPreferencesStore for KindleApp {
    fn app_preferences(&mut self) -> &mut AppPreferences {
        &mut self.preferences
    }

    fn app_preferences_path(&self) -> &Path {
        Path::new(config::PREFERENCES_PATH)
    }
}
