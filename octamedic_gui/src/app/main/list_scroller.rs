use rustuition::{gadget::Gadget, gadgets::slider::SliderDirection, painter::Painter, rect::Rect};

use crate::{
    app::state::AppState,
    framework::{event::Event, wheel::WheelLines, widgets::SteppedSlider},
};

pub const SCROLLER_WIDTH: i32 = 20;
const ARROW_HEIGHT: i32 = 10;
const ARROW: [i32; 2] = [12, 6];
const MIN_HANDLE: i32 = 4;

/// A list's scroller: a slider with up and down arrows under it, keeping track of the first entry
/// shown. The wheel scrolls it too, over the list or the scroller.
pub struct ListScroller {
    slider: SteppedSlider,
    pub first_visible_entry: usize,
    // set by whoever draws the list
    pub lines_that_fit: usize,
    pub list_area: Rect,
    // from the last render
    area: Rect,
    // from the last update
    last_first_entry: usize,
    wheel: WheelLines,
}

impl ListScroller {
    pub fn new() -> Self {
        return Self {
            slider: SteppedSlider::new(SliderDirection::Vertical, 0, 0, ARROW, ARROW_HEIGHT),
            first_visible_entry: 0,
            lines_that_fit: 0,
            list_area: Rect::default(),
            area: Rect::default(),
            last_first_entry: 0,
            wheel: WheelLines::default(),
        };
    }

    pub fn update(&mut self, state: &mut AppState, total: usize) {
        let last = total
            .saturating_sub(self.lines_that_fit)
            .min(u16::MAX as usize);
        self.last_first_entry = last;
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
        self.area = area;
    }

    pub fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        if let Event::Wheel(wheel) = *event {
            let over = self.list_area.contains(wheel.pos) || self.area.contains(wheel.pos);
            if over {
                let lines = self.wheel.lines(wheel.delta[1]);
                self.scroll(lines);
            }
            return over;
        }
        return self.slider.handle_event(state, event);
    }

    fn scroll(&mut self, lines: i32) {
        let first = self
            .first_visible_entry
            .saturating_add_signed(lines as isize);
        self.first_visible_entry = first.min(self.last_first_entry);
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::{rect::Point, target::Framebuffer};

    use super::*;
    use crate::framework::event::{MouseButton, Wheel};

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

    const LIST: Rect = Rect::new(0, 0, 100, 106);
    const SCROLLER: Rect = Rect::new(100, 0, SCROLLER_WIDTH, 106);

    // 20 entries, 13 fit, so the first can go from 0 to 7
    fn drawn_scroller(state: &mut AppState) -> ListScroller {
        let mut scroller = scroller_fitting(13);
        scroller.list_area = LIST;
        scroller.update(state, 20);
        let mut screen = Framebuffer::new(SCROLLER.right(), SCROLLER.h);
        let mut painter = Painter::new(&mut screen, &state.palette);
        scroller.render(state, SCROLLER, 20, &mut painter);
        return scroller;
    }

    // a notch of a mouse wheel, towards you and away
    const DOWN: f32 = -1.0;
    const UP: f32 = 1.0;

    fn wheel(x: i32, y: f32) -> Event {
        return Event::Wheel(Wheel {
            pos: Point::new(x, 50),
            delta: [0.0, y],
        });
    }

    #[test]
    fn wheel_over_the_list_scrolls() {
        let mut state = state();
        let mut scroller = drawn_scroller(&mut state);
        for _ in 0..3 {
            assert!(scroller.handle_event(&mut state, &wheel(50, DOWN)));
        }
        assert!(scroller.handle_event(&mut state, &wheel(50, UP)));
        scroller.update(&mut state, 20);
        assert_eq!(scroller.first_visible_entry, 2);
        assert_eq!(scroller.slider.slider_mut().value(), 2);
    }

    #[test]
    fn wheel_over_the_scroller_scrolls() {
        let mut state = state();
        let mut scroller = drawn_scroller(&mut state);
        assert!(scroller.handle_event(&mut state, &wheel(SCROLLER.x + 5, DOWN)));
        assert_eq!(scroller.first_visible_entry, 1);
    }

    #[test]
    fn trackpad_deltas_add_up_to_a_line_at_most() {
        let mut state = state();
        let mut scroller = drawn_scroller(&mut state);
        let mut first_after = |y| {
            scroller.handle_event(&mut state, &wheel(50, y));
            return scroller.first_visible_entry;
        };
        assert_eq!(first_after(-0.5), 0);
        assert_eq!(first_after(-0.5), 1);
        assert_eq!(first_after(-30.0), 2);
    }

    #[test]
    fn wheel_stops_at_the_ends() {
        let mut state = state();
        let mut scroller = drawn_scroller(&mut state);
        scroller.handle_event(&mut state, &wheel(50, UP));
        assert_eq!(scroller.first_visible_entry, 0);
        for _ in 0..20 {
            scroller.handle_event(&mut state, &wheel(50, DOWN));
        }
        assert_eq!(scroller.first_visible_entry, 7);
    }

    #[test]
    fn wheel_elsewhere_is_left_for_others() {
        let mut state = state();
        let mut scroller = drawn_scroller(&mut state);
        assert!(!scroller.handle_event(&mut state, &wheel(SCROLLER.right() + 5, DOWN)));
        assert_eq!(scroller.first_visible_entry, 0);
    }
}
