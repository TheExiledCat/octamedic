// moving into the rustuition crate, re-exported here until the app imports it directly
pub use rustuition::{rect, shape};

pub mod clickable;
pub mod event;
pub mod font;
pub mod keymap;
pub mod painter;
pub mod screen;
pub mod target;
pub mod text;
pub mod widget;
pub mod widgets;
