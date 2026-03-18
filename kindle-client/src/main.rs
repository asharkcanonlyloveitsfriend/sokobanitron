mod app;
mod config;
mod level;
mod platform;
mod ui;

use std::io::Result;

fn main() -> Result<()> {
    let mut app = app::KindleApp::new();
    app.run()
}
