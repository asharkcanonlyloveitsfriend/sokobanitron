mod app;
mod config;
mod platform;

use std::io::Result;

fn main() -> Result<()> {
    let mut app = app::KindleApp::new()?;
    app.run()
}
