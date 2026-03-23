use crate::{app_driver::KindleApp, config};
use renderer::{
    ControlsUiMode, Renderer, RendererOverrides, UiIcon, draw_controls_ui,
    draw_overlay_primary_action_button, draw_top_left_level_button,
};
use sokobanitron_app::{
    AppScreen, FrameRequest, FrameSink, PresentMode, active_screen, is_editor_menu_open,
    is_gameplay_menu_open, is_gameplay_screen, is_level_select_open, level_select_page_start,
};
use std::io::Result;
use std::thread;
use std::time::Duration;

const KINDLE_BOX_PRIMARY: [u8; 4] = [60, 63, 66, 255];
const KINDLE_BOX_SHADOW: [u8; 4] = [30, 31, 33, 255];
const KINDLE_PLAYER_BODY: [u8; 4] = [117, 117, 117, 255];
const KINDLE_PLAYER_LIMB: [u8; 4] = [80, 80, 80, 255];

impl KindleApp {
    fn draw_gameplay_screen(
        &mut self,
        frame: &mut [u8],
        box_trail: Option<&[(u32, u32)]>,
        box_trail_consumed_segments: Option<f32>,
        draw_player: bool,
        draw_player_blink: bool,
        show_solved_overlay: bool,
    ) {
        self.renderer.draw_with_box_trail_progress_effects(
            frame,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            self.controller.board(),
            &self.viewport,
            box_trail,
            box_trail_consumed_segments,
            draw_player,
            draw_player_blink,
            show_solved_overlay,
        );
        draw_controls_ui(
            frame,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            ControlsUiMode::Gameplay,
            self.controller.can_undo(),
            self.controller.can_restart(),
        );
        draw_top_left_level_button(
            frame,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            self.controller.current_level() + 1,
        );
    }

    pub(crate) fn build_renderer() -> Renderer {
        Renderer::with_overrides(RendererOverrides {
            box_primary: Some(KINDLE_BOX_PRIMARY),
            box_shadow: Some(KINDLE_BOX_SHADOW),
            player_body: Some(KINDLE_PLAYER_BODY),
            player_limb: Some(KINDLE_PLAYER_LIMB),
            selected_box_primary: Some(config::KINDLE_SELECTED_BOX_PRIMARY),
            selected_box_highlight: Some(config::KINDLE_SELECTED_BOX_HIGHLIGHT),
            selected_box_shadow: Some(config::KINDLE_SELECTED_BOX_SHADOW),
            ..RendererOverrides::default()
        })
    }

    pub(crate) fn render(&mut self) -> Result<()> {
        self.render_with_options(None, true, false, true)
    }

    pub(crate) fn render_with_options(
        &mut self,
        box_trail: Option<&[(u32, u32)]>,
        draw_player: bool,
        fast_partial: bool,
        show_solved_overlay: bool,
    ) -> Result<()> {
        let mut rgba = vec![0u8; config::WIDTH * config::HEIGHT * 4];
        if is_level_select_open(&self.app_state) {
            let page_start = level_select_page_start(&self.app_state).unwrap_or(0);
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
                page_start,
            );
            draw_controls_ui(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                ControlsUiMode::MenuOpen,
                false,
                false,
            );
        } else if is_gameplay_menu_open(&self.app_state) {
            self.renderer.draw_background_only(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
            );
            draw_controls_ui(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                ControlsUiMode::MenuOpen,
                false,
                false,
            );
            draw_overlay_primary_action_button(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                UiIcon::Draw,
                [220, 220, 220, 255],
            );
        } else if is_editor_menu_open(&self.app_state) {
            self.renderer.draw_background_only(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
            );
            draw_controls_ui(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                ControlsUiMode::MenuOpen,
                false,
                false,
            );
            draw_overlay_primary_action_button(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                UiIcon::Manipulate,
                [220, 220, 220, 255],
            );
        } else if matches!(active_screen(&self.app_state), AppScreen::Editor) {
            self.renderer.draw_background_only(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
            );
            draw_controls_ui(
                &mut rgba,
                config::WIDTH as u32,
                config::HEIGHT as u32,
                ControlsUiMode::Gameplay,
                false,
                false,
            );
        } else {
            self.draw_gameplay_screen(
                &mut rgba,
                box_trail,
                None,
                draw_player,
                false,
                show_solved_overlay,
            );
        }
        if fast_partial {
            self.display.present_rgba_fast_partial(&rgba)
        } else {
            self.display.present_rgba(&rgba)
        }
    }

    fn run_player_blink_animation(&mut self) -> Result<()> {
        let mut blink_frame = vec![0u8; config::WIDTH * config::HEIGHT * 4];
        self.draw_gameplay_screen(&mut blink_frame, None, None, true, true, false);
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

    fn run_box_vanish_animation(
        &mut self,
        to_x: u32,
        to_y: u32,
        show_solved_overlay: bool,
    ) -> Result<()> {
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
            return self.render_with_options(None, true, true, show_solved_overlay);
        }
        let base_x = box_x + ((raw_base_size - base_size) / 2) as i32;
        let base_y = box_y + ((raw_base_size - base_size) / 2) as i32;

        let steps = config::BOX_VANISH_STEPS.max(1);
        for step in 0..steps {
            let mut frame = vec![0u8; config::WIDTH * config::HEIGHT * 4];
            self.draw_gameplay_screen(&mut frame, None, None, true, false, show_solved_overlay);

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

        self.render_with_options(None, true, true, show_solved_overlay)
    }
}

impl FrameSink for KindleApp {
    type Error = std::io::Error;

    fn render_frame(&mut self, request: &FrameRequest) -> std::result::Result<(), Self::Error> {
        if !is_gameplay_screen(&self.app_state) {
            return Ok(());
        }
        match request {
            FrameRequest::Gameplay {
                box_trail,
                draw_player,
                show_solved_overlay,
                present_mode,
            } => self.render_with_options(
                box_trail.as_deref(),
                *draw_player,
                matches!(present_mode, PresentMode::FastPartial),
                *show_solved_overlay,
            ),
            FrameRequest::Menu { .. } => Ok(()),
        }
    }

    fn animate_player_blink(&mut self) -> std::result::Result<(), Self::Error> {
        self.run_player_blink_animation()
    }

    fn animate_box_vanish(
        &mut self,
        to_x: u32,
        to_y: u32,
        show_solved_overlay: bool,
    ) -> std::result::Result<(), Self::Error> {
        self.run_box_vanish_animation(to_x, to_y, show_solved_overlay)
    }

    fn animate_box_path_disappear(
        &mut self,
        _path: &[(u32, u32)],
        _show_solved_overlay: bool,
    ) -> std::result::Result<(), Self::Error> {
        Ok(())
    }
}
