use ggez::input::keyboard::{KeyCode, KeyMods};
pub use rustuition::event::MouseButton;
use rustuition::{
    event::{MouseEvent, PointerEvent},
    rect::Point,
};

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

impl PointerEvent for Event {
    fn mouse(&self) -> Option<MouseEvent> {
        return match *self {
            Event::MouseMove { pos } => Some(MouseEvent::Move { pos }),
            Event::MouseDown { pos, button } => Some(MouseEvent::Down { pos, button }),
            Event::MouseUp { pos, button } => Some(MouseEvent::Up { pos, button }),
            Event::Key { .. } | Event::Text { .. } => None,
        };
    }
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
