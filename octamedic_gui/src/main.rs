use std::path::PathBuf;

use ggez::{
    ContextBuilder, GameResult,
    conf::{WindowMode, WindowSetup},
    event,
};
use octamed::mmd::{conversion::FromModule, parser::OctamedMMDParser};
use octamedic_core::data::project::OctamedicProject;

use crate::{
    app::{app::App, files},
    framework::screen::{PIXEL_ASPECT, SCREEN_HEIGHT, SCREEN_WIDTH},
};

mod app;
mod framework;

/// The module given as the first argument, or a new project without one.
fn load_project() -> OctamedicProject {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        return OctamedicProject::new();
    };
    match OctamedMMDParser::new().parse_file(&path) {
        Ok(modules) => match modules.first() {
            Some(module) => return OctamedicProject::from_module(module),
            None => eprintln!("{} has no module in it", path.display()),
        },
        Err(error) => eprintln!("Couldn't open {}: {error}", path.display()),
    }
    return OctamedicProject::new();
}

/// How many window pixels a screen pixel starts as, across; its tall pixels are twice that down.
const WINDOW_SCALE: i32 = 2;

fn main() -> GameResult<()> {
    if let Err(error) = files::initialize() {
        eprintln!("Couldn't create OctaMEDIC's folders: {error}");
    }
    let project = load_project();
    let (mut ctx, event_loop) = ContextBuilder::new("OctaMEDIC", "Amano Rosuko")
        .backend(ggez::conf::Backend::Vulkan)
        .window_mode(WindowMode {
            maximized: false,
            fullscreen_type: ggez::conf::FullscreenType::Windowed,
            borderless: false,
            width: (SCREEN_WIDTH * WINDOW_SCALE) as f32,
            height: SCREEN_HEIGHT as f32 * PIXEL_ASPECT * WINDOW_SCALE as f32,
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
