use ggez::{
    ContextBuilder, GameResult,
    conf::{WindowMode, WindowSetup},
    event,
};
use octamedic_core::data::project::OctamedicProject;

use crate::app::app::App;

mod app;
mod framework;

fn main() -> GameResult<()> {
    let project = OctamedicProject::new();
    let (mut ctx, event_loop) = ContextBuilder::new("OctaMEDIC", "Amano Rosuko")
        .backend(ggez::conf::Backend::Vulkan)
        .window_mode(WindowMode {
            maximized: false,
            fullscreen_type: ggez::conf::FullscreenType::Windowed,
            borderless: false,
            // the screen at 2×, with its tall pixels
            width: 1280.0,
            height: 1024.0,
            resizable: true,
            ..Default::default()
        })
        // vsync queues frames, which makes the pointer trail the mouse; the app caps its own
        // frame rate instead
        .window_setup(WindowSetup::default().title("OctaMEDIC").vsync(false))
        .build()?;
    let app = App::new(&mut ctx, project)?;
    event::run(ctx, event_loop, app);
}
