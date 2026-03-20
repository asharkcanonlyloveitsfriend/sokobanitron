use crate::{config, platform};
use renderer::{
    BoardViewport, ControlsButtonAction, ControlsUiMode, Renderer, RendererOverrides,
    controls_button_action_at, draw_controls_ui, fit_board_viewport_for_controls,
    level_select_menu_nav_action_at, level_select_menu_start_for_nav,
    level_select_menu_start_index, level_select_menu_target_at,
};
use sokobanitron_gameplay::{
    BoardView, BoxMovedTrailPresentation, BoxRemovedPresentation, GameplayController,
    GameplayControllerChanges, GameplayPreferences, GameplayPresentMode,
    GameplayTapPresentationPlan, GameplayTapPresentationStep, GameplayTapPresentationStyle,
    OrientationPolicy, build_tap_presentation_plan, load_levels_from_default_locations,
};
use std::io::Result;
use std::thread;
use std::time::Duration;

const PORTRAIT_LEVEL_VISUAL: &str = "\
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
const EINK_TAP_STYLE: GameplayTapPresentationStyle = GameplayTapPresentationStyle {
    box_removed_presentation: BoxRemovedPresentation::VanishThenBlink,
    long_box_path_presentation: BoxMovedTrailPresentation::FlashThenHide,
    delayed_win_present_mode: GameplayPresentMode::FastPartial,
};

fn default_fallback_level_ascii() -> String {
    PORTRAIT_LEVEL_VISUAL
        .chars()
        .map(|ch| if ch == '_' { ' ' } else { ch })
        .collect()
}

pub struct KindleApp {
    renderer: Renderer,
    levels: Vec<String>,
    preview_boards: Vec<BoardView>,
    controller: GameplayController,
    preferences: GameplayPreferences,
    viewport: BoardViewport,
    menu_open: bool,
    menu_page_start: usize,
    display: platform::Display,
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
            renderer: Renderer::with_overrides(RendererOverrides {
                selected_box_primary: Some(config::KINDLE_SELECTED_BOX_PRIMARY),
                selected_box_highlight: Some(config::KINDLE_SELECTED_BOX_HIGHLIGHT),
                selected_box_shadow: Some(config::KINDLE_SELECTED_BOX_SHADOW),
                ..RendererOverrides::default()
            }),
            levels,
            preview_boards,
            controller,
            preferences,
            viewport,
            menu_open: false,
            menu_page_start: 0,
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

    fn advance_after_win(&mut self, target_level: usize) -> Result<()> {
        let changes = self.controller.advance_after_win(target_level);
        self.apply_controller_changes(changes);
        self.render()
    }

    fn animate_player_blink(&mut self) -> Result<()> {
        let mut blink_frame = vec![0u8; config::WIDTH * config::HEIGHT * 4];
        self.renderer.draw_with_box_trail_progress_effects(
            &mut blink_frame,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            self.controller.board(),
            &self.viewport,
            None,
            None,
            true,
            true,
            false,
        );
        draw_controls_ui(
            &mut blink_frame,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            ControlsUiMode::Gameplay,
        );
        self.display.present_rgba_fast_partial(&blink_frame)?;
        thread::sleep(Duration::from_millis(config::BLINK_ON_MS));

        self.render_with_options(None, true, true, true)
    }

