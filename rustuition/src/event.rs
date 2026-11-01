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
