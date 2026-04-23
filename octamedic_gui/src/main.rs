use display_info::DisplayInfo;
use pix_engine::{ engine::Engine, prelude::* };

use crate::app::app::App;

mod app;
mod framework;
mod util;
fn main() -> PixResult<()> {
    let display = DisplayInfo::all().unwrap();
    let display = display
        .into_iter()
        .find(|d| d.is_primary)
        .unwrap();
    let mut engine = Engine::builder()
        .fullscreen()
        .dimensions(display.width, display.height)
        .borderless()
        .title("OctaMEDIC")
        .show_frame_rate()

        .build()?;
    let mut app = App::new();

    return engine.run(&mut app);
}
