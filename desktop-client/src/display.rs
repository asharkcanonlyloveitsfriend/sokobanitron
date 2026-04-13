use crate::app_driver::App;
use presentation::renderer::{
    draw_controls_ui, draw_gameplay_menu_level_set_button, draw_overlay_primary_action_button,
    draw_top_menu_toggle,
};
use sokobanitron_app::{
    app::{AppScreen, FrameRequest, FrameSink},
    editor::build_current_editor_frame_request,
    gameplay::build_current_frame_request,
};

const BUTTON_TEXT_COLOR: [u8; 4] = [220, 220, 220, 255];

impl App {
    pub(crate) fn render_current(&mut self) {
        let request = match self.app_state.active_screen() {
            AppScreen::Gameplay => build_current_frame_request(&self.controller, &self.app_state),
            AppScreen::Editor => build_current_editor_frame_request(&self.app_state, &self.editor),
        };
        let _ = self.render_request(&request);
    }

    pub(crate) fn render_active_gameplay_screen(&mut self) {
        let request = build_current_frame_request(&self.controller, &self.app_state);
        let _ = self.render_request(&request);
    }

    pub(crate) fn render_active_gameplay_presentation(&mut self) {
        if let Some(pixels) = &mut self.pixels {
            let frame = pixels.frame_mut();
            self.gameplay_presentation.draw(
                &mut self.renderer,
                frame,
                self.surface_width,
                self.surface_height,
            );
            pixels.render().expect("render");
            if self.gameplay_presentation.has_active_animation() {
                self.request_window_redraw();
            }
        }
    }

    fn render_request(&mut self, request: &FrameRequest) -> Result<(), ()> {
        match request {
            FrameRequest::Gameplay { update, .. } => {
                if let Some(pixels) = &mut self.pixels {
                    self.gameplay_presentation.replace_update(update.clone());
                    let frame = pixels.frame_mut();
                    self.gameplay_presentation.draw(
                        &mut self.renderer,
                        frame,
                        self.surface_width,
                        self.surface_height,
                    );
                    pixels.render().expect("render");
                    if self.gameplay_presentation.has_active_animation() {
                        self.request_window_redraw();
                    }
                }
            }
            FrameRequest::GameplayMenu { screen } => {
                if let Some(pixels) = &mut self.pixels {
                    let frame = pixels.frame_mut();
                    self.renderer.draw_background_only(
                        frame,
                        self.surface_width,
                        self.surface_height,
                    );
                    draw_top_menu_toggle(frame, self.surface_width, self.surface_height, true);
                    if screen.show_change_level_set {
                        draw_gameplay_menu_level_set_button(
                            frame,
                            self.surface_width,
                            self.surface_height,
                        );
                    }
                    if let Some(icon) = screen.primary_action_icon {
                        draw_overlay_primary_action_button(
                            frame,
                            self.surface_width,
                            self.surface_height,
                            icon,
                            BUTTON_TEXT_COLOR,
                        );
                    }
                    pixels.render().expect("render");
                }
            }
            FrameRequest::LevelSelect { screen, .. } => {
                if let Some(pixels) = &mut self.pixels {
                    let frame = pixels.frame_mut();
                    self.renderer.draw_background_only(
                        frame,
                        self.surface_width,
                        self.surface_height,
                    );
                    self.renderer.draw_level_select_menu_contents(
                        frame,
                        self.surface_width,
                        self.surface_height,
                        &self.preview_boards,
                        screen.resume_level,
                        screen.page_start,
                    );
                    draw_controls_ui(frame, self.surface_width, self.surface_height, true);
                    pixels.render().expect("render");
                }
            }
            FrameRequest::LevelSetSelect { screen, .. } => {
                if let Some(pixels) = &mut self.pixels {
                    let frame = pixels.frame_mut();
                    self.renderer.draw_background_only(
                        frame,
                        self.surface_width,
                        self.surface_height,
                    );
                    self.renderer.draw_level_set_select_menu_contents(
                        frame,
                        self.surface_width,
                        self.surface_height,
                        screen,
                    );
                    draw_controls_ui(frame, self.surface_width, self.surface_height, true);
                    pixels.render().expect("render");
                }
            }
            FrameRequest::Editor { screen } => {
                if let Some(pixels) = &mut self.pixels {
                    let frame = pixels.frame_mut();
                    self.renderer.draw_editor_screen(
                        frame,
                        self.surface_width,
                        self.surface_height,
                        screen,
                    );
                    pixels.render().expect("render");
                }
            }
            FrameRequest::EditorMenu { screen } => {
                if let Some(pixels) = &mut self.pixels {
                    let frame = pixels.frame_mut();
                    self.renderer.draw_editor_menu(
                        frame,
                        self.surface_width,
                        self.surface_height,
                        screen,
                    );
                    pixels.render().expect("render");
                }
            }
        }
        Ok(())
    }
}

impl FrameSink for App {
    type Error = ();

    fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error> {
        if !self.app_state.is_gameplay_screen() {
            return Ok(());
        }
        self.render_request(request)
    }
}
