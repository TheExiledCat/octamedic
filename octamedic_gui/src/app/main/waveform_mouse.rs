use ggez::input::keyboard::KeyMods;
use rustuition::rect::{Point, Rect};

use crate::framework::event::{Event, MouseButton};

/// The mouse on the sample editor's waveform. A press there is followed until its button's let go,
/// wherever the mouse goes in between, so a drag can carry on past the waveform's edges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WaveformMouse {
    // where the waveform was last drawn, as events come in screen positions
    area: Rect,
    held: Option<MouseButton>,
    pressed: Option<MouseButton>,
    released: Option<MouseButton>,
    shifted: bool,
    column: i32,
    press_column: i32,
}

impl WaveformMouse {
    /// Call when drawing the waveform at `area`.
    pub fn place(&mut self, area: Rect) {
        self.area = area;
    }

    /// Whether `event` was the waveform's: a press on it, or anything the mouse does while that
    /// press is held. `mods` are the modifier keys down at the time.
    pub fn handle_event(&mut self, event: &Event, mods: KeyMods) -> bool {
        match *event {
            Event::MouseDown { pos, button } if self.held.is_none() && self.area.contains(pos) => {
                self.held = Some(button);
                self.pressed = Some(button);
                self.shifted = mods.contains(KeyMods::SHIFT);
                self.follow(pos);
                self.press_column = self.column;
                return true;
            }
            Event::MouseMove { pos } if self.held.is_some() => {
                self.follow(pos);
                return false;
            }
            Event::MouseUp { pos, button } if self.held == Some(button) => {
                self.held = None;
                self.released = Some(button);
                self.follow(pos);
                return true;
            }
            _ => return false,
        }
    }

    /// The button pressed on the waveform since this was last asked.
    pub fn take_press(&mut self) -> Option<MouseButton> {
        return self.pressed.take();
    }

    /// The button let go of since this was last asked, after being pressed on the waveform.
    pub fn take_release(&mut self) -> Option<MouseButton> {
        return self.released.take();
    }

    /// Whether shift was down for the press that's held, or the last one.
    pub fn shifted(&self) -> bool {
        return self.shifted;
    }

    /// The button that was pressed on the waveform, while it's still down.
    pub fn held(&self) -> Option<MouseButton> {
        return self.held;
    }

    /// The waveform's column under the mouse, from 0 at its left, as of the last press, release
    /// or move while held. Kept inside the waveform when the mouse leaves it.
    pub fn column(&self) -> i32 {
        return self.column;
    }

    /// The column the press that's held, or the last one, was on, as the mouse can have moved on
    /// by the time the press is looked at.
    pub fn press_column(&self) -> i32 {
        return self.press_column;
    }

    fn follow(&mut self, [x, _]: Point) {
        self.column = (x - self.area.x).clamp(0, (self.area.w - 1).max(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mouse() -> WaveformMouse {
        let mut mouse = WaveformMouse::default();
        mouse.place(Rect::new(10, 20, 640, 64));
        return mouse;
    }

    #[test]
    fn a_press_is_followed_until_its_let_go() {
        let mut mouse = mouse();
        let left = MouseButton::Left;
        assert!(mouse.handle_event(
            &Event::MouseDown {
                pos: [110, 30],
                button: left
            },
            KeyMods::NONE
        ));
        assert_eq!((mouse.take_press(), mouse.take_press()), (Some(left), None));
        assert_eq!((mouse.held(), mouse.column()), (Some(left), 100));
        // past either edge it stays on the waveform
        mouse.handle_event(&Event::MouseMove { pos: [2000, 500] }, KeyMods::NONE);
        assert_eq!((mouse.column(), mouse.press_column()), (639, 100));
        mouse.handle_event(&Event::MouseMove { pos: [0, 30] }, KeyMods::NONE);
        assert_eq!(mouse.column(), 0);
        // only the button that pressed lets go
        let right = MouseButton::Right;
        let up = |button| Event::MouseUp {
            pos: [50, 30],
            button,
        };
        assert!(!mouse.handle_event(&up(right), KeyMods::NONE));
        assert!(mouse.handle_event(&up(left), KeyMods::NONE));
        assert_eq!((mouse.held(), mouse.take_release()), (None, Some(left)));
        assert_eq!(mouse.column(), 40);
    }

    #[test]
    fn presses_elsewhere_arent_its() {
        let mut mouse = mouse();
        let down = Event::MouseDown {
            pos: [110, 200],
            button: MouseButton::Left,
        };
        assert!(!mouse.handle_event(&down, KeyMods::NONE));
        assert!(!mouse.handle_event(&Event::MouseMove { pos: [110, 30] }, KeyMods::NONE));
        assert_eq!((mouse.held(), mouse.take_press()), (None, None));
    }

    #[test]
    fn remembers_shift_for_the_press() {
        let mut mouse = mouse();
        let down = Event::MouseDown {
            pos: [110, 30],
            button: MouseButton::Left,
        };
        mouse.handle_event(&down, KeyMods::SHIFT);
        assert!(mouse.shifted());
        // letting go of shift first doesn't change what the press was
        mouse.handle_event(&Event::MouseMove { pos: [120, 30] }, KeyMods::NONE);
        assert!(mouse.shifted());
    }
}
