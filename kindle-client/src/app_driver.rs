use crate::{config, platform};
use sokobanitron_app::{
    AppPreferences,
    app::{AppPointerInput, AppState, SharedAppRuntime},
    gameplay::{set_gameplay_max_cell_size, set_gameplay_touch_slop},
    level_bootstrap::load_initial_levels_for_app,
    shared::PointerPhase,
};
use std::io::Result;

const KINDLE_GAMEPLAY_TAP_SLOP_PX: i32 = 24;
const KINDLE_GAMEPLAY_MAX_CELL_SIZE: u32 = 178;

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
    pub(crate) runtime: SharedAppRuntime,
    sleep_state: AppSleepState,
    preferences: AppPreferences,
    pub(crate) display: platform::Display,
}

impl KindleApp {
    pub fn new() -> Result<Self> {
        let initial_levels =
            load_initial_levels_for_app(std::path::Path::new(config::LEVEL_SETS_ROOT))?;
        let preferences = AppPreferences::load_and_save_normalized(config::PREFERENCES_PATH)
            .unwrap_or_else(|err| {
                eprintln!("warning: failed to load or normalize preferences: {err}");
                AppPreferences::default()
            });
        let app_state = AppState {
            supports_multi_touch: true,
            ..AppState::default()
        };
        let mut runtime = SharedAppRuntime::new(
            initial_levels,
            app_state,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            Self::build_frame_renderer(),
        );
        set_gameplay_max_cell_size(
            &mut runtime.app_state_mut().gameplay,
            KINDLE_GAMEPLAY_MAX_CELL_SIZE,
        );
        set_gameplay_touch_slop(
            &mut runtime.app_state_mut().gameplay,
            KINDLE_GAMEPLAY_TAP_SLOP_PX,
        );
        Ok(Self {
            runtime,
            sleep_state: AppSleepState::Awake,
            preferences,
            display: platform::Display::new()?,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.render()?;

        let mut touch = platform::TouchReader::new()?;
        loop {
            let timeout_ms = if self.runtime.has_pending_render_work() {
                Some(50)
            } else {
                self.preferences
                    .kindle
                    .use_app_sleep_screen
                    .then_some(config::SLEEP_STATE_POLL_TIMEOUT_MS)
            };
            let event = touch.next_input_event(timeout_ms)?;
            let sync = self.sync_sleep_state()?;

            match event {
                platform::AppInputEvent::IdleTick => {
                    if self.runtime.has_pending_render_work() {
                        let _ = self.continue_pending_render_work_and_render()?;
                    }
                }
                platform::AppInputEvent::Pointer {
                    id,
                    phase,
                    screen_x,
                    screen_y,
                } => self.on_pointer(id, phase, screen_x, screen_y)?,
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

    fn on_pointer(
        &mut self,
        id: u64,
        phase: PointerPhase,
        screen_x: usize,
        screen_y: usize,
    ) -> Result<()> {
        let _ = self.handle_pointer_input_and_render(AppPointerInput::Pointer {
            id,
            phase,
            x: screen_x as f64,
            y: screen_y as f64,
        })?;
        Ok(())
    }
}
