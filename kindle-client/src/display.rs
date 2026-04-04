use crate::{app_driver::KindleApp, config};
use presentation::renderer::{
    Renderer, RendererOverrides, draw_controls_ui, draw_gameplay_menu_level_set_button,
    draw_overlay_primary_action_button, draw_top_menu_toggle,
};
use sokobanitron_app::{
    app::{FrameRequest, FrameSink, PresentMode},
    gameplay::{build_current_frame_request, build_sleep_gameplay_frame_request},
};
use std::io::Result;

const KINDLE_MID_1: [u8; 4] = [117, 117, 117, 255];
const KINDLE_MID_3: [u8; 4] = [60, 63, 66, 255];
const KINDLE_MID_4: [u8; 4] = [80, 80, 80, 255];
const KINDLE_DARK_1: [u8; 4] = [30, 31, 33, 255];

impl KindleApp {
    pub(crate) fn build_renderer() -> Renderer {
        Renderer::with_overrides(RendererOverrides {
            mid_1: Some(KINDLE_MID_1),
            mid_2: Some(config::KINDLE_MID_2),
            mid_3: Some(KINDLE_MID_3),
            mid_4: Some(KINDLE_MID_4),
            mid_5: Some(config::KINDLE_MID_5),
            dark_1: Some(KINDLE_DARK_1),
            dark_2: Some(config::KINDLE_DARK_2),
            ..RendererOverrides::default()
        })
    }

    pub(crate) fn render(&mut self) -> Result<()> {
        let request = build_current_frame_request(&self.controller, &self.app_state);
        self.render_request(&request)
    }

    fn render_request(&mut self, request: &FrameRequest) -> Result<()> {
        match request {
            FrameRequest::Gameplay {
                update,
                present_mode,
            } => {
                self.gameplay_presentation.replace_update(update.clone());
                let (renderer, rgba, display) =
                    (&mut self.renderer, &mut self.rgba_frame, &mut self.display);
                self.gameplay_presentation.draw(
                    renderer,
                    rgba,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                );
                // Keep the Kindle partial-refresh path available as a documented platform
                // capability even though gameplay no longer uses timed or animated presentation.
                if matches!(present_mode, PresentMode::FastPartial) {
                    display.present_rgba_fast_partial(rgba)
                } else {
                    display.present_rgba(rgba)
                }
            }
            FrameRequest::GameplayMenu { screen } => {
                let (renderer, rgba, display) =
                    (&mut self.renderer, &mut self.rgba_frame, &mut self.display);
                renderer.draw_background_only(rgba, config::WIDTH as u32, config::HEIGHT as u32);
                draw_top_menu_toggle(rgba, config::WIDTH as u32, config::HEIGHT as u32, true);
                if screen.show_change_level_set {
                    draw_gameplay_menu_level_set_button(
                        rgba,
                        config::WIDTH as u32,
                        config::HEIGHT as u32,
                    );
                }
                if let Some(icon) = screen.primary_action_icon {
                    draw_overlay_primary_action_button(
                        rgba,
                        config::WIDTH as u32,
                        config::HEIGHT as u32,
                        icon,
                        [220, 220, 220, 255],
                    );
                }
                display.present_rgba(rgba)
            }
            FrameRequest::LevelSelect {
                screen,
                present_mode,
            } => {
                let (renderer, rgba, display, preview_boards) = (
                    &mut self.renderer,
                    &mut self.rgba_frame,
                    &mut self.display,
                    &self.preview_boards,
                );
                renderer.draw_background_only(rgba, config::WIDTH as u32, config::HEIGHT as u32);
                renderer.draw_level_select_menu_contents(
                    rgba,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                    preview_boards,
                    screen.resume_level,
                    screen.page_start,
                );
                draw_controls_ui(rgba, config::WIDTH as u32, config::HEIGHT as u32, true);
                if matches!(present_mode, PresentMode::FastPartial) {
                    display.present_rgba_fast_partial(rgba)
                } else {
                    display.present_rgba(rgba)
                }
            }
            FrameRequest::LevelSetSelect {
                screen,
                present_mode,
            } => {
                let (renderer, rgba, display) =
                    (&mut self.renderer, &mut self.rgba_frame, &mut self.display);
                renderer.draw_background_only(rgba, config::WIDTH as u32, config::HEIGHT as u32);
                renderer.draw_level_set_select_menu_contents(
                    rgba,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                    screen,
                );
                draw_controls_ui(rgba, config::WIDTH as u32, config::HEIGHT as u32, true);
                if matches!(present_mode, PresentMode::FastPartial) {
                    display.present_rgba_fast_partial(rgba)
                } else {
                    display.present_rgba(rgba)
                }
            }
            FrameRequest::Editor { .. } | FrameRequest::EditorMenu { .. } => {
                let (renderer, rgba, display) =
                    (&mut self.renderer, &mut self.rgba_frame, &mut self.display);
                // Kindle still opts out of editor support; keep the fallback explicit until
                // that client is migrated onto a real editor presentation path.
                renderer.draw_background_only(rgba, config::WIDTH as u32, config::HEIGHT as u32);
                display.present_rgba(rgba)
            }
        }
    }

    pub(crate) fn render_sleep_screen(&mut self) -> Result<()> {
        let request = build_sleep_gameplay_frame_request(&self.controller, &self.app_state);
        self.render_request(&request)
    }
}

impl FrameSink for KindleApp {
    type Error = std::io::Error;

    fn render_frame(&mut self, request: &FrameRequest) -> std::result::Result<(), Self::Error> {
        if !self.app_state.is_gameplay_screen() {
            return Ok(());
        }
        self.render_request(request)
    }
}
