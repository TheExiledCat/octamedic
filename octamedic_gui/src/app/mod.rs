use rustuition::screen::Screen;

pub mod app;
pub mod copyright_bar;
pub mod cursor;
pub mod debug_console;
pub mod files;
pub mod fonts;
pub mod footer;
pub mod header;
pub mod keybinds;
pub mod main;
pub mod main_view;
pub mod mode;
pub mod mode_buttons;
pub mod pens;
pub mod state;
pub mod status_bar;
pub mod title_bar;

pub const SCREEN: Screen = Screen::PAL_HIRES;
