use rustuition::{
    gadget::Gadget,
    gadgets::{button::Triangle, slider::SliderDirection},
    painter::Painter,
    rect::Rect,
    shape::Direction::{Down, Up},
};

use crate::{
    app::state::AppState,
    framework::{
        event::Event,
        widgets::{Button, Slider},
    },
};

pub const SCROLLER_WIDTH: i32 = 20;
const ARROW_HEIGHT: i32 = 10;
const ARROW: [i32; 2] = [12, 6];
const MIN_HANDLE: i32 = 4;

/// A list's scroller: a slider with up and down arrows under it, keeping track of the first entry
/// shown.
pub struct ListScroller {
    slider: Slider,
    arrows: [Button; 2],
    pub first_visible_entry: usize,
    // set by whoever draws the list
    pub lines_that_fit: usize,
    // the value last set on the slider, so a drag shows up as a change
    last_slider_value: u16,
}

impl ListScroller {
    pub fn new() -> Self {
        let [width, height] = ARROW;
        return Self {
            slider: Slider::new(SliderDirection::Vertical, false, 1),
            arrows: [Up, Down]
                .map(|direction| Button::new(Triangle::new(direction, width, height))),
            first_visible_entry: 0,
            lines_that_fit: 0,
            last_slider_value: 0,
        };
    }

    pub fn update(&mut self, total: usize) {
        if self.slider.value() != self.last_slider_value {
            self.first_visible_entry = self.slider.value() as usize;
        }
        let [up, down] = &mut self.arrows;
        if up.clicked().is_some() {
            self.first_visible_entry = self.first_visible_entry.saturating_sub(1);
        }
        if down.clicked().is_some() {
            self.first_visible_entry += 1;
        }
        let last = total.saturating_sub(self.lines_that_fit);
        self.first_visible_entry = self.first_visible_entry.min(last);
        self.slider
            .set_steps((last + 1).min(u16::MAX as usize) as u16);
        self.slider.set_value(self.first_visible_entry as u16);
        self.last_slider_value = self.slider.value();
    }

    pub fn render(&mut self, state: &AppState, area: Rect, total: usize, painter: &mut Painter) {
        let (slider, arrows) = area.split_top(area.h - 2 * ARROW_HEIGHT);
        let track = slider.h - 2;
        let handle = if total > self.lines_that_fit {
            (track * self.lines_that_fit as i32 / total as i32).max(MIN_HANDLE)
        } else {
            track
        };
        self.slider.set_handle(Some(handle));
        self.slider.render(state, slider, painter);
        for (arrow, rect) in self.arrows.iter_mut().zip(arrows.split_rows(2)) {
            arrow.render(state, rect, painter);
        }
    }

    pub fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.slider.handle_event(state, event)
            || self
                .arrows
                .iter_mut()
                .any(|arrow| arrow.handle_event(state, event));
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::target::Framebuffer;

    use super::*;
    use crate::framework::event::MouseButton;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    fn scroller_fitting(lines_that_fit: usize) -> ListScroller {
        let mut scroller = ListScroller::new();
        scroller.lines_that_fit = lines_that_fit;
        return scroller;
    }

    fn drag_to(scroller: &mut ListScroller, value: u16) {
        scroller.slider.set_value(value);
    }

    #[test]
    fn dragging_scrolls() {
        let mut scroller = scroller_fitting(13);
        scroller.update(20);
        assert_eq!(
            (scroller.first_visible_entry, scroller.slider.value()),
            (0, 0)
        );
        drag_to(&mut scroller, 5);
        scroller.update(20);
        assert_eq!(scroller.first_visible_entry, 5);
        drag_to(&mut scroller, 7);
        scroller.update(20);
        assert_eq!(scroller.first_visible_entry, 7);
    }

    #[test]
    fn scrolling_stops_at_the_ends() {
        let mut scroller = scroller_fitting(13);
        scroller.first_visible_entry = 100;
        scroller.update(20);
        assert_eq!(scroller.first_visible_entry, 7);
        scroller.update(5);
        assert_eq!(scroller.first_visible_entry, 0);
        assert_eq!(scroller.slider.value(), 0);
    }

    #[test]
    fn arrows_scroll_a_line() {
        let mut state = state();
        let mut scroller = scroller_fitting(13);
        let area = Rect::new(0, 0, SCROLLER_WIDTH, 106);
        let mut screen = Framebuffer::new(20, 106);
        scroller.update(20);
        let mut painter = Painter::new(&mut screen, &state.palette);
        scroller.render(&state, area, 20, &mut painter);
        let press = |y| Event::MouseDown {
            pos: [5, y],
            button: MouseButton::Left,
        };
        let down = area.bottom() - ARROW_HEIGHT / 2;
        let up = down - ARROW_HEIGHT;
        for _ in 0..3 {
            scroller.handle_event(&mut state, &press(down));
            scroller.update(20);
        }
        assert_eq!(scroller.first_visible_entry, 3);
        scroller.handle_event(&mut state, &press(up));
        scroller.update(20);
        assert_eq!(scroller.first_visible_entry, 2);
    }
}
