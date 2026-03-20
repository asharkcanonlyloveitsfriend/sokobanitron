mod app_driver;
mod config;
mod display;
mod platform;

use app_driver::KindleApp;
use std::io::Result;

fn main() -> Result<()> {
    let mut app = KindleApp::new()?;
    app.run()
}
