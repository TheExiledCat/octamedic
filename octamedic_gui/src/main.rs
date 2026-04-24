use ggez::{ ContextBuilder, GameResult, conf::WindowMode, event };
use taffy::TaffyTree;

use crate::app::app::App;

mod app;
mod framework;
fn main() -> GameResult<()> {
    let (ctx, event_loop) = ContextBuilder::new("OctaMEDIC", "Amano Rosuko")
        .backend(ggez::conf::Backend::Vulkan)
        .window_mode(WindowMode { maximized: true, ..Default::default() })
        .build()?;
    let taffy = TaffyTree::new();
    let app = App::new(taffy);

    event::run(ctx, event_loop, app);
}
