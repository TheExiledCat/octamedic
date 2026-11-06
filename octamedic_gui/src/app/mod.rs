use octamedic_core::amiga::clock::VideoStandard;
use rustuition::screen::Screen;

pub mod app;
pub mod copyright_bar;
pub mod cursor;
pub mod debug_console;
pub mod files;
pub mod fonts;
pub mod footer;
pub mod header;
pub mod icon;
pub mod keybinds;
pub mod main;
pub mod main_view;
pub mod mode;
pub mod output;
pub mod pens;
pub mod prompt;
pub mod screen_filters;
pub mod settings;
pub mod state;
pub mod status_bar;
pub mod title_bar;
pub mod window_shape;

pub const DEFAULT_VIDEO: VideoStandard = VideoStandard::Pal;

// NTSC is the PAL screen with the bottom cut off, so the footer goes first
pub const fn screen(video: VideoStandard) -> Screen {
    return match video {
        VideoStandard::Pal => Screen::PAL_HIRES,
        VideoStandard::Ntsc => Screen::NTSC_HIRES,
    };
}
