use ggez::input::keyboard::{KeyCode, KeyMods};

use crate::framework::rect::Point;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
}

// positions are screen pixels, not window pixels
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
    Text {
        character: char,
        mods: KeyMods,
        repeat: bool,
    },
}

#[cfg(test)]
impl Event {
    pub fn text(character: char) -> Self {
        return Event::Text {
            character,
            mods: KeyMods::NONE,
            repeat: false,
        };
    }
}
