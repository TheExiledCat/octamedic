use crate::framework::{
    event::{Event, MouseButton},
    rect::{Point, Rect},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    Ignored,
    /// The release of a press that clicked.
    Handled,
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

/// Clicks on press, like OctaMED's gadgets. The release is taken too, wherever it lands, so
/// nothing under the pointer sees half a click.
#[derive(Clone, Debug, Default)]
pub struct Clickable {
    area: Rect,
    hovered: bool,
    pressed: Option<MouseButton>,
}

impl Clickable {
    pub fn set_area(&mut self, area: Rect) {
        self.area = area;
    }

    // nothing draws itself pressed at the moment
    #[allow(dead_code)]
    pub fn is_pressed(&self) -> bool {
        return self.hovered && self.pressed.is_some();
    }

    // Mouse moves are never taken, so every clickable sees them.
    pub fn handle(&mut self, event: &Event) -> Click {
        match *event {
            Event::MouseMove { pos } => {
                self.hovered = self.area.contains(pos);
                return Click::Ignored;
            }
            Event::MouseDown { pos, button } => {
                self.hovered = self.area.contains(pos);
                // a press of the button held means its release went to another widget
                let other_held = self.pressed.is_some_and(|held| held != button);
                if !self.hovered || other_held {
                    return Click::Ignored;
                }
                self.pressed = Some(button);
                return Click::Clicked { button, pos };
            }
            Event::MouseUp { pos, button } => {
                if self.pressed != Some(button) {
                    return Click::Ignored;
                }
                self.pressed = None;
                self.hovered = self.area.contains(pos);
                return Click::Handled;
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
    fn pressing_clicks_at_once() {
        let mut clickable = clickable();
        assert_eq!(
            clickable.handle(&down(15, 15)),
            Click::Clicked {
                button: MouseButton::Left,
                pos: [15, 15],
            }
        );
        assert!(clickable.is_pressed());
        assert_eq!(clickable.handle(&up(16, 16)), Click::Handled);
        assert!(!clickable.is_pressed());
    }

    #[test]
    fn the_release_is_taken_wherever_it_is() {
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
    fn a_press_whose_release_was_missed_still_clicks_next_time() {
        let mut clickable = clickable();
        clickable.handle(&down(15, 15));
        assert!(clickable.handle(&down(15, 15)) != Click::Ignored);
        let right = Event::MouseDown {
            pos: [15, 15],
            button: MouseButton::Right,
        };
        assert_eq!(clickable.handle(&right), Click::Ignored);
    }

    #[test]
    fn presses_and_releases_elsewhere_are_ignored() {
        let mut clickable = clickable();
        assert_eq!(clickable.handle(&down(100, 100)), Click::Ignored);
        assert_eq!(clickable.handle(&up(15, 15)), Click::Ignored);
    }
}
