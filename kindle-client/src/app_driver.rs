use crate::{config, platform};
use presentation::{GameplayAnimationPolicy, GameplayPresentationState, Renderer};
use sokobanitron_app::{
    AppPreferences,
    app::{
        AppDriverContext, AppPointerInput, AppRuntimeMut, AppState, PresentMode,
        handle_pointer_input_and_render_in_context,
    },
    gameplay::{
        resize_gameplay_surface, set_gameplay_level_sets, set_gameplay_max_cell_size,
        set_gameplay_touch_slop,
    },
    level_bootstrap::load_initial_levels_for_app,
    persistence::LevelPersistence,
    shared::PointerPhase,
};
use sokobanitron_gameplay::{BoardView, GameplayController};
use std::io::Result;

const TOUCH_POINTER_ID: u64 = 1;
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
    pub(crate) renderer: Renderer,
    pub(crate) gameplay_presentation: GameplayPresentationState,
    pub(crate) gray_frame: Vec<u8>,
    sleep_state: AppSleepState,
    pub(crate) preview_boards: Vec<BoardView>,
    pub(crate) controller: GameplayController,
    pub(crate) app_state: AppState,
    preferences: AppPreferences,
    level_persistence: LevelPersistence,
    pub(crate) display: platform::Display,
}

impl KindleApp {
    pub fn new() -> Result<Self> {
        let initial_levels =
            load_initial_levels_for_app(std::path::Path::new(config::LEVEL_SETS_ROOT))?;
        let levels = initial_levels.levels;
        let preview_boards = initial_levels.preview_boards;
        let preferences = AppPreferences::load_and_save_normalized(config::PREFERENCES_PATH)
            .unwrap_or_else(|err| {
                eprintln!("warning: failed to load or normalize preferences: {err}");
                AppPreferences::default()
            });
        let controller = GameplayController::new_at_level(
            levels.clone(),
            initial_levels.initial_level_index,
            initial_levels.persisted_resume_level_index,
        );
        let mut app_state = AppState::default();
        resize_gameplay_surface(
            &mut app_state.gameplay,
            config::WIDTH as u32,
            config::HEIGHT as u32,
        );
        set_gameplay_max_cell_size(&mut app_state.gameplay, KINDLE_GAMEPLAY_MAX_CELL_SIZE);
        set_gameplay_touch_slop(&mut app_state.gameplay, KINDLE_GAMEPLAY_TAP_SLOP_PX);
        set_gameplay_level_sets(
            &mut app_state.gameplay,
            initial_levels.level_set_catalog.clone(),
            Some(initial_levels.active_level_set_index),
        );
        Ok(Self {
            renderer: Self::build_renderer(),
            gameplay_presentation: GameplayPresentationState::with_animation_policy(
                GameplayAnimationPolicy::Limited,
            ),
            gray_frame: vec![0; config::WIDTH * config::HEIGHT],
            sleep_state: AppSleepState::Awake,
            preview_boards,
            controller,
            app_state,
            preferences,
            level_persistence: initial_levels.persistence,
            display: platform::Display::new()?,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.render()?;

        let mut touch = platform::TouchReader::new()?;
        loop {
            let timeout_ms = if self.gameplay_presentation.has_pending_presentation() {
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
                    if self.gameplay_presentation.has_pending_presentation() {
                        self.render_active_gameplay_presentation()?;
                    }
                }
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

    fn render_active_gameplay_presentation(&mut self) -> Result<()> {
        let result = self
            .gameplay_presentation
            .advance_presentation_with_damage();
        let scene = self.gameplay_presentation.current_scene().cloned();
        let (renderer, gray, display) =
            (&mut self.renderer, &mut self.gray_frame, &mut self.display);
        self.gameplay_presentation.draw_damage(
            renderer,
            gray,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            &result.damage,
        );
        let Some(scene) = scene else {
            return Ok(());
        };
        crate::display::present_gameplay_damage(
            display,
            &scene,
            &result.damage,
            gray,
            PresentMode::FastPartial,
        )?;
        Ok(())
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

    fn on_pointer(&mut self, phase: PointerPhase, raw_x: i32, raw_y: i32) -> Result<()> {
        let (screen_x, screen_y) = platform::map_touch_to_screen(raw_x, raw_y)?;
        let _ = handle_pointer_input_and_render_in_context(
            self,
            AppPointerInput::Pointer {
                id: TOUCH_POINTER_ID,
                phase,
                x: screen_x as f64,
                y: screen_y as f64,
            },
        )?;
        Ok(())
    }
}

impl AppDriverContext for KindleApp {
    type Error = std::io::Error;

    fn app_runtime_mut(&mut self) -> AppRuntimeMut<'_> {
        AppRuntimeMut {
            controller: &mut self.controller,
            app_state: &mut self.app_state,
            level_persistence: &mut self.level_persistence,
            preview_boards: &mut self.preview_boards,
        }
    }
}
