use crate::{
    event::{MouseButton, MouseEvent, PointerEvent},
    rect::{Point, Rect},
};

/// What a [`Clickable`] made of an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    Ignored,
    // the release after a click
    Handled,
    Clicked { button: MouseButton, pos: Point },
}

impl Click {
    /// Whether the event was used, so nothing else should get it.
    pub fn handled(self) -> bool {
        return self != Click::Ignored;
    }
}

/// Tracks presses on one area and turns them into clicks.
// clicks on press like Intuition's GACT_IMMEDIATE gadgets. the release gets eaten wherever it lands
// so whatever's under the pointer doesn't see half a click
#[derive(Clone, Debug, Default)]
pub struct Clickable {
    area: Rect,
    hovered: bool,
    pressed: Option<MouseButton>,
}

impl Clickable {
    /// Sets the area clicks land in, call it from `render`.
    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
    }

    /// Forgets any press, like when its gadget gets disabled while held.
    pub fn reset(&mut self) {
        self.pressed = None;
        self.hovered = false;
    }

    /// Whether a mouse button is held on it with the pointer still over it.
    pub fn is_pressed(&self) -> bool {
        return self.hovered && self.pressed.is_some();
    }

    /// Feeds it an event and says what came of it.
    // moves are never taken, every clickable needs them for hover
    pub fn handle(&mut self, event: &impl PointerEvent) -> Click {
        let Some(event) = event.mouse() else {
            return Click::Ignored;
        };
        match event {
            MouseEvent::Move { pos } => {
                self.hovered = self.area.contains(pos);
                return Click::Ignored;
            }
            MouseEvent::Down { pos, button } => {
                self.hovered = self.area.contains(pos);
                // a press of the button held means its release went to another widget
                let other_held = self.pressed.is_some_and(|held| held != button);
                if !self.hovered || other_held {
                    return Click::Ignored;
                }
                self.pressed = Some(button);
                return Click::Clicked { button, pos };
            }
            MouseEvent::Up { pos, button } => {
                if self.pressed != Some(button) {
                    return Click::Ignored;
                }
                self.pressed = None;
                self.hovered = self.area.contains(pos);
                return Click::Handled;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clickable() -> Clickable {
        let mut clickable = Clickable::default();
        clickable.set_area(Rect::new(10, 10, 20, 10));
        return clickable;
    }

    fn down(x: i32, y: i32) -> MouseEvent {
        return MouseEvent::Down {
            pos: Point::new(x, y),
            button: MouseButton::Left,
        };
    }

    fn up(x: i32, y: i32) -> MouseEvent {
        return MouseEvent::Up {
            pos: Point::new(x, y),
            button: MouseButton::Left,
        };
    }

    #[test]
    fn press_clicks() {
        let mut clickable = clickable();
        assert_eq!(
            clickable.handle(&down(15, 15)),
            Click::Clicked {
                button: MouseButton::Left,
                pos: Point::new(15, 15),
            }
        );
        assert!(clickable.is_pressed());
        assert_eq!(clickable.handle(&up(16, 16)), Click::Handled);
        assert!(!clickable.is_pressed());
    }

    #[test]
    fn release_anywhere() {
        let mut clickable = clickable();
        clickable.handle(&down(15, 15));
        assert_eq!(clickable.handle(&up(100, 100)), Click::Handled);
    }

    #[test]
    fn pressed_while_hovered() {
        let mut clickable = clickable();
        clickable.handle(&down(15, 15));
        clickable.handle(&MouseEvent::Move {
            pos: Point::new(100, 0),
        });
        assert!(!clickable.is_pressed());
        clickable.handle(&MouseEvent::Move {
            pos: Point::new(15, 15),
        });
        assert!(clickable.is_pressed());
    }

    #[test]
    fn missed_release() {
        let mut clickable = clickable();
        clickable.handle(&down(15, 15));
        assert!(clickable.handle(&down(15, 15)) != Click::Ignored);
        let right = MouseEvent::Down {
            pos: Point::new(15, 15),
            button: MouseButton::Right,
        };
        assert_eq!(clickable.handle(&right), Click::Ignored);
    }

    #[test]
    fn outside_ignored() {
        let mut clickable = clickable();
        assert_eq!(clickable.handle(&down(100, 100)), Click::Ignored);
        assert_eq!(clickable.handle(&up(15, 15)), Click::Ignored);
    }
}
