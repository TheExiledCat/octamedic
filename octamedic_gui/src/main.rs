use ggez::{ContextBuilder, GameResult, conf::WindowMode, event};
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
            ..Default::default()
        })
        .build()?;
    let app = App::new(&mut ctx, project)?;
    event::run(ctx, event_loop, app);
}
