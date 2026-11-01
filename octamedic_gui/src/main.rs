use std::path::PathBuf;

use ggez::{
    ContextBuilder, GameResult,
    conf::{WindowMode, WindowSetup},
    event,
};
use octamed::mmd::{conversion::FromModule, parser::OctamedMMDParser};
use octamedic_core::data::project::OctamedicProject;

use crate::app::{SCREEN, app::App, files};

mod app;
mod framework;

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

// window pixels per screen pixel across; screen pixels are twice as tall
const WINDOW_SCALE: f32 = 2.0;

fn main() -> GameResult<()> {
    if let Err(error) = files::initialize() {
        eprintln!("Couldn't create OctaMEDIC's folders: {error}");
    }
    let project = load_project();
    let [width, height] = SCREEN.window_size(WINDOW_SCALE);
    let (mut ctx, event_loop) = ContextBuilder::new("OctaMEDIC", "Amano Rosuko")
        .backend(ggez::conf::Backend::Vulkan)
        .window_mode(WindowMode {
            maximized: false,
            fullscreen_type: ggez::conf::FullscreenType::Windowed,
            borderless: false,
            width,
            height,
            resizable: true,
            ..Default::default()
        })
        // vsync queues frames and the pointer ends up trailing the mouse, so App paces itself
        .window_setup(WindowSetup::default().title("OctaMEDIC").vsync(false))
        .build()?;
    let app = App::new(&mut ctx, project)?;
    event::run(ctx, event_loop, app);
}