    fn draw_rounded_rect_rgba(
        frame: &mut [u8],
        x: i32,
        y: i32,
        w: u32,
        h: u32,
        radius: u32,
        color: [u8; 4],
    ) {
        let start_x = x.max(0) as usize;
        let start_y = y.max(0) as usize;
        let end_x = (x + w as i32).clamp(0, config::WIDTH as i32) as usize;
        let end_y = (y + h as i32).clamp(0, config::HEIGHT as i32) as usize;
        if start_x >= end_x || start_y >= end_y {
            return;
        }

        let radius = radius.min(w / 2).min(h / 2) as i32;
        let w_i = w as i32;
        let h_i = h as i32;
        let r2 = radius * radius;

        for py in start_y..end_y {
            let row = py * config::WIDTH * 4;
            for px in start_x..end_x {
                let local_x = px as i32 - x;
                let local_y = py as i32 - y;

                if radius > 0 {
                    let in_left = local_x < radius;
                    let in_right = local_x >= w_i - radius;
                    let in_top = local_y < radius;
                    let in_bottom = local_y >= h_i - radius;

                    if (in_left || in_right) && (in_top || in_bottom) {
                        let cx = if in_left { radius - 1 } else { w_i - radius };
                        let cy = if in_top { radius - 1 } else { h_i - radius };
                        let dx = local_x - cx;
                        let dy = local_y - cy;
                        if dx * dx + dy * dy > r2 {
                            continue;
                        }
                    }
                }

                let idx = row + px * 4;
                frame[idx] = color[0];
                frame[idx + 1] = color[1];
                frame[idx + 2] = color[2];
                frame[idx + 3] = color[3];
            }
        }
    }

    fn animate_box_vanish(&mut self, to_x: u32, to_y: u32, show_win_overlay: bool) -> Result<()> {
        let (cell_x, cell_y, cell_w, cell_h) = self.viewport.cell_to_screen_rect(to_x, to_y);
        let inset = (cell_w / 24).max(1);
        let box_x = cell_x + inset as i32;
        let box_y = cell_y + inset as i32;
        let box_w = cell_w.saturating_sub(inset * 2);
        let box_h = cell_h.saturating_sub(inset * 2);
        let raw_base_size = box_w.min(box_h);
        let base_size =
            ((raw_base_size as usize * config::BOX_VANISH_START_SCALE_PERCENT) / 100).max(1) as u32;
        if base_size == 0 {
            return self.render_with_options(None, true, true, show_win_overlay);
        }
        let base_x = box_x + ((raw_base_size - base_size) / 2) as i32;
        let base_y = box_y + ((raw_base_size - base_size) / 2) as i32;

        let steps = config::BOX_VANISH_STEPS.max(1);
        for step in 0..steps {
            let mut frame = vec![0u8; config::WIDTH * config::HEIGHT * 4];
            self.renderer.draw_with_box_trail_options(
                &mut frame,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                self.controller.board(),
                &self.viewport,
                None,
                true,
                show_win_overlay,
            );
            draw_controls_ui(
                &mut frame,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                ControlsUiMode::Gameplay,
            );

            let remaining = steps - step;
            let size = if step + 1 == steps {
                ((base_size as usize * config::BOX_VANISH_TAIL_SCALE_PERCENT) / 100).max(1) as u32
            } else {
                ((base_size as usize * remaining) / steps).max(1) as u32
            };
            let draw_x = base_x + ((base_size - size) / 2) as i32;
            let draw_y = base_y + ((base_size - size) / 2) as i32;
            let radius = (size * 14) / 100;
            Self::draw_rounded_rect_rgba(
                &mut frame,
                draw_x,
                draw_y,
                size,
                size,
                radius,
                [0, 0, 0, 255],
            );

            self.display.present_rgba_fast_partial(&frame)?;
            thread::sleep(Duration::from_millis(config::BOX_VANISH_STEP_MS));
        }

        self.render_with_options(None, true, true, show_win_overlay)
    }

