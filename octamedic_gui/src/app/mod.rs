use octamedic_core::amiga::clock::VideoStandard;
use rustuition::screen::Screen;

pub mod app;
pub mod browser;
pub mod copyright_bar;
pub mod cursor;
pub mod debug_console;
pub mod files;
pub mod fonts;
pub mod footer;
pub mod header;
pub mod keybinds;
pub mod load_instrument;
pub mod main;
pub mod main_view;
pub mod mode;
pub mod mode_buttons;
pub mod pens;
pub mod state;
pub mod status_bar;
pub mod title_bar;

pub const DEFAULT_VIDEO: VideoStandard = VideoStandard::Pal;

// NTSC is the PAL screen with the bottom cut off, so the footer goes first
pub const fn screen(video: VideoStandard) -> Screen {
    return match video {
        VideoStandard::Pal => Screen::PAL_HIRES,
        VideoStandard::Ntsc => Screen::NTSC_HIRES,
    };
}
