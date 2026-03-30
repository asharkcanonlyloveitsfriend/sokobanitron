mod app_driver;
mod display;

use app_driver::App;
use std::io;
use winit::event_loop::EventLoop;

fn main() -> io::Result<()> {
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App::new()?;
    event_loop.run_app(&mut app).expect("run app");
    Ok(())
}