    fn apply_controller_changes(&mut self, changes: GameplayControllerChanges) {
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

    fn execute_tap_presentation_plan(&mut self, plan: GameplayTapPresentationPlan) -> Result<()> {
        for step in plan.steps {
            match step {
                GameplayTapPresentationStep::Render {
                    box_trail,
                    draw_player,
                    show_win_overlay,
                    present_mode,
                } => {
                    self.render_with_options(
                        box_trail.as_deref(),
                        draw_player,
                        matches!(present_mode, GameplayPresentMode::FastPartial),
                        show_win_overlay,
                    )?;
                }
                GameplayTapPresentationStep::AnimatePlayerBlink => {
                    self.animate_player_blink()?;
                }
                GameplayTapPresentationStep::AnimateBoxVanish {
                    to_x,
                    to_y,
                    show_win_overlay,
                } => {
                    self.animate_box_vanish(to_x, to_y, show_win_overlay)?;
                }
                GameplayTapPresentationStep::AnimateBoxPathDisappear { .. } => {}
            }
        }
        Ok(())
    }

    fn compute_viewport(board: &sokobanitron_gameplay::BoardView) -> BoardViewport {
        fit_board_viewport_for_controls(config::WIDTH as u32, config::HEIGHT as u32, board)
    }

    fn render(&mut self) -> Result<()> {
        self.render_with_options(None, true, false, true)
    }

    fn render_with_options(
        &mut self,
        box_trail: Option<&[(u32, u32)]>,
        draw_player: bool,
        fast_partial: bool,
        show_win_overlay: bool,
    ) -> Result<()> {
        let mut rgba = vec![0u8; config::WIDTH * config::HEIGHT * 4];
        if self.menu_open {
            self.renderer.draw_background_only(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
            );
            self.renderer.draw_level_select_menu_contents(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                &self.preview_boards,
                self.controller.current_level(),
                self.menu_page_start,
            );
            draw_controls_ui(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                ControlsUiMode::MenuOpen,
            );
        } else {
            self.renderer.draw_with_box_trail_options(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                self.controller.board(),
                &self.viewport,
                box_trail,
                draw_player,
                show_win_overlay,
            );
            draw_controls_ui(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                ControlsUiMode::Gameplay,
            );
        }
        if fast_partial {
            self.display.present_rgba_fast_partial(&rgba)
        } else {
            self.display.present_rgba(&rgba)
        }
    }

    fn on_tap(&mut self, raw_x: i32, raw_y: i32) -> Result<()> {
        let (screen_x, screen_y) = platform::map_touch_to_screen(raw_x, raw_y)?;
        if let Some(action) = controls_button_action_at(
            screen_x as f64,
            screen_y as f64,
            config::WIDTH as u32,
            config::HEIGHT as u32,
        ) {
            match action {
                ControlsButtonAction::Restart => {
                    if !self.menu_open {
                        let changes = self.controller.restart_with_changes();
                        self.apply_controller_changes(changes);
                        self.render()?;
                        return Ok(());
                    }
                }
                ControlsButtonAction::Undo => {
                    if !self.menu_open {
                        let changes = self.controller.undo_with_changes();
                        self.apply_controller_changes(changes);
                        self.render()?;
                        return Ok(());
                    }
                }
                ControlsButtonAction::ShowMenu => {
                    self.menu_open = !self.menu_open;
                    if self.menu_open {
                        self.menu_page_start = level_select_menu_start_index(
                            self.levels.len(),
                            self.controller.current_level(),
                        );
                    }
                    self.render()?;
                    return Ok(());
                }
            }
        }
        if self.menu_open {
            if let Some(nav_action) = level_select_menu_nav_action_at(
                screen_x as f64,
                screen_y as f64,
                config::WIDTH as u32,
                config::HEIGHT as u32,
            ) {
                self.menu_page_start = level_select_menu_start_for_nav(
                    self.levels.len(),
                    self.controller.current_level(),
                    self.menu_page_start,
                    nav_action,
                );
                self.render()?;
                return Ok(());
            }

            if let Some(target) = level_select_menu_target_at(
                screen_x as f64,
                screen_y as f64,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                self.levels.len(),
                self.menu_page_start,
            ) {
                let changes = self.controller.jump_to_level(target);
                self.apply_controller_changes(changes);
                self.menu_open = false;
                self.render()?;
            }
            return Ok(());
        }
        if self.controller.board().is_won() {
            if let Some(next) = self.controller.peek_level(1) {
                self.advance_after_win(next)?;
            }
            return Ok(());
        }

        if let Some((x, y)) =
            self.viewport
                .screen_to_cell(screen_x as f64, screen_y as f64, self.controller.board())
        {
            let tap_outcome = self.controller.click_cell_with_outcome(x, y);
            self.apply_controller_changes(tap_outcome.changes);
            let plan = build_tap_presentation_plan(
                &tap_outcome,
                self.preferences.show_box_path,
                EINK_TAP_STYLE,
            );
            self.execute_tap_presentation_plan(plan)?;
        }
        Ok(())
    }
}
