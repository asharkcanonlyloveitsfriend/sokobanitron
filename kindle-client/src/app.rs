use crate::{config, level, platform, ui};
use renderer::{BoardViewport, BoardViewportOptions, Renderer, RendererOverrides};
use sokobanitron_gameplay::{ClickOutcome, GameplayKey, GameplaySession};
use std::fs;
use std::io::Result;
use std::thread;
use std::time::Duration;

pub struct KindleApp {
    renderer: Renderer,
    levels: Vec<String>,
    current_level: usize,
    last_attempted_level: Option<usize>,
    session: GameplaySession,
    viewport: BoardViewport,
    display: platform::Display,
}

impl KindleApp {
    pub fn new() -> Result<Self> {
        let levels = level::load_kindle_levels();
        let last_attempted_level = Self::load_last_attempted_level(levels.len());
        let current_level = last_attempted_level.unwrap_or(0);
        let session = GameplaySession::from_level_ascii(levels[current_level].clone());
        let viewport = Self::compute_viewport(session.board());
        Ok(Self {
            renderer: Renderer::with_overrides(RendererOverrides {
                selected_box_primary: Some(config::KINDLE_SELECTED_BOX_PRIMARY),
                selected_box_highlight: Some(config::KINDLE_SELECTED_BOX_HIGHLIGHT),
                selected_box_shadow: Some(config::KINDLE_SELECTED_BOX_SHADOW),
                ..RendererOverrides::default()
            }),
            levels,
            current_level,
            last_attempted_level,
            session,
            viewport,
            display: platform::Display::new()?,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.render()?;

        let mut touch = platform::TouchReader::new()?;
        loop {
            let (raw_x, raw_y) = touch.next_tap_raw()?;
            self.on_tap(raw_x, raw_y)?;
        }
    }

    fn update_viewport(&mut self) {
        self.viewport = Self::compute_viewport(self.session.board());
    }

    fn load_current_level(&mut self) {
        self.session = GameplaySession::from_level_ascii(self.levels[self.current_level].clone());
        self.update_viewport();
    }

    fn jump_to_level(&mut self, index: usize) {
        if self.levels.is_empty() {
            return;
        }
        let clamped = index.min(self.levels.len().saturating_sub(1));
        self.current_level = clamped;
        self.load_current_level();
    }

    fn peek_level(&self, delta: i32) -> Option<usize> {
        if self.levels.is_empty() {
            return None;
        }
        let len = self.levels.len() as i32;
        let next = (self.current_level as i32 + delta).rem_euclid(len);
        Some(next as usize)
    }

    fn navigate_with_flash(&mut self, target_level: usize) -> Result<()> {
        self.flash_level_number(target_level)?;
        self.jump_to_level(target_level);
        self.render()
    }

    fn flash_level_number(&mut self, level_index: usize) -> Result<()> {
        let mut rgba = vec![0u8; config::WIDTH * config::HEIGHT * 4];
        self.renderer.draw_with_box_trail_options(
            &mut rgba,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            self.session.board(),
            &self.viewport,
            None,
            true,
            false,
        );
        ui::draw_controls_ui(&mut rgba, self.show_play_button());
        ui::draw_level_flash_overlay(&mut rgba, level_index + 1);
        self.display.present_rgba(&rgba)
    }

    fn animate_player_blink(&mut self) -> Result<()> {
        let mut blink_frame = vec![0u8; config::WIDTH * config::HEIGHT * 4];
        self.renderer.draw_with_box_trail_options(
            &mut blink_frame,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            self.session.board(),
            &self.viewport,
            None,
            true,
            false,
        );
        self.renderer.draw_player_blink_overlay(
            &mut blink_frame,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            self.session.board(),
            &self.viewport,
        );
        ui::draw_controls_ui(&mut blink_frame, self.show_play_button());
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
                self.session.board(),
                &self.viewport,
                None,
                true,
                show_win_overlay,
            );
            ui::draw_controls_ui(&mut frame, self.show_play_button());

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

    fn show_play_button(&self) -> bool {
        matches!(
            self.last_attempted_level,
            Some(last) if last != self.current_level
        )
    }

    fn load_last_attempted_level(level_count: usize) -> Option<usize> {
        let raw = fs::read_to_string(config::LAST_ATTEMPTED_LEVEL_PATH).ok()?;
        let value = raw.trim().parse::<usize>().ok()?;
        if value == 0 {
            return None;
        }
        let idx = value - 1;
        if idx < level_count { Some(idx) } else { None }
    }

    fn persist_last_attempted_level(&mut self, level_index: usize) {
        self.last_attempted_level = Some(level_index);
        let one_based = level_index + 1;
        if let Err(err) = fs::write(config::LAST_ATTEMPTED_LEVEL_PATH, format!("{one_based}\n")) {
            eprintln!("warning: failed to persist last attempted level: {err}");
        }
    }

    fn record_first_move_if_needed(&mut self, was_started: bool) {
        if !was_started && self.session.is_started() {
            self.persist_last_attempted_level(self.current_level);
        }
    }

    fn compute_viewport(board: &sokobanitron_gameplay::BoardView) -> BoardViewport {
        let h_margin = config::BOARD_HORIZONTAL_MARGIN as u32;
        let v_margin = config::BOARD_VERTICAL_MARGIN as u32;
        let safe_width = (config::WIDTH as u32).saturating_sub(h_margin * 2).max(1);
        let safe_height = (config::HEIGHT as u32).saturating_sub(v_margin * 2).max(1);

        let mut viewport = BoardViewport::fit_to_window_with_options(
            safe_width,
            safe_height,
            board,
            BoardViewportOptions::fill_available_space(),
        );
        viewport.origin_x += h_margin as i32;
        viewport.origin_y += v_margin as i32;
        viewport
    }

    fn render(&mut self) -> Result<()> {
        let box_trail = self.session.take_pending_box_trail();
        self.render_with_options(box_trail.as_deref(), true, false, true)
    }

    fn render_with_options(
        &mut self,
        box_trail: Option<&[(u32, u32)]>,
        draw_player: bool,
        fast_partial: bool,
        show_win_overlay: bool,
    ) -> Result<()> {
        let mut rgba = vec![0u8; config::WIDTH * config::HEIGHT * 4];
        self.renderer.draw_with_box_trail_options(
            &mut rgba,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            self.session.board(),
            &self.viewport,
            box_trail,
            draw_player,
            show_win_overlay,
        );
        ui::draw_controls_ui(&mut rgba, self.show_play_button());
        if fast_partial {
            self.display.present_rgba_fast_partial(&rgba)
        } else {
            self.display.present_rgba(&rgba)
        }
    }

    fn on_tap(&mut self, raw_x: i32, raw_y: i32) -> Result<()> {
        let (screen_x, screen_y) = platform::map_touch_to_screen(raw_x, raw_y)?;
        if let Some(action) = ui::button_action_at(screen_x, screen_y, self.show_play_button()) {
            match action {
                ui::ButtonAction::Restart => {
                    self.session.restart();
                    self.update_viewport();
                    self.render()?;
                    return Ok(());
                }
                ui::ButtonAction::Undo => {
                    self.session.on_key(GameplayKey::Backspace);
                    self.update_viewport();
                    self.render()?;
                    return Ok(());
                }
                ui::ButtonAction::Previous => {
                    if let Some(next) = self.peek_level(-1) {
                        self.navigate_with_flash(next)?;
                    }
                    return Ok(());
                }
                ui::ButtonAction::Next => {
                    if let Some(next) = self.peek_level(1) {
                        self.navigate_with_flash(next)?;
                    }
                    return Ok(());
                }
                ui::ButtonAction::JumpStart => {
                    self.navigate_with_flash(0)?;
                    return Ok(());
                }
                ui::ButtonAction::JumpEnd => {
                    let last = self.levels.len().saturating_sub(1);
                    self.navigate_with_flash(last)?;
                    return Ok(());
                }
                ui::ButtonAction::Play => {
                    if let Some(level_index) = self.last_attempted_level {
                        self.navigate_with_flash(level_index)?;
                    }
                    return Ok(());
                }
            }
        }
        if self.session.board().is_won() {
            if let Some(next) = self.peek_level(1) {
                self.navigate_with_flash(next)?;
            }
            return Ok(());
        }

        if let Some((x, y)) =
            self.viewport
                .screen_to_cell(screen_x as f64, screen_y as f64, self.session.board())
        {
            if self.session.board().player() == Some((x, y)) {
                self.animate_player_blink()?;
                return Ok(());
            }

            let was_won = self.session.board().is_won();
            let was_started = self.session.is_started();
            let click_outcome = self.session.click_cell_with_feedback(x, y);
            if click_outcome == ClickOutcome::IllegalBoxDestination {
                self.animate_player_blink()?;
                return Ok(());
            }
            if let ClickOutcome::BoxRemoved { to_x, to_y } = click_outcome {
                self.record_first_move_if_needed(was_started);
                let now_won = self.session.board().is_won();
                let delay_win_overlay = !was_won && now_won;
                let dirty_win = delay_win_overlay && !self.session.is_clean_solution();
                self.animate_box_vanish(to_x, to_y, !delay_win_overlay)?;
                if dirty_win {
                    self.animate_player_blink()?;
                } else if delay_win_overlay {
                    self.render_with_options(None, true, true, true)?;
                }
                return Ok(());
            }
            if click_outcome == ClickOutcome::NoOp {
                return Ok(());
            }
            self.record_first_move_if_needed(was_started);
            let box_trail = self.session.take_pending_box_trail();
            let now_won = self.session.board().is_won();
            let delay_win_overlay = !was_won && now_won;
            let dirty_win = delay_win_overlay && !self.session.is_clean_solution();
            if box_trail.as_ref().is_some_and(|path| path.len() > 2) {
                self.render_with_options(box_trail.as_deref(), false, false, !delay_win_overlay)?;
                if dirty_win {
                    self.animate_player_blink()?;
                } else {
                    self.render_with_options(None, true, true, true)?;
                }
            } else {
                self.render_with_options(None, true, false, !delay_win_overlay)?;
                if dirty_win {
                    self.animate_player_blink()?;
                } else if delay_win_overlay {
                    self.render_with_options(None, true, true, true)?;
                }
            }
        }
        Ok(())
    }
}
