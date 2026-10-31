use std::rc::Rc;

use crate::{
    app::state::AppState,
    framework::{
        event::{Event, MouseButton},
        painter::{Painter, Pen},
        rect::{Point, Rect},
        widget::OctamedicWidget,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderDirection {
    Horizontal,
    Vertical,
}

/// What a slider's handler is told while it's dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slide {
    /// The value when the drag started.
    pub start_value: u16,
    pub value: u16,
}

pub trait SlideHandler: Fn(&mut AppState, Slide) + 'static {}

impl<F: Fn(&mut AppState, Slide) + 'static> SlideHandler for F {}

/// Where the pointer took hold of the handle, and the value it started at.
#[derive(Clone, Copy, Debug)]
struct Drag {
    start_value: u16,
    /// From the handle's start along the slider.
    grab: i32,
}

/// A handle in a dotted track, dragged between `steps` positions. A clone shares the original's
/// handler.
#[derive(Clone)]
pub struct Slider {
    direction: SliderDirection,
    /// The last step at the start of the track: the left, or the top when vertical.
    reversed: bool,
    /// How many positions the handle has; the handle is the track split into this many, unless
    /// `handle` says how long it is.
    steps: u16,
    handle: Option<i32>,
    value: u16,
    on_slide: Option<Rc<dyn SlideHandler>>,
    drag: Option<Drag>,
    /// Where the slider was last rendered.
    area: Rect,
}

impl Slider {
    pub fn new(direction: SliderDirection, reversed: bool, steps: u16) -> Self {
        return Self {
            direction,
            reversed,
            steps: steps.max(1),
            handle: None,
            value: 0,
            on_slide: None,
            drag: None,
            area: Rect::default(),
        };
    }

    /// A handle `length` pixels long whatever the steps, like the play panel's tempo sliders'.
    pub fn fixed_handle(mut self, length: i32) -> Self {
        self.handle = Some(length);
        return self;
    }

    pub fn on_slide(mut self, handler: impl Fn(&mut AppState, Slide) + 'static) -> Self {
        self.set_on_slide(handler);
        return self;
    }

    pub fn set_on_slide(&mut self, handler: impl Fn(&mut AppState, Slide) + 'static) {
        self.on_slide = Some(Rc::new(handler));
    }

    pub fn value(&self) -> u16 {
        return self.value;
    }

    /// A step from 0 to `steps - 1`.
    pub fn set_value(&mut self, value: u16) {
        self.value = value.min(self.steps - 1);
    }

    /// `area` laid flat: `w` along the slider and `h` across it, whichever way it goes.
    fn track(&self, area: Rect) -> Rect {
        return match self.direction {
            SliderDirection::Horizontal => area,
            SliderDirection::Vertical => area.rotate(),
        };
    }

    fn handle_length(&self, track: Rect) -> i32 {
        return self
            .handle
            .unwrap_or(track.w / self.steps as i32)
            .clamp(1, track.w.max(1));
    }

    /// Where the handle starts along the track for `value`.
    fn handle_offset(&self, value: u16, track: Rect) -> i32 {
        let last = self.steps as i32 - 1;
        if last == 0 {
            return 0;
        }
        let step = if self.reversed {
            last - value as i32
        } else {
            value as i32
        };
        return step * (track.w - self.handle_length(track)) / last;
    }

    /// The value whose handle starts nearest `offset` along the track.
    fn value_at(&self, offset: i32, track: Rect) -> u16 {
        let last = self.steps as i32 - 1;
        let room = track.w - self.handle_length(track);
        if last == 0 || room <= 0 {
            return 0;
        }
        let step = ((offset * last + room / 2) / room).clamp(0, last);
        let step = if self.reversed { last - step } else { step };
        return step as u16;
    }

    /// The handle in `area`.
    fn handle_rect(&self, area: Rect) -> Rect {
        let track = self.track(area);
        let offset = self.handle_offset(self.value, track);
        let handle = Rect::new(offset, 0, self.handle_length(track), track.h);
        let handle = match self.direction {
            SliderDirection::Horizontal => handle,
            SliderDirection::Vertical => handle.swap_xy().rotate(),
        };
        return handle.within(area);
    }

    /// How far `pos` is along the slider from the start of `area`.
    fn along(&self, pos: Point, area: Rect) -> i32 {
        return match self.direction {
            SliderDirection::Horizontal => pos[0] - area.x,
            SliderDirection::Vertical => pos[1] - area.y,
        };
    }

    /// Moves the handle to follow the pointer, and tells the handler if that changed the value.
    fn drag_to(&mut self, state: &mut AppState, pos: Point, drag: Drag) {
        let track = self.track(self.area);
        let value = self.value_at(self.along(pos, self.area) - drag.grab, track);
        if value == self.value {
            return;
        }
        self.value = value;
        if let Some(handler) = &self.on_slide {
            let slide = Slide {
                start_value: drag.start_value,
                value,
            };
            handler(state, slide);
        }
    }
}

impl OctamedicWidget for Slider {
    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        match *event {
            Event::MouseDown {
                pos,
                button: MouseButton::Left,
            } if self.area.contains(pos) => {
                let handle = self.handle_rect(self.area);
                let handle_start = self.along([handle.x, handle.y], self.area);
                let handle_length = self.handle_length(self.track(self.area));
                // pressed on the track rather than the handle: the handle jumps to the pointer
                let grab = if handle.contains(pos) {
                    self.along(pos, self.area) - handle_start
                } else {
                    handle_length / 2
                };
                let drag = Drag {
                    start_value: self.value,
                    grab,
                };
                self.drag = Some(drag);
                self.drag_to(state, pos, drag);
                return true;
            }
            Event::MouseMove { pos } => {
                if let Some(drag) = self.drag {
                    self.drag_to(state, pos, drag);
                }
                return false;
            }
            Event::MouseUp {
                button: MouseButton::Left,
                ..
            } => return self.drag.take().is_some(),
            _ => return false,
        }
    }

    /// As OctaMED draws its sliders: a dotted track in a frame, and a light handle with a shadow
    /// along its bottom and just past its right edge.
    fn render(&mut self, _state: &AppState, area: Rect, painter: &mut Painter) {
        let track = painter.frame(area, 1);
        self.area = track;
        painter.dotted_rect(track, Pen::BLACK, Pen::SHADOW);
        let handle = self.handle_rect(track);
        painter.fill(handle, Pen::LIGHT);
        let bottom = Rect::new(handle.x, handle.bottom() - 1, handle.w, 1);
        painter.fill(bottom, Pen::SHADOW);
        if handle.right() < track.right() {
            painter.fill(
                Rect::new(handle.right(), handle.y, 1, handle.h),
                Pen::SHADOW,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    fn slider(direction: SliderDirection, reversed: bool) -> Slider {
        let mut slider = Slider::new(direction, reversed, 16);
        slider.area = match direction {
            SliderDirection::Horizontal => Rect::new(10, 20, 128, 8),
            SliderDirection::Vertical => Rect::new(10, 20, 8, 128),
        };
        return slider;
    }

    #[test]
    fn the_handle_steps_along_the_track() {
        let mut slider = slider(SliderDirection::Horizontal, false);
        assert_eq!(slider.handle_rect(slider.area), Rect::new(10, 20, 8, 8));
        slider.set_value(15);
        assert_eq!(slider.handle_rect(slider.area), Rect::new(130, 20, 8, 8));
        slider.set_value(100);
        assert_eq!(slider.value(), 15);
    }

    #[test]
    fn a_vertical_slider_steps_down_and_reversed_starts_at_the_end() {
        let mut slider = slider(SliderDirection::Vertical, true);
        assert_eq!(slider.handle_rect(slider.area), Rect::new(10, 140, 8, 8));
        slider.set_value(15);
        assert_eq!(slider.handle_rect(slider.area), Rect::new(10, 20, 8, 8));
    }

    #[test]
    fn dragging_moves_the_value_and_tells_the_handler() {
        let seen = Rc::new(Cell::new(None));
        let told = seen.clone();
        let mut slider = slider(SliderDirection::Horizontal, false)
            .on_slide(move |_, slide| told.set(Some(slide)));
        let mut state = state();
        let left = MouseButton::Left;
        // grabbing the handle where it is doesn't move it
        let press = Event::MouseDown {
            pos: [12, 22],
            button: left,
        };
        assert!(slider.handle_event(&mut state, &press));
        assert_eq!(seen.get(), None);
        let moved = Event::MouseMove {
            pos: [12 + 8 * 4, 22],
        };
        slider.handle_event(&mut state, &moved);
        assert_eq!(
            seen.get(),
            Some(Slide {
                start_value: 0,
                value: 4
            })
        );
        let release = Event::MouseUp {
            pos: [44, 22],
            button: left,
        };
        assert!(slider.handle_event(&mut state, &release));
        slider.handle_event(&mut state, &Event::MouseMove { pos: [130, 22] });
        assert_eq!(slider.value(), 4);
    }

    #[test]
    fn pressing_the_track_jumps_the_handle_there() {
        let mut slider = slider(SliderDirection::Horizontal, false);
        let press = Event::MouseDown {
            pos: [10 + 64, 22],
            button: MouseButton::Left,
        };
        slider.handle_event(&mut state(), &press);
        assert_eq!(slider.value(), 8);
    }
}
