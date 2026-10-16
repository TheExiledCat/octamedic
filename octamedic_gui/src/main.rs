use ggez::{ContextBuilder, GameResult, conf::WindowMode, event};

use crate::app::app::App;

mod app;
mod framework;

fn main() -> GameResult<()> {

    let (mut ctx, event_loop) = ContextBuilder::new("OctaMEDIC", "Amano Rosuko")
        .backend(ggez::conf::Backend::Vulkan)
        .window_mode(WindowMode {
            maximized: true,
            ..Default::default()
        })
        .build()?;

    let app = App::new(&mut ctx)?;

    event::run(ctx, event_loop, app);
}
