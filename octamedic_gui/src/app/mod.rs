use octamedic_core::amiga::clock::VideoStandard;
use rustuition::screen::Screen;

pub mod advance;
pub mod app;
pub mod audio_devices;
pub mod block_range;
pub mod chord;
pub mod clipboard;
pub mod config;
pub mod copyright_bar;
pub mod cursor;
pub mod debug_console;
pub mod files;
pub mod fonts;
pub mod footer;
pub mod header;
pub mod history;
pub mod icon;
pub mod keybinds;
pub mod keypad;
pub mod main;
pub mod main_view;
pub mod mode;
pub mod pens;
pub mod prompt;
pub mod screen_filters;
pub mod state;
pub mod status_bar;
pub mod title_bar;
pub mod transpose;
pub mod window_shape;

pub const DEFAULT_VIDEO: VideoStandard = VideoStandard::Pal;

// NTSC is the PAL screen with the bottom cut off, so the footer goes first
pub const fn screen(video: VideoStandard) -> Screen {
    return match video {
        VideoStandard::Pal => Screen::PAL_HIRES,
        VideoStandard::Ntsc => Screen::NTSC_HIRES,
    };
}
