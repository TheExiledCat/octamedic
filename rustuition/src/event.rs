use crate::rect::Point;

/// The two Amiga mouse buttons.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
}

/// A mouse move, press or release, in screen pixels rather than window pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseEvent {
    Move { pos: Point },
    Down { pos: Point, button: MouseButton },
    Up { pos: Point, button: MouseButton },
}

/// What gadgets need out of an app's events: just the mouse. Implement it for your own event type
/// to send keys and whatever else through the same gadgets.
pub trait PointerEvent {
    /// The mouse event in it, if it is one.
    fn mouse(&self) -> Option<MouseEvent>;
}

impl PointerEvent for MouseEvent {
    fn mouse(&self) -> Option<MouseEvent> {
        return Some(*self);
    }
}

/// A key that edits text, for [`Textbox`](crate::gadgets::textbox::Textbox).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditKey {
    Left,
    Right,
    Home,
    End,
    Backspace,
    Delete,
    Enter,
    Escape,
    SelectAll,
}

/// Typing: a character, or an editing key. `shift` extends the selection, `ctrl` goes by words.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextInput {
    Char(char),
    Key {
        key: EditKey,
        shift: bool,
        ctrl: bool,
    },
}

/// What text boxes need out of an app's events. Map your own key events to [`TextInput`].
pub trait TextEvent {
    /// The typing in it, if it is any.
    fn text_input(&self) -> Option<TextInput>;
}

impl TextEvent for MouseEvent {
    fn text_input(&self) -> Option<TextInput> {
        return None;
    }
}
