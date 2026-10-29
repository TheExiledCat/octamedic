use crate::{
    app::state::AppState,
    framework::{
        clickable::{Click, Clickable},
        event::{Event, MouseButton},
        painter::{CHAR_WIDTH, Painter, Pen, centered_text_x, inside_frame},
        rect::{Point, Rect},
        widget::OctamedicWidget,
    },
};

/// What a button's click handler is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonClick {
    pub button: MouseButton,
    /// The label's character under the pointer, counting from 0, if it was over the label.
    pub char_index: Option<usize>,
}

type ClickHandler = Box<dyn FnMut(&mut AppState, ButtonClick)>;

/// A text gadget, which changes the state when clicked.
pub struct Button {
    text: String,
    clickable: Clickable,
    on_click: Option<ClickHandler>,
    /// Drawn inverted, like OctaMED's buttons that are switched on.
    toggled: bool,
    /// Pixels between the frame and the label, on every side.
    padding: i32,
    /// Where the label started when last rendered.
    label_x: i32,
}

impl Button {
    pub fn new(text: impl Into<String>) -> Self {
        return Self {
            text: text.into(),
            clickable: Clickable::default(),
            on_click: None,
            toggled: false,
            padding: 1,
            label_x: 0,
        };
    }

    pub fn on_click(mut self, handler: impl FnMut(&mut AppState, ButtonClick) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        return self;
    }

    pub fn padding(mut self, padding: i32) -> Self {
        self.padding = padding;
        return self;
    }

    /// Where the label is centred: inside the frame and the padding. When that's shorter than the
    /// text, the text starts at its top.
    fn content(&self, area: Rect) -> Rect {
        return inside_frame(area).inset(self.padding, self.padding);
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    /// Switches between drawn normally and inverted.
    pub fn toggle(&mut self) {
        self.toggled = !self.toggled;
    }

    pub fn set_toggled(&mut self, toggled: bool) {
        self.toggled = toggled;
    }

    pub fn is_toggled(&self) -> bool {
        return self.toggled;
    }

    /// The fill and text pens. Toggled and pressed each swap them, so pressing a toggled button
    /// shows it normally, as a preview of letting go.
    fn pens(&self) -> (Pen, Pen) {
        let (fill, text) = (Pen::FILL, Pen::BLACK);
        return if self.toggled != self.clickable.is_pressed() {
            (text, fill)
        } else {
            (fill, text)
        };
    }

    fn char_at(&self, pos: Point) -> Option<usize> {
        let offset = pos[0] - self.label_x;
        if offset < 0 {
            return None;
        }
        let index = (offset / CHAR_WIDTH) as usize;
        return (index < self.text.chars().count()).then_some(index);
    }
}

impl OctamedicWidget for Button {
    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        let click = self.clickable.handle(event);
        if let Click::Clicked { button, pos } = click {
            let char_index = self.char_at(pos);
            if let Some(handler) = &mut self.on_click {
                handler(state, ButtonClick { button, char_index });
            }
        }
        return click.handled();
    }

    fn render(&mut self, _state: &AppState, area: Rect, painter: &mut Painter) {
        self.clickable.set_area(area);
        let content = self.content(area);
        self.label_x = centered_text_x(content, &self.text);
        let (fill, text) = self.pens();
        painter.fill(area, fill);
        painter.frame(area);
        painter.text_centered(content, &self.text, text);
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    fn click(button: &mut Button, state: &mut AppState, pos: Point) {
        let mouse = MouseButton::Left;
        button.handle_event(state, &Event::MouseDown { pos, button: mouse });
        button.handle_event(state, &Event::MouseUp { pos, button: mouse });
    }

    /// "NEXT" centred in 40 pixels starts at x 4.
    fn next_button(handler: impl FnMut(&mut AppState, ButtonClick) + 'static) -> Button {
        let mut button = Button::new("NEXT").on_click(handler);
        let area = Rect::new(0, 0, 40, 10);
        button.clickable.set_area(area);
        button.label_x = centered_text_x(button.content(area), "NEXT");
        return button;
    }

    #[test]
    fn toggling_and_pressing_each_invert_the_button() {
        let normal = (Pen::FILL, Pen::BLACK);
        let inverted = (Pen::BLACK, Pen::FILL);
        let mut button = next_button(|_, _| {});
        assert_eq!(button.pens(), normal);
        button.toggle();
        assert!(button.is_toggled());
        assert_eq!(button.pens(), inverted);
        let pos = [5, 5];
        let down = Event::MouseDown {
            pos,
            button: MouseButton::Left,
        };
        button.handle_event(&mut state(), &down);
        assert_eq!(button.pens(), normal);
        button.set_toggled(false);
        assert_eq!(button.pens(), inverted);
    }

    #[test]
    fn the_label_sits_inside_the_frame_and_padding() {
        let area = Rect::new(3, 65, 84, 11);
        assert_eq!(Button::new("").content(area), Rect::new(6, 67, 78, 7));
        assert_eq!(
            Button::new("").padding(0).content(area),
            Rect::new(5, 66, 80, 9)
        );
    }

    #[test]
    fn a_click_runs_the_handler_on_the_state() {
        let mut state = state();
        let mut button = next_button(|state, _| state.next_pattern());
        click(&mut button, &mut state, [5, 5]);
        assert_eq!(state.active_pattern.0, 1);
    }

    #[test]
    fn a_click_says_which_character_of_the_label_it_was_on() {
        let clicked = Rc::new(Cell::new(None));
        let seen = clicked.clone();
        let mut button = next_button(move |_, click| seen.set(Some(click.char_index)));
        let mut state = state();
        for (x, index) in [
            (4, Some(0)),
            (13, Some(1)),
            (35, Some(3)),
            (2, None),
            (37, None),
        ] {
            click(&mut button, &mut state, [x, 5]);
            assert_eq!(clicked.get(), Some(index), "x {x}");
        }
    }
}
