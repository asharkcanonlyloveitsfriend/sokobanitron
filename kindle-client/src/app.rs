use crate::{config, level, platform, ui};
use renderer::{BoardViewport, Renderer};
use sokobanitron_gameplay::GameplaySession;
use std::io::Result;

pub struct KindleApp {
    renderer: Renderer,
    session: GameplaySession,
    viewport: BoardViewport,
    display: platform::Display,
}

impl KindleApp {
    pub fn new() -> Result<Self> {
        let session = GameplaySession::from_level_ascii(level::portrait_level_ascii());
        let board_area_height = (config::HEIGHT.saturating_sub(config::FOOTER_HEIGHT)) as u32;
        let viewport =
            BoardViewport::fit_to_window(config::WIDTH as u32, board_area_height, session.board());
        Ok(Self {
            renderer: Renderer::new(),
            session,
            viewport,
            display: platform::Display::new()?,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        self.render()?;

        let mut touch = platform::TouchReader::new()?;
        loop {
            let (raw_x, raw_y) = touch.next_tap_raw()?;
            self.on_tap(raw_x, raw_y)?;
        }
    }

    fn update_viewport(&mut self) {
        let board_area_height = (config::HEIGHT.saturating_sub(config::FOOTER_HEIGHT)) as u32;
        self.viewport =
            BoardViewport::fit_to_window(config::WIDTH as u32, board_area_height, self.session.board());
    }

    fn render(&mut self) -> Result<()> {
        let mut rgba = vec![0u8; config::WIDTH * config::HEIGHT * 4];
        self.renderer.draw(
            &mut rgba,
            config::WIDTH as u32,
            config::HEIGHT as u32,
            self.session.board(),
            &self.viewport,
        );
        ui::draw_restart_ui(&mut rgba);
        self.display.present_rgba(&rgba)
    }

    fn on_tap(&mut self, raw_x: i32, raw_y: i32) -> Result<()> {
        let (screen_x, screen_y) = platform::map_touch_to_screen(raw_x, raw_y)?;
        let restart = ui::restart_button_rect();

        if restart.contains(screen_x, screen_y) {
            self.session.restart();
            self.update_viewport();
            self.render()?;
            return Ok(());
        }

        if let Some((x, y)) =
            self.viewport
                .screen_to_cell(screen_x as f64, screen_y as f64, self.session.board())
        {
            self.session.click_cell(x, y);
            self.render()?;
        }
        Ok(())
    }
}
