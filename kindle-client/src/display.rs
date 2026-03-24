use crate::{app_driver::KindleApp, config};
use renderer::{
    ControlsUiMode, Renderer, RendererOverrides, UiIcon, draw_controls_ui,
    draw_overlay_primary_action_button, draw_top_menu_toggle,
};
use sokobanitron_app::{
    AppScreen, FrameRequest, FrameSink, PresentMode, active_screen, build_current_frame_request,
    is_editor_menu_open, is_gameplay_screen,
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
        // Editor-side screens are rendered directly here.
        if is_editor_menu_open(&self.app_state) {
            let mut rgba = vec![0u8; config::WIDTH * config::HEIGHT * 4];
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
            self.display.present_rgba(&rgba)
        } else if matches!(active_screen(&self.app_state), AppScreen::Editor) {
            let mut rgba = vec![0u8; config::WIDTH * config::HEIGHT * 4];
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
            self.display.present_rgba(&rgba)
        } else {
            let request = build_current_frame_request(&self.controller, &self.app_state);
            self.render_request(&request)
        }
    }

    fn render_request(&mut self, request: &FrameRequest) -> Result<()> {
        match request {
            FrameRequest::Gameplay {
                screen,
                present_mode,
            } => {
                let mut rgba = vec![0u8; config::WIDTH * config::HEIGHT * 4];
                self.renderer.draw_gameplay_screen(
                    &mut rgba,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                    self.controller.board(),
                    &self.viewport,
                    screen,
                );
                if matches!(present_mode, PresentMode::FastPartial) {
                    self.display.present_rgba_fast_partial(&rgba)
                } else {
                    self.display.present_rgba(&rgba)
                }
            }
            FrameRequest::GameplayMenu => {
                let mut rgba = vec![0u8; config::WIDTH * config::HEIGHT * 4];
                self.renderer.draw_background_only(
                    &mut rgba,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                );
                draw_top_menu_toggle(&mut rgba, config::WIDTH as u32, config::HEIGHT as u32, true);
                draw_overlay_primary_action_button(
                    &mut rgba,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                    UiIcon::Draw,
                    [220, 220, 220, 255],
                );
                self.display.present_rgba(&rgba)
            }
            FrameRequest::LevelSelect {
                screen,
                present_mode,
            } => {
                let mut rgba = vec![0u8; config::WIDTH * config::HEIGHT * 4];
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
                    screen.page_start,
                );
                draw_controls_ui(
                    &mut rgba,
                    config::WIDTH as u32,
                    config::HEIGHT as u32,
                    ControlsUiMode::MenuOpen,
                    false,
                    false,
                );
                if matches!(present_mode, PresentMode::FastPartial) {
                    self.display.present_rgba_fast_partial(&rgba)
                } else {
                    self.display.present_rgba(&rgba)
                }
            }
        }
    }
}

impl FrameSink for KindleApp {
    type Error = std::io::Error;

    fn render_frame(&mut self, request: &FrameRequest) -> std::result::Result<(), Self::Error> {
        if !is_gameplay_screen(&self.app_state) {
            return Ok(());
        }
        self.render_request(request)
    }
}
