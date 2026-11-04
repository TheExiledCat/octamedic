use rustuition::{gadget::Gadget, gadgets::slider::SliderDirection, painter::Painter, rect::Rect};

use crate::{
    app::state::AppState,
    framework::{event::Event, widgets::SteppedSlider},
};

pub const SCROLLER_WIDTH: i32 = 20;
const ARROW_HEIGHT: i32 = 10;
const ARROW: [i32; 2] = [12, 6];
const MIN_HANDLE: i32 = 4;

/// A list's scroller: a slider with up and down arrows under it, keeping track of the first entry
/// shown.
pub struct ListScroller {
    slider: SteppedSlider,
    pub first_visible_entry: usize,
    // set by whoever draws the list
    pub lines_that_fit: usize,
}

impl ListScroller {
    pub fn new() -> Self {
        return Self {
            slider: SteppedSlider::new(SliderDirection::Vertical, 0, 0, ARROW, ARROW_HEIGHT),
            first_visible_entry: 0,
            lines_that_fit: 0,
        };
    }

    pub fn update(&mut self, state: &mut AppState, total: usize) {
        let last = total
            .saturating_sub(self.lines_that_fit)
            .min(u16::MAX as usize);
        self.first_visible_entry = self.first_visible_entry.min(last);
        self.slider.set_range(0, last as u16);
        if let Some(first) = self.slider.update(state, self.first_visible_entry as u16) {
            self.first_visible_entry = first as usize;
        }
    }

    pub fn render(&mut self, state: &AppState, area: Rect, total: usize, painter: &mut Painter) {
        let track = area.h - 2 * ARROW_HEIGHT - 2;
        let handle = if total > self.lines_that_fit {
            (track * self.lines_that_fit as i32 / total as i32).max(MIN_HANDLE)
        } else {
            track
        };
        self.slider.set_handle(Some(handle));
        self.slider.render(state, area, painter);
    }

    pub fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.slider.handle_event(state, event);
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::{rect::Point, target::Framebuffer};

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
        scroller.slider.slider_mut().set_value(value);
    }

    #[test]
    fn dragging_scrolls() {
        let mut state = state();
        let mut scroller = scroller_fitting(13);
        scroller.update(&mut state, 20);
        assert_eq!(
            (
                scroller.first_visible_entry,
                scroller.slider.slider_mut().value()
            ),
            (0, 0)
        );
        drag_to(&mut scroller, 5);
        scroller.update(&mut state, 20);
        assert_eq!(scroller.first_visible_entry, 5);
        drag_to(&mut scroller, 7);
        scroller.update(&mut state, 20);
        assert_eq!(scroller.first_visible_entry, 7);
    }

    #[test]
    fn scrolling_stops_at_the_ends() {
        let mut state = state();
        let mut scroller = scroller_fitting(13);
        scroller.first_visible_entry = 100;
        scroller.update(&mut state, 20);
        assert_eq!(scroller.first_visible_entry, 7);
        scroller.update(&mut state, 5);
        assert_eq!(scroller.first_visible_entry, 0);
        assert_eq!(scroller.slider.slider_mut().value(), 0);
    }

    #[test]
    fn arrows_scroll_a_line() {
        let mut state = state();
        let mut scroller = scroller_fitting(13);
        let area = Rect::new(0, 0, SCROLLER_WIDTH, 106);
        let mut screen = Framebuffer::new(20, 106);
        scroller.update(&mut state, 20);
        let mut painter = Painter::new(&mut screen, &state.palette);
        scroller.render(&state, area, 20, &mut painter);
        let press = |y| Event::MouseDown {
            pos: Point::new(5, y),
            button: MouseButton::Left,
        };
        let down = area.bottom() - ARROW_HEIGHT / 2;
        let up = down - ARROW_HEIGHT;
        for _ in 0..3 {
            scroller.handle_event(&mut state, &press(down));
            scroller.update(&mut state, 20);
        }
        assert_eq!(scroller.first_visible_entry, 3);
        scroller.handle_event(&mut state, &press(up));
        scroller.update(&mut state, 20);
        assert_eq!(scroller.first_visible_entry, 2);
    }
}
