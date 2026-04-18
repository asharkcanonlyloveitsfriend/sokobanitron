use crate::app_driver::App;
use presentation::renderer::{
    draw_controls_ui, draw_gameplay_menu_level_set_button, draw_overlay_primary_action_button,
    draw_top_menu_toggle,
};
use sokobanitron_app::app::{FrameRequest, FrameSink, build_current_app_screen_frame_request};

impl App {
    pub(crate) fn render_current(&mut self) {
        let request =
            build_current_app_screen_frame_request(&self.controller, &self.app_state, &self.editor);
        let _ = self.render_request(&request);
    }

    pub(crate) fn render_active_gameplay_presentation(&mut self) {
        if let Some(pixels) = &mut self.pixels {
            let result = self
                .gameplay_presentation
                .advance_presentation_with_damage();
            let gray_frame = &mut self.gray_frame;
            // Desktop keeps full RGBA presentation, but still reuses the shared partial gameplay
            // redraw path inside the persistent grayscale buffer.
            self.gameplay_presentation.draw_damage(
                &mut self.renderer,
                gray_frame,
                self.surface_width,
                self.surface_height,
                &result.damage,
            );
            copy_gray_to_rgba(gray_frame, pixels.frame_mut());
            pixels.render().expect("render");
            if result.has_pending_presentation {
                self.request_window_redraw();
            }
        }
    }

    fn render_request(&mut self, request: &FrameRequest) -> Result<(), ()> {
        match request {
            FrameRequest::Gameplay { update, .. } => {
                if let Some(pixels) = &mut self.pixels {
                    let result = self
                        .gameplay_presentation
                        .replace_update_with_damage(update.clone());
                    let gray_frame = &mut self.gray_frame;
                    self.gameplay_presentation.draw_damage(
                        &mut self.renderer,
                        gray_frame,
                        self.surface_width,
                        self.surface_height,
                        &result.damage,
                    );
                    copy_gray_to_rgba(gray_frame, pixels.frame_mut());
                    pixels.render().expect("render");
                    if result.has_pending_presentation {
                        self.request_window_redraw();
                    }
                }
            }
            FrameRequest::GameplayMenu { screen } => {
                self.gameplay_presentation.clear();
                if let Some(pixels) = &mut self.pixels {
                    let gray_frame = &mut self.gray_frame;
                    self.renderer.draw_background_only(
                        gray_frame,
                        self.surface_width,
                        self.surface_height,
                    );
                    let theme = self.renderer.theme();
                    draw_top_menu_toggle(
                        gray_frame,
                        self.surface_width,
                        self.surface_height,
                        true,
                        theme,
                    );
                    if screen.show_change_level_set {
                        draw_gameplay_menu_level_set_button(
                            gray_frame,
                            self.surface_width,
                            self.surface_height,
                            theme,
                        );
                    }
                    if let Some(icon) = screen.primary_action_icon {
                        draw_overlay_primary_action_button(
                            gray_frame,
                            self.surface_width,
                            self.surface_height,
                            icon,
                            theme.gray_2,
                        );
                    }
                    copy_gray_to_rgba(gray_frame, pixels.frame_mut());
                    pixels.render().expect("render");
                }
            }
            FrameRequest::LevelSelect { screen, .. } => {
                self.gameplay_presentation.clear();
                if let Some(pixels) = &mut self.pixels {
                    let gray_frame = &mut self.gray_frame;
                    self.renderer.draw_background_only(
                        gray_frame,
                        self.surface_width,
                        self.surface_height,
                    );
                    self.renderer.draw_level_select_menu_contents(
                        gray_frame,
                        self.surface_width,
                        self.surface_height,
                        &self.preview_boards,
                        screen.resume_level,
                        screen.page_start,
                    );
                    draw_controls_ui(
                        gray_frame,
                        self.surface_width,
                        self.surface_height,
                        true,
                        self.renderer.theme(),
                    );
                    copy_gray_to_rgba(gray_frame, pixels.frame_mut());
                    pixels.render().expect("render");
                }
            }
            FrameRequest::LevelSetSelect { screen, .. } => {
                self.gameplay_presentation.clear();
                if let Some(pixels) = &mut self.pixels {
                    let gray_frame = &mut self.gray_frame;
                    self.renderer.draw_background_only(
                        gray_frame,
                        self.surface_width,
                        self.surface_height,
                    );
                    self.renderer.draw_level_set_select_menu_contents(
                        gray_frame,
                        self.surface_width,
                        self.surface_height,
                        screen,
                    );
                    draw_controls_ui(
                        gray_frame,
                        self.surface_width,
                        self.surface_height,
                        true,
                        self.renderer.theme(),
                    );
                    copy_gray_to_rgba(gray_frame, pixels.frame_mut());
                    pixels.render().expect("render");
                }
            }
            FrameRequest::Editor { screen } => {
                self.gameplay_presentation.clear();
                if let Some(pixels) = &mut self.pixels {
                    let gray_frame = &mut self.gray_frame;
                    self.renderer.draw_editor_screen(
                        gray_frame,
                        self.surface_width,
                        self.surface_height,
                        screen,
                    );
                    copy_gray_to_rgba(gray_frame, pixels.frame_mut());
                    pixels.render().expect("render");
                }
            }
            FrameRequest::EditorMenu { screen } => {
                self.gameplay_presentation.clear();
                if let Some(pixels) = &mut self.pixels {
                    let gray_frame = &mut self.gray_frame;
                    self.renderer.draw_editor_menu(
                        gray_frame,
                        self.surface_width,
                        self.surface_height,
                        screen,
                    );
                    copy_gray_to_rgba(gray_frame, pixels.frame_mut());
                    pixels.render().expect("render");
                }
            }
        }
        Ok(())
    }
}

fn copy_gray_to_rgba(gray: &[u8], rgba: &mut [u8]) {
    for (src, dst) in gray.iter().zip(rgba.chunks_exact_mut(4)) {
        dst[0] = *src;
        dst[1] = *src;
        dst[2] = *src;
        dst[3] = 255;
    }
}

impl FrameSink for App {
    type Error = ();

    fn render_frame(&mut self, request: &FrameRequest) -> Result<(), Self::Error> {
        self.render_request(request)
    }
}
