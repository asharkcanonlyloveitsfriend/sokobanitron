use crate::app_driver::App;
use renderer::{
    ControlsUiMode, UiIcon, draw_controls_ui, draw_overlay_primary_action_button,
    draw_top_left_level_button, draw_top_menu_toggle,
};
use sokobanitron_app::{
    AppScreen, FrameRequest, FrameSink, active_screen, is_editor_menu_open, is_gameplay_menu_open,
    is_gameplay_screen, is_level_select_open, level_select_page_start,
};
use std::thread;
use std::time::Duration;

const ANIMATION_TICK_MS: u64 = 50;
const BOX_PATH_SPEED_SCALE: f32 = 1.3;
const BOX_PATH_SPEED_EXPONENT: f32 = 0.5;
const BLINK_ON_MS: u64 = 120;

impl App {
    pub(crate) fn render_current(&mut self) {
        match active_screen(&self.app_state) {
            AppScreen::Gameplay => self.render_with_options(None, true, true),
            AppScreen::Editor => self.render_editor_mode(),
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
            if is_level_select_open(&self.app_state) {
                let page_start = level_select_page_start(&self.app_state).unwrap_or(0);
                self.renderer
                    .draw_background_only(frame, self.surface_width, self.surface_height);
                self.renderer.draw_level_select_menu_contents(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    &self.preview_boards,
                    self.controller.current_level(),
                    page_start,
                );
                draw_controls_ui(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    ControlsUiMode::MenuOpen,
                    false,
                    false,
                );
            } else if is_gameplay_menu_open(&self.app_state) {
                self.renderer
                    .draw_background_only(frame, self.surface_width, self.surface_height);
                draw_top_menu_toggle(frame, self.surface_width, self.surface_height, true);
                draw_overlay_primary_action_button(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    UiIcon::Draw,
                    [220, 220, 220, 255],
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
                    self.controller.can_undo(),
                    self.controller.can_restart(),
                );
                draw_top_left_level_button(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    self.controller.current_level() + 1,
                );
            }
            pixels.render().expect("render");
        }
    }

    pub(crate) fn render_editor_mode(&mut self) {
        if let Some(pixels) = &mut self.pixels {
            let frame = pixels.frame_mut();
            if is_editor_menu_open(&self.app_state) {
                self.renderer
                    .draw_background_only(frame, self.surface_width, self.surface_height);
                draw_top_menu_toggle(frame, self.surface_width, self.surface_height, true);
                draw_overlay_primary_action_button(
                    frame,
                    self.surface_width,
                    self.surface_height,
                    UiIcon::Manipulate,
                    [220, 220, 220, 255],
                );
            } else {
                self.editor_session
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
                self.controller.can_undo(),
                self.controller.can_restart(),
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
                    self.controller.can_undo(),
                    self.controller.can_restart(),
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
        if !is_gameplay_screen(&self.app_state) {
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
        if is_gameplay_screen(&self.app_state) {
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
        if is_gameplay_screen(&self.app_state) {
            self.run_box_path_disappear_animation(path, show_solved_overlay);
        }
        Ok(())
    }
}
