use rustuition::gadgets::button::ButtonClick;

use super::Button;

/// A row of buttons where at most one is picked at a time, like the Files panel's save-format
/// row or Misc's danger-zone buttons. Call [`Self::take_click`], let the caller decide what the
/// click means for its own selection state, then [`Self::sync`] so this frame's toggled look
/// already reflects that decision.
pub struct ToggleGroup<'a, T> {
    buttons: Vec<(&'a mut Button, T)>,
}

impl<'a, T: Copy + PartialEq> ToggleGroup<'a, T> {
    /// Groups `buttons`, each tagged with the value it picks.
    pub fn new(buttons: impl IntoIterator<Item = (&'a mut Button, T)>) -> Self {
        return Self {
            buttons: buttons.into_iter().collect(),
        };
    }

    /// Takes whichever button in the group was clicked, if any.
    pub fn take_click(&mut self) -> Option<(T, ButtonClick)> {
        let mut clicked = None;
        for (button, value) in &mut self.buttons {
            if let Some(click) = button.clicked() {
                clicked = Some((*value, click));
            }
        }
        return clicked;
    }

    /// Sets every button's toggled look to whether it's `selected`.
    pub fn sync(&mut self, selected: Option<T>) {
        for (button, value) in &mut self.buttons {
            button.set_toggled(Some(*value) == selected);
        }
    }
}

#[cfg(test)]
mod tests {
    use rustuition::{
        event::MouseButton,
        gadget::Gadget,
        painter::Painter,
        rect::{Point, Rect},
        target::Framebuffer,
    };

    use super::*;
    use crate::{app::state::AppState, framework::event::Event};

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Choice {
        A,
        B,
    }

    fn press_and_release(button: &mut Button, state: &mut AppState, pos: Point) {
        let button_event = MouseButton::Left;
        button.handle_event(
            state,
            &Event::MouseDown {
                pos,
                button: button_event,
            },
        );
        button.handle_event(
            state,
            &Event::MouseUp {
                pos,
                button: button_event,
            },
        );
    }

    #[test]
    fn reports_the_click_and_syncs_toggles() {
        use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut a = Button::new("A");
        let mut b = Button::new("B");
        let mut screen = Framebuffer::new(40, 10);
        let mut painter = Painter::new(&mut screen, &state.palette);
        a.render(&state, Rect::new(0, 0, 20, 10), &mut painter);
        b.render(&state, Rect::new(20, 0, 20, 10), &mut painter);
        press_and_release(&mut b, &mut state, Point::new(25, 5));
        let mut group = ToggleGroup::new([(&mut a, Choice::A), (&mut b, Choice::B)]);
        let clicked = group.take_click();
        assert_eq!(clicked.map(|(value, _)| value), Some(Choice::B));
        // the caller decides what the click means, here just picking what was clicked
        group.sync(clicked.map(|(value, _)| value));
        assert!(!a.is_toggled());
        assert!(b.is_toggled());
    }

    #[test]
    fn nothing_clicked_still_syncs_toggles() {
        let mut a = Button::new("A");
        let mut b = Button::new("B");
        let mut group = ToggleGroup::new([(&mut a, Choice::A), (&mut b, Choice::B)]);
        assert_eq!(group.take_click(), None);
        group.sync(Some(Choice::B));
        assert!(!a.is_toggled());
        assert!(b.is_toggled());
    }
}
