use ggez::input::keyboard::{KeyCode, KeyMods};
pub use rustuition::event::MouseButton;
use rustuition::{
    event::{EditKey, MouseEvent, PointerEvent, TextEvent, TextInput},
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
    /// The mouse wheel or a trackpad, in whole lines, positive towards the end of a list.
    Wheel {
        pos: Point,
        lines: i32,
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

impl Event {
    pub fn button(&self) -> Option<MouseButton> {
        return match *self {
            Event::MouseDown { button, .. } | Event::MouseUp { button, .. } => Some(button),
            _ => None,
        };
    }
}

impl PointerEvent for Event {
    fn mouse(&self) -> Option<MouseEvent> {
        return match *self {
            Event::MouseMove { pos } => Some(MouseEvent::Move { pos }),
            Event::MouseDown { pos, button } => Some(MouseEvent::Down { pos, button }),
            Event::MouseUp { pos, button } => Some(MouseEvent::Up { pos, button }),
            Event::Wheel { .. } | Event::Key { .. } | Event::Text { .. } => None,
        };
    }
}

impl TextEvent for Event {
    fn text_input(&self) -> Option<TextInput> {
        let shortcut = KeyMods::CTRL | KeyMods::ALT | KeyMods::LOGO;
        return match *self {
            Event::Text {
                character, mods, ..
            } if !mods.intersects(shortcut) => Some(TextInput::Char(character)),
            Event::Key { key, mods, .. } => {
                let ctrl = mods.contains(KeyMods::CTRL);
                let key = match key {
                    KeyCode::Left => EditKey::Left,
                    KeyCode::Right => EditKey::Right,
                    KeyCode::Home => EditKey::Home,
                    KeyCode::End => EditKey::End,
                    KeyCode::Back => EditKey::Backspace,
                    KeyCode::Delete => EditKey::Delete,
                    KeyCode::Return | KeyCode::NumpadEnter => EditKey::Enter,
                    KeyCode::Escape => EditKey::Escape,
                    KeyCode::A if ctrl => EditKey::SelectAll,
                    _ => return None,
                };
                let shift = mods.contains(KeyMods::SHIFT);
                Some(TextInput::Key { key, shift, ctrl })
            }
            _ => None,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: KeyCode, mods: KeyMods) -> Option<TextInput> {
        let event = Event::Key {
            key,
            mods,
            repeat: false,
        };
        return event.text_input();
    }

    #[test]
    fn keys_to_text_input() {
        assert_eq!(Event::text('a').text_input(), Some(TextInput::Char('a')));
        let shortcut = Event::Text {
            character: 'a',
            mods: KeyMods::CTRL,
            repeat: false,
        };
        assert_eq!(shortcut.text_input(), None);
        assert_eq!(
            key(KeyCode::Left, KeyMods::SHIFT | KeyMods::CTRL),
            Some(TextInput::Key {
                key: EditKey::Left,
                shift: true,
                ctrl: true
            })
        );
        assert_eq!(
            key(KeyCode::A, KeyMods::CTRL),
            Some(TextInput::Key {
                key: EditKey::SelectAll,
                shift: false,
                ctrl: true
            })
        );
        assert_eq!(key(KeyCode::A, KeyMods::NONE), None);
        assert_eq!(key(KeyCode::Up, KeyMods::NONE), None);
    }
}
