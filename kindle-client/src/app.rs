use crate::{config, level, platform, ui};
use renderer::{BoardViewport, BoardViewportOptions, Renderer, RendererOverrides};
use sokobanitron_gameplay::{GameplayKey, GameplaySession};
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
        let viewport = Self::compute_viewport(session.board());
        Ok(Self {
            renderer: Renderer::with_overrides(RendererOverrides {
                selected_box_primary: Some(config::KINDLE_SELECTED_BOX_PRIMARY),
                selected_box_highlight: Some(config::KINDLE_SELECTED_BOX_HIGHLIGHT),
                selected_box_shadow: Some(config::KINDLE_SELECTED_BOX_SHADOW),
                ..RendererOverrides::default()
            }),
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
        self.viewport = Self::compute_viewport(self.session.board());
    }

    fn compute_viewport(board: &sokobanitron_gameplay::BoardView) -> BoardViewport {
        let h_margin = config::BOARD_HORIZONTAL_MARGIN as u32;
        let v_margin = config::BOARD_VERTICAL_MARGIN as u32;
        let safe_width = (config::WIDTH as u32).saturating_sub(h_margin * 2).max(1);
        let safe_height = (config::HEIGHT as u32).saturating_sub(v_margin * 2).max(1);

        let mut viewport = BoardViewport::fit_to_window_with_options(
            safe_width,
            safe_height,
            board,
            BoardViewportOptions::fill_available_space(),
        );
        viewport.origin_x += h_margin as i32;
        viewport.origin_y += v_margin as i32;
        viewport
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
        ui::draw_controls_ui(&mut rgba);
        self.display.present_rgba(&rgba)
    }

    fn on_tap(&mut self, raw_x: i32, raw_y: i32) -> Result<()> {
        let (screen_x, screen_y) = platform::map_touch_to_screen(raw_x, raw_y)?;
        if let Some(action) = ui::button_action_at(screen_x, screen_y) {
            match action {
                ui::ButtonAction::Restart => {
                    self.session.restart();
                    self.update_viewport();
                    self.render()?;
                    return Ok(());
                }
                ui::ButtonAction::Undo => {
                    self.session.on_key(GameplayKey::Backspace);
                    self.update_viewport();
                    self.render()?;
                    return Ok(());
                }
                ui::ButtonAction::Previous | ui::ButtonAction::Next => {
                    return Ok(());
                }
            }
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
