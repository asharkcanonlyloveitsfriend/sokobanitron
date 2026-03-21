use crate::app_driver::{ActiveScreen, App, create_menu_return_button_rect};
use renderer::{ControlsUiMode, draw_controls_ui};
use sokobanitron_app::{AppMode, FrameRequest, FrameSink};
use sokobanitron_level_creator::{
    ModeIcon, draw_mode_icon_in_rect, draw_top_menu_toggle, mode_toggle_button_rect,
};
use std::thread;
use std::time::Duration;

const ANIMATION_TICK_MS: u64 = 50;
const BOX_PATH_SPEED_SCALE: f32 = 1.3;
const BOX_PATH_SPEED_EXPONENT: f32 = 0.5;
const BLINK_ON_MS: u64 = 120;

impl App {
    pub(crate) fn render_current(&mut self) {
        match self.active_screen {
            ActiveScreen::Gameplay => self.render_with_options(None, true, true),
            ActiveScreen::Create => self.render_create_mode(),
        }
    }

    pub(crate) fn render_with_options(
        &mut self,
        box_trail: Option<&[(u32, u32)]>,
        draw_player: bool,
        show_solved_overlay: bool,
    ) {
        if let Some(pixels) = &mut self.pixels {
            let frame = pixels.frame_mut();
            if let AppMode::Menu { page_start } = &self.app_state.ui.mode {
                self.renderer
                    .draw_background_only(frame, self.surface_width, self.surface_height);
                self.renderer.draw_level_select_menu_contents(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    &self.preview_boards,
                    self.controller.current_level(),
                    *page_start,
                );
                draw_controls_ui(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    ControlsUiMode::MenuOpen,
                );
                draw_mode_icon_in_rect(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    mode_toggle_button_rect(),
                    ModeIcon::Draw,
                );
            } else {
                self.renderer.draw_with_box_trail_options(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    self.controller.board(),
                    &self.board_viewport,
                    box_trail,
                    draw_player,
                    show_solved_overlay,
                );
                draw_controls_ui(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    ControlsUiMode::Gameplay,
                );
            }
            pixels.render().expect("render");
        }
    }

    pub(crate) fn render_create_mode(&mut self) {
        if let Some(pixels) = &mut self.pixels {
            let frame = pixels.frame_mut();
            if self.create_menu_open {
                self.renderer
                    .draw_background_only(frame, self.surface_width, self.surface_height);
                draw_top_menu_toggle(frame, self.surface_width, self.surface_height, true);
                draw_mode_icon_in_rect(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    create_menu_return_button_rect(self.surface_width),
                    ModeIcon::Manipulate,
                );
            } else {
                self.create_session
                    .render(frame, self.surface_width, self.surface_height);
                draw_top_menu_toggle(frame, self.surface_width, self.surface_height, false);
            }
            pixels.render().expect("render");
        }
    }

    fn run_player_blink_animation(&mut self) {
        if let Some(pixels) = &mut self.pixels {
            let frame = pixels.frame_mut();
            self.renderer.draw_with_box_trail_progress_effects(
                frame,
                self.surface_width,
                self.surface_height,
                self.controller.board(),
                &self.board_viewport,
                None,
                None,
                true,
                true,
                false,
            );
            draw_controls_ui(
                frame,
                self.surface_width,
                self.surface_height,
                ControlsUiMode::Gameplay,
            );
            pixels.render().expect("render");
        }
        thread::sleep(Duration::from_millis(BLINK_ON_MS));
        self.render_with_options(None, true, true);
    }

    fn run_box_path_disappear_animation(&mut self, path: &[(u32, u32)], show_solved_overlay: bool) {
        if path.len() <= 2 {
            self.render_with_options(None, true, show_solved_overlay);
            return;
        }

        let total_segments = (path.len() - 1) as f32;
        let speed_per_tick = BOX_PATH_SPEED_SCALE * total_segments.powf(BOX_PATH_SPEED_EXPONENT);
        let mut consumed = 0.0f32;
        while consumed < total_segments {
            if let Some(pixels) = &mut self.pixels {
                let frame = pixels.frame_mut();
                self.renderer.draw_with_box_trail_progress_options(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    self.controller.board(),
                    &self.board_viewport,
                    Some(path),
                    Some(consumed),
                    false,
                    show_solved_overlay,
                );
                draw_controls_ui(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    ControlsUiMode::Gameplay,
                );
                pixels.render().expect("render");
            }
            thread::sleep(Duration::from_millis(ANIMATION_TICK_MS));
            consumed += speed_per_tick;
        }

        self.render_with_options(None, true, show_solved_overlay);
    }
}

impl FrameSink for App {
    type Error = ();

    fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error> {
        if !matches!(self.active_screen, ActiveScreen::Gameplay) {
            return Ok(());
        }

        match request {
            FrameRequest::Gameplay {
                box_trail,
                draw_player,
                show_solved_overlay,
                ..
            } => {
                self.render_with_options(box_trail.as_deref(), *draw_player, *show_solved_overlay);
                Ok(())
            }
            FrameRequest::Menu { .. } => Ok(()),
        }
    }

    fn animate_player_blink(&mut self) -> Result<(), Self::Error> {
        if matches!(self.active_screen, ActiveScreen::Gameplay) {
            self.run_player_blink_animation();
        }
        Ok(())
    }

    fn animate_box_vanish(
        &mut self,
        _to_x: u32,
        _to_y: u32,
        _show_solved_overlay: bool,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn animate_box_path_disappear(
        &mut self,
        path: &[(u32, u32)],
        show_solved_overlay: bool,
    ) -> Result<(), Self::Error> {
        if matches!(self.active_screen, ActiveScreen::Gameplay) {
            self.run_box_path_disappear_animation(path, show_solved_overlay);
        }
        Ok(())
    }
}
