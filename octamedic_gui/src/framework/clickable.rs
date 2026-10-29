use crate::framework::{
    event::{Event, MouseButton},
    rect::{Point, Rect},
};

/// What a `Clickable` made of an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    Ignored,
    /// Taken, but not a click yet: a press, or a release away from where it was pressed.
    Handled,
    /// Released at `pos`, over the area.
    Clicked {
        button: MouseButton,
        pos: Point,
    },
}

impl Click {
    pub fn handled(self) -> bool {
        return self != Click::Ignored;
    }
}

/// An area that clicks like an Amiga gadget: pressed with the pointer over it, and clicked when
/// released over it again. Moving off and back on while held is fine.
#[derive(Debug, Default)]
pub struct Clickable {
    area: Rect,
    hovered: bool,
    pressed: Option<MouseButton>,
}

impl Clickable {
    /// Set while rendering, which is where the area is worked out.
    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
    }

    /// Held down with the pointer over it.
    pub fn is_pressed(&self) -> bool {
        return self.hovered && self.pressed.is_some();
    }

    /// Mouse moves are never taken, so every clickable sees them.
    pub fn handle(&mut self, event: &Event) -> Click {
        match *event {
            Event::MouseMove { pos } => {
                self.hovered = self.area.contains(pos);
                return Click::Ignored;
            }
            Event::MouseDown { pos, button } => {
                self.hovered = self.area.contains(pos);
                if !self.hovered || self.pressed.is_some() {
                    return Click::Ignored;
                }
                self.pressed = Some(button);
                return Click::Handled;
            }
            Event::MouseUp { pos, button } => {
                if self.pressed != Some(button) {
                    return Click::Ignored;
                }
                self.pressed = None;
                self.hovered = self.area.contains(pos);
                return if self.hovered {
                    Click::Clicked { button, pos }
                } else {
                    Click::Handled
                };
            }
            _ => return Click::Ignored,
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

    fn down(x: i32, y: i32) -> Event {
        return Event::MouseDown {
            pos: [x, y],
            button: MouseButton::Left,
        };
    }

    fn up(x: i32, y: i32) -> Event {
        return Event::MouseUp {
            pos: [x, y],
            button: MouseButton::Left,
        };
    }

    #[test]
    fn releasing_where_it_was_pressed_clicks() {
        let mut clickable = clickable();
        assert_eq!(clickable.handle(&down(15, 15)), Click::Handled);
        assert!(clickable.is_pressed());
        assert_eq!(
            clickable.handle(&up(16, 16)),
            Click::Clicked {
                button: MouseButton::Left,
                pos: [16, 16],
            }
        );
        assert!(!clickable.is_pressed());
    }

    #[test]
    fn releasing_elsewhere_takes_the_release_without_clicking() {
        let mut clickable = clickable();
        clickable.handle(&down(15, 15));
        assert_eq!(clickable.handle(&up(100, 100)), Click::Handled);
    }

    #[test]
    fn it_only_looks_pressed_while_the_pointer_is_over_it() {
        let mut clickable = clickable();
        clickable.handle(&down(15, 15));
        clickable.handle(&Event::MouseMove { pos: [100, 0] });
        assert!(!clickable.is_pressed());
        clickable.handle(&Event::MouseMove { pos: [15, 15] });
        assert!(clickable.is_pressed());
    }

    #[test]
    fn presses_and_releases_elsewhere_are_ignored() {
        let mut clickable = clickable();
        assert_eq!(clickable.handle(&down(100, 100)), Click::Ignored);
        assert_eq!(clickable.handle(&up(15, 15)), Click::Ignored);
    }
}
