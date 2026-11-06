// a GUI program on Windows, so it doesn't open a console window first
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::{convert::Infallible, path::PathBuf};

use ggez::{
    Context, ContextBuilder, GameResult,
    conf::{WindowMode, WindowSetup},
    event,
    winit::{dpi::PhysicalPosition, monitor::MonitorHandle, window::Window},
};
use octamedic_core::data::project::OctamedicProject;
use rustuition::screen::Screen;

use crate::app::{DEFAULT_VIDEO, app::App, files, icon, screen};

mod app;
mod crash_log;
mod framework;

// just the file's name in the error, a whole path doesn't fit in the notification bar
fn load_project() -> Result<OctamedicProject, String> {
    let Some(path) = std::env::args_os().nth(1).map(PathBuf::from) else {
        return Ok(OctamedicProject::new());
    };
    return files::load_module(&path).map_err(|error| {
        let name = path.file_name().unwrap_or(path.as_os_str());
        format!("Couldn't open {}: {error}", name.to_string_lossy())
    });
}

// window points per screen pixel across; screen pixels are twice as tall
const WINDOW_SCALE: f32 = 2.0;
// how much of the monitor the window can start out on, leaving room for the menu bar, taskbar or
// dock, which winit doesn't tell us about
const MONITOR_SHARE: f32 = 0.9;

fn main() {
    crash_log::log_panics();
    // shown once the window's open, there's no console to print them to on Windows
    let mut errors = Vec::new();
    if let Err(error) = files::initialize() {
        errors.push(format!("Couldn't create OctaMEDIC's folders: {error}"));
    }
    let project = load_project().unwrap_or_else(|error| {
        errors.push(error);
        OctamedicProject::new()
    });
    // only comes back if the window couldn't open
    let Err(error) = run(project, &errors);
    crash_log::log_error(&error.to_string(), &errors);
    std::process::exit(1);
}

fn run(project: OctamedicProject, errors: &[String]) -> GameResult<Infallible> {
    let [width, height] = screen(DEFAULT_VIDEO).window_size(WINDOW_SCALE);
    let mode = WindowMode {
        maximized: false,
        fullscreen_type: ggez::conf::FullscreenType::Windowed,
        borderless: false,
        width,
        height,
        resizable: true,
        // shown by open_window, once it knows the monitor
        visible: false,
        // keeps its size in points when moved to a monitor with more or fewer pixels per point
        resize_on_scale_factor_change: true,
        ..Default::default()
    };
    let (mut ctx, event_loop) = ContextBuilder::new("OctaMEDIC", "Amano Rosuko")
        .window_mode(mode)
        // vsync queues frames and the pointer ends up trailing the mouse, so App paces itself
        .window_setup(WindowSetup::default().title("OctaMEDIC").vsync(false))
        .build()?;
    open_window(&mut ctx, mode)?;
    let app = App::new(&mut ctx, project, errors.to_vec())?;
    icon::set_window_icons(ctx.gfx.window())?;
    event::run(ctx, event_loop, app);
}

// sizes are in physical pixels, so a 2x display (Retina) needs twice as many for the same size
fn open_window(ctx: &mut Context, mode: WindowMode) -> GameResult {
    let screen = screen(DEFAULT_VIDEO);
    let window = ctx.gfx.window();
    let most = WINDOW_SCALE * window.scale_factor() as f32;
    let [frame_width, frame_height] = frame_size(window);
    let monitor = window
        .current_monitor()
        .or_else(|| window.primary_monitor());
    let space = monitor.as_ref().map(|monitor| {
        let size = monitor.size();
        [
            size.width as f32 * MONITOR_SHARE - frame_width,
            size.height as f32 * MONITOR_SHARE - frame_height,
        ]
    });
    let [width, height] = screen.window_size(start_scale(screen, most, space));
    if let Some(monitor) = monitor {
        let outer = [width + frame_width, height + frame_height];
        ctx.gfx.set_window_position(centered(&monitor, outer))?;
    }
    return ctx.gfx.set_mode(WindowMode {
        width,
        height,
        visible: true,
        ..mode
    });
}

// the title bar and borders, which the window's size doesn't count
fn frame_size(window: &Window) -> [f32; 2] {
    let (outer, inner) = (window.outer_size(), window.inner_size());
    return [
        outer.width.saturating_sub(inner.width) as f32,
        outer.height.saturating_sub(inner.height) as f32,
    ];
}

// the screen's scaled to fit the window, so any size that fits will do
fn start_scale(screen: Screen, most: f32, space: Option<[f32; 2]>) -> f32 {
    let [width, height] = screen.window_size(1.0);
    let fit = space.map_or(most, |[space_width, space_height]| {
        (space_width / width).min(space_height / height)
    });
    return most.min(fit).max(1.0);
}

fn centered(monitor: &MonitorHandle, size: [f32; 2]) -> PhysicalPosition<i32> {
    let (position, area) = (monitor.position(), monitor.size());
    let offset = |area: u32, size: f32| ((area as f32 - size) / 2.0).max(0.0) as i32;
    return PhysicalPosition::new(
        position.x + offset(area.width, size[0]),
        position.y + offset(area.height, size[1]),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Screen = Screen::PAL_HIRES;

    #[test]
    fn start_scale_is_the_most_that_fits() {
        // a 5K display at 2x, with the space it leaves for 2x points (4)
        assert_eq!(start_scale(SCREEN, 4.0, Some([4608.0, 2536.0])), 4.0);
        // a 13" MacBook at 2x is too short for 4
        assert_eq!(start_scale(SCREEN, 4.0, Some([2646.0, 1664.0])), 3.25);
        // 1080p at 1x, too short for 2
        assert_eq!(
            start_scale(SCREEN, 2.0, Some([1712.0, 941.0])),
            941.0 / 512.0
        );
    }

    #[test]
    fn start_scale_needs_not_be_whole() {
        // 125% Windows scaling
        assert_eq!(start_scale(SCREEN, 2.5, Some([3000.0, 3000.0])), 2.5);
    }

    #[test]
    fn start_scale_without_a_monitor() {
        assert_eq!(start_scale(SCREEN, 4.0, None), 4.0);
    }

    #[test]
    fn start_scale_at_least_one() {
        assert_eq!(start_scale(SCREEN, 2.0, Some([300.0, 200.0])), 1.0);
    }
}
