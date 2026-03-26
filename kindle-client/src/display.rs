use crate::{app_driver::KindleApp, config};
use presentation::layout::ControlsUiMode;
use presentation::renderer::{
    Renderer, RendererOverrides, draw_controls_ui, draw_overlay_primary_action_button,
    draw_top_menu_toggle,
};
use sokobanitron_app::{
    app::{FrameRequest, FrameSink, PresentMode},
    gameplay::build_current_frame_request,
};
use std::io::Result;

const KINDLE_BOX_PRIMARY: [u8; 4] = [60, 63, 66, 255];
const KINDLE_BOX_SHADOW: [u8; 4] = [30, 31, 33, 255];
const KINDLE_PLAYER_BODY: [u8; 4] = [117, 117, 117, 255];
const KINDLE_PLAYER_LIMB: [u8; 4] = [80, 80, 80, 255];

impl KindleApp {
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
        let request = build_current_frame_request(&self.controller, &self.app_state);
        self.render_request(&request)
    }

    fn render_request(&mut self, request: &FrameRequest) -> Result<()> {
        match request {
            FrameRequest::Gameplay {
                screen,
                present_mode,
            } => {
                let (renderer, rgba, display, controller, viewport) = (
                    &mut self.renderer,
                    &mut self.rgba_frame,
                    &mut self.display,
                    &self.controller,
                    &self.viewport,
                );
                renderer.draw_gameplay_screen(
                    rgba,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                    controller.board(),
                    viewport,
                    screen,
                );
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
                let (renderer, rgba, display, preview_boards, controller) = (
                    &mut self.renderer,
                    &mut self.rgba_frame,
                    &mut self.display,
                    &self.preview_boards,
                    &self.controller,
                );
                renderer.draw_background_only(rgba, config::WIDTH as u32, config::HEIGHT as u32);
                renderer.draw_level_select_menu_contents(
                    rgba,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                    preview_boards,
                    controller.current_level(),
                    screen.page_start,
                );
                draw_controls_ui(
                    rgba,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                    ControlsUiMode::MenuOpen,
                    false,
                    false,
                );
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
