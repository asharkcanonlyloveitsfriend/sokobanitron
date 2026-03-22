use crate::{config, platform};
use renderer::{
    BoardViewport, ControlsButtonAction, controls_button_action_at,
    fit_board_viewport_for_controls, level_select_menu_nav_action_at,
    level_select_menu_start_for_nav, level_select_menu_target_at,
    overlay_primary_action_button_contains, top_left_level_button_rect,
};
use sokobanitron_app::{
    AppAction, AppDriverContext, AppInput, AppState, BoxPathStyle, BoxRemovedStyle, PresentMode,
    PresentationProfile, apply_action_and_present_in_context, interpret_input, is_editor_menu_open,
    is_gameplay_menu_open, is_gameplay_screen, is_level_select_open, is_overlay_open,
    level_select_page_start,
};
use sokobanitron_gameplay::{
    BoardView, GameplayController, GameplayControllerChanges, GameplayPreferences,
    OrientationPolicy, load_levels_from_default_locations,
};
use std::io::Result;

const DEFAULT_FALLBACK_LEVEL_ASCII: &str = "\
_@_#\n\
_#_#\n\
___#\n\
#_.#\n\
#_.#\n\
#_.#\n\
__##\n\
_$__\n\
_$$_\n\
____";
const KINDLE_PRESENTATION_PROFILE: PresentationProfile = PresentationProfile {
    box_removed_style: BoxRemovedStyle::VanishThenBlink,
    box_path_style: BoxPathStyle::FlashThenHide,
    delayed_solved_present_mode: PresentMode::FastPartial,
    allow_delays: true,
};

fn default_fallback_level_ascii() -> String {
    DEFAULT_FALLBACK_LEVEL_ASCII
        .chars()
        .map(|ch| if ch == '_' { ' ' } else { ch })
        .collect()
}

pub struct KindleApp {
    pub(crate) renderer: renderer::Renderer,
    levels: Vec<String>,
    pub(crate) preview_boards: Vec<BoardView>,
    pub(crate) controller: GameplayController,
    pub(crate) app_state: AppState,
    preferences: GameplayPreferences,
    pub(crate) viewport: BoardViewport,
    pub(crate) display: platform::Display,
}

impl KindleApp {
    fn build_preview_board(level_ascii: &str) -> BoardView {
        GameplayController::new(vec![level_ascii.to_string()], None)
            .board()
            .clone()
    }

    pub fn new() -> Result<Self> {
        let fallback_level = default_fallback_level_ascii();
        let levels = load_levels_from_default_locations(
            OrientationPolicy::RotateWideToPortrait,
            &fallback_level,
        );
        let preview_boards = levels
            .iter()
            .map(|level| Self::build_preview_board(level))
            .collect::<Vec<_>>();
        let preferences = GameplayPreferences::load(config::PREFERENCES_PATH);
        let last_attempted_level = preferences.level_index(levels.len());
        let controller = GameplayController::new(levels.clone(), last_attempted_level);
        let viewport = Self::compute_viewport(controller.board());
        Ok(Self {
            renderer: Self::build_renderer(),
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

    fn build_effective_presentation_profile(&self) -> PresentationProfile {
        PresentationProfile {
            box_removed_style: KINDLE_PRESENTATION_PROFILE.box_removed_style,
            box_path_style: if self.preferences.show_box_path {
                KINDLE_PRESENTATION_PROFILE.box_path_style
            } else {
                BoxPathStyle::Hidden
            },
            delayed_solved_present_mode: KINDLE_PRESENTATION_PROFILE.delayed_solved_present_mode,
            allow_delays: KINDLE_PRESENTATION_PROFILE.allow_delays,
        }
    }

    fn apply_app_action(&mut self, action: AppAction) -> Result<()> {
        let profile = self.build_effective_presentation_profile();
        let applied = apply_action_and_present_in_context(self, action, &profile)?;
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

    fn on_tap(&mut self, raw_x: i32, raw_y: i32) -> Result<()> {
        let (screen_x, screen_y) = platform::map_touch_to_screen(raw_x, raw_y)?;
        if is_gameplay_menu_open(&self.app_state)
            && overlay_primary_action_button_contains(
                screen_x as f64,
                screen_y as f64,
                config::WIDTH as u32,
                config::HEIGHT as u32,
            )
        {
            self.apply_app_input(AppInput::EnterEditorMode)?;
            self.render()?;
            return Ok(());
        }
        if is_editor_menu_open(&self.app_state)
            && overlay_primary_action_button_contains(
                screen_x as f64,
                screen_y as f64,
                config::WIDTH as u32,
                config::HEIGHT as u32,
            )
        {
            self.apply_app_input(AppInput::EnterGameplayMode)?;
            self.render()?;
            return Ok(());
        }

        if !is_overlay_open(&self.app_state)
            && top_left_level_button_rect().contains(screen_x as f64, screen_y as f64)
        {
            self.apply_app_input(AppInput::OpenLevelSelect)?;
            self.render()?;
            return Ok(());
        }
        if let Some(action) = controls_button_action_at(
            screen_x as f64,
            screen_y as f64,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            self.controller.can_undo(),
            self.controller.can_restart(),
        ) {
            match action {
                ControlsButtonAction::Restart => {
                    if !is_overlay_open(&self.app_state) && is_gameplay_screen(&self.app_state) {
                        self.apply_app_input(AppInput::ControlRestart)?;
                        self.render()?;
                        return Ok(());
                    }
                }
                ControlsButtonAction::Undo => {
                    if !is_overlay_open(&self.app_state) && is_gameplay_screen(&self.app_state) {
                        self.apply_app_input(AppInput::ControlUndo)?;
                        self.render()?;
                        return Ok(());
                    }
                }
                ControlsButtonAction::ShowMenu => {
                    self.apply_app_input(AppInput::OverlayToggle)?;
                    self.render()?;
                    return Ok(());
                }
            }
        }
        if is_level_select_open(&self.app_state) {
            let page_start_idx = level_select_page_start(&self.app_state).unwrap_or(0);
            if let Some(nav_action) = level_select_menu_nav_action_at(
                screen_x as f64,
                screen_y as f64,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                self.levels.len(),
                self.controller.current_level(),
                page_start_idx,
            ) {
                let page_start = level_select_menu_start_for_nav(
                    self.levels.len(),
                    self.controller.current_level(),
                    page_start_idx,
                    nav_action,
                );
                self.apply_app_input(AppInput::LevelSelectNavigate { page_start })?;
                self.render()?;
                return Ok(());
            }

            if let Some(selected_level) = level_select_menu_target_at(
                screen_x as f64,
                screen_y as f64,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                self.levels.len(),
                page_start_idx,
            ) {
                self.apply_app_input(AppInput::LevelSelectSelect(selected_level))?;
                self.render()?;
            }
            return Ok(());
        }
        if self.controller.board().is_won() {
            if is_gameplay_screen(&self.app_state) {
                self.apply_app_input(AppInput::SolvedAdvance)?;
                self.render()?;
                return Ok(());
            }
        }

        if is_gameplay_screen(&self.app_state)
            && let Some((x, y)) = self.viewport.screen_to_cell(
                screen_x as f64,
                screen_y as f64,
                self.controller.board(),
            )
        {
            self.apply_app_input(AppInput::BoardTap { x, y })?;
        }
        Ok(())
    }
}

impl AppDriverContext for KindleApp {
    fn controller_and_app_state_mut(&mut self) -> (&mut GameplayController, &mut AppState) {
        (&mut self.controller, &mut self.app_state)
    }
}
