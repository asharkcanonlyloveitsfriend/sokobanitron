use crate::app_driver::App;
use presentation::layout::ControlsUiMode;
use presentation::renderer::{
    draw_controls_ui, draw_overlay_primary_action_button, draw_top_menu_toggle,
};
use sokobanitron_app::{
    FrameRequest, FrameSink, active_screen, build_current_editor_frame_request,
    build_current_frame_request, is_gameplay_screen,
};

const BUTTON_TEXT_COLOR: [u8; 4] = [220, 220, 220, 255];

impl App {
    pub(crate) fn render_current(&mut self) {
        let request = match active_screen(&self.app_state) {
            sokobanitron_app::AppScreen::Gameplay => {
                build_current_frame_request(&self.controller, &self.app_state)
            }
            sokobanitron_app::AppScreen::Editor => {
                build_current_editor_frame_request(&self.app_state, &self.editor)
            }
        };
        let _ = self.render_request(&request);
    }

    pub(crate) fn render_active_gameplay_screen(&mut self) {
        let request = build_current_frame_request(&self.controller, &self.app_state);
        let _ = self.render_request(&request);
    }

    fn render_request(&mut self, request: &FrameRequest) -> Result<(), ()> {
        match request {
            FrameRequest::Gameplay { screen, .. } => {
                if let Some(pixels) = &mut self.pixels {
                    let frame = pixels.frame_mut();
                    self.renderer.draw_gameplay_screen(
                        frame,
                        self.surface_width,
                        self.surface_height,
                        self.controller.board(),
                        &self.board_viewport,
                        screen,
                    );
                    pixels.render().expect("render");
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
                        self.controller.current_level(),
                        screen.page_start,
                    );
                    draw_controls_ui(
                        frame,
                        self.surface_width,
                        self.surface_height,
                        ControlsUiMode::MenuOpen,
                        false,
                        false,
                    );
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
        if !is_gameplay_screen(&self.app_state) {
            return Ok(());
        }
        self.render_request(request)
    }
}
