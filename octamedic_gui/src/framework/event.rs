use ggez::input::keyboard::{KeyCode, KeyMods};

use crate::framework::rect::Point;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
}

/// Input, with mouse positions in screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    MouseMove {
        pos: Point,
    },
    MouseDown {
        pos: Point,
        button: MouseButton,
    },
    MouseUp {
        pos: Point,
        button: MouseButton,
    },
    Key {
        key: KeyCode,
        mods: KeyMods,
        repeat: bool,
    },
    Text(char),
}
