use crate::app_driver::App;
use renderer::{
    ControlsUiMode, UiIcon, draw_controls_ui, draw_overlay_primary_action_button,
    draw_top_menu_toggle,
};
use sokobanitron_app::{
    AppScreen, FrameRequest, FrameSink, active_screen, build_current_frame_request,
    is_editor_menu_open, is_gameplay_screen,
};

impl App {
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
            FrameRequest::GameplayMenu => {
                if let Some(pixels) = &mut self.pixels {
                    let frame = pixels.frame_mut();
                    self.renderer.draw_background_only(
                        frame,
                        self.surface_width,
                        self.surface_height,
                    );
                    draw_top_menu_toggle(frame, self.surface_width, self.surface_height, true);
                    draw_overlay_primary_action_button(
                        frame,
                        self.surface_width,
                        self.surface_height,
                        UiIcon::Draw,
                        [220, 220, 220, 255],
                    );
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
        }
        Ok(())
    }

    pub(crate) fn render_current(&mut self) {
        match active_screen(&self.app_state) {
            AppScreen::Gameplay => self.render_active_gameplay_screen(),
            AppScreen::Editor => self.render_editor_mode(),
        }
    }

    pub(crate) fn render_active_gameplay_screen(&mut self) {
        let request = build_current_frame_request(&self.controller, &self.app_state);
        let _ = self.render_request(&request);
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
