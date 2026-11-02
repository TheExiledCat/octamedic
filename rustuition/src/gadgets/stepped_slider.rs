use std::time::Duration;

use crate::{
    event::PointerEvent,
    gadget::Gadget,
    gadgets::{
        button::{Button, DEFAULT_REPEAT_DELAY, DEFAULT_REPEAT_INTERVAL, Triangle},
        slider::{Slider, SliderDirection},
    },
    painter::Painter,
    rect::Rect,
    shape::Direction,
};

/// A slider with two arrow buttons after it, right of it or under it, that step its value by one
/// and keep stepping while they're held. Its value goes from `min` to `max`.
///
/// Call [`SteppedSlider::update`] every frame with the value it's showing: it hands back a new one
/// when the slider's dragged or an arrow's pressed.
pub struct SteppedSlider<S, E> {
    slider: Slider<S, E>,
    arrows: [Button<S, E>; 2],
    direction: SliderDirection,
    // across the arrows for a horizontal slider, down them for a vertical one
    arrow_size: i32,
    min: u16,
    max: u16,
    // what the slider was last set to, so a drag shows up as a change
    shown: u16,
}

impl<S, E> SteppedSlider<S, E> {
    /// `arrow` is the triangles' width and height, `arrow_size` how much of the slider's length
    /// each arrow button takes.
    pub fn new(
        direction: SliderDirection,
        min: u16,
        max: u16,
        arrow: [i32; 2],
        arrow_size: i32,
    ) -> Self {
        let directions = match direction {
            SliderDirection::Horizontal => [Direction::Left, Direction::Right],
            SliderDirection::Vertical => [Direction::Up, Direction::Down],
        };
        let [width, height] = arrow;
        let mut stepped = Self {
            slider: Slider::new(direction, false, 1),
            arrows: directions.map(|direction| {
                Button::new(Triangle::new(direction, width, height))
                    .repeat(DEFAULT_REPEAT_DELAY, DEFAULT_REPEAT_INTERVAL)
            }),
            direction,
            arrow_size,
            min,
            max,
            shown: 0,
        };
        stepped.set_range(min, max);
        return stepped;
    }

    /// How long an arrow's held before it starts stepping, and how often after that.
    pub fn repeat(mut self, delay: Duration, interval: Duration) -> Self {
        self.arrows = self.arrows.map(|arrow| arrow.repeat(delay, interval));
        return self;
    }

    /// See [`Slider::fixed_handle`].
    pub fn fixed_handle(mut self, length: i32) -> Self {
        self.slider = self.slider.fixed_handle(length);
        return self;
    }

    /// See [`Slider::set_handle`].
    pub fn set_handle(&mut self, length: Option<i32>) {
        self.slider.set_handle(length);
    }

    pub fn set_range(&mut self, min: u16, max: u16) {
        self.min = min;
        self.max = max.max(min);
        self.slider.set_steps(self.max - self.min + 1);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.slider.set_enabled(enabled);
        for arrow in &mut self.arrows {
            arrow.set_enabled(enabled);
        }
    }

    /// For moving the slider directly, like a drag would.
    pub fn slider_mut(&mut self) -> &mut Slider<S, E> {
        return &mut self.slider;
    }

    pub fn arrows(&self) -> &[Button<S, E>; 2] {
        return &self.arrows;
    }

    fn show(&mut self, value: u16) {
        self.slider
            .set_value(value.clamp(self.min, self.max) - self.min);
        self.shown = self.slider.value();
    }
}

impl<S, E: PointerEvent> SteppedSlider<S, E> {
    /// The new value if the slider's been dragged or an arrow pressed, otherwise `None`. Either
    /// way the slider ends up showing it.
    pub fn update(&mut self, state: &mut S, current: u16) -> Option<u16> {
        let mut wanted = None;
        if self.slider.value() != self.shown {
            wanted = Some(self.slider.value() + self.min);
        }
        for (arrow, step) in self.arrows.iter_mut().zip([-1, 1]) {
            arrow.update(state);
            if arrow.clicked().is_some() {
                wanted = Some(current.saturating_add_signed(step));
            }
        }
        let wanted = wanted
            .map(|value| value.clamp(self.min, self.max))
            .filter(|&value| value != current);
        self.show(wanted.unwrap_or(current));
        return wanted;
    }
}

impl<S, E: PointerEvent> Gadget<S, E> for SteppedSlider<S, E> {
    fn handle_event(&mut self, state: &mut S, event: &E) -> bool {
        return self.slider.handle_event(state, event)
            || self
                .arrows
                .iter_mut()
                .any(|arrow| arrow.handle_event(state, event));
    }

    fn render(&mut self, state: &S, area: Rect, painter: &mut Painter) {
        let arrows = 2 * self.arrow_size;
        let (slider, arrow_rects) = match self.direction {
            SliderDirection::Horizontal => {
                let (slider, arrows) = area.split_left(area.w - arrows);
                (
                    slider,
                    [0, 1].map(|index| {
                        Rect::new(
                            arrows.x + index * self.arrow_size,
                            arrows.y,
                            self.arrow_size,
                            arrows.h,
                        )
                    }),
                )
            }
            SliderDirection::Vertical => {
                let (slider, arrows) = area.split_top(area.h - arrows);
                (
                    slider,
                    [0, 1].map(|index| {
                        Rect::new(
                            arrows.x,
                            arrows.y + index * self.arrow_size,
                            arrows.w,
                            self.arrow_size,
                        )
                    }),
                )
            }
        };
        self.slider.render(state, slider, painter);
        for (arrow, rect) in self.arrows.iter_mut().zip(arrow_rects) {
            arrow.render(state, rect, painter);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{MouseButton, MouseEvent},
        palette::{Color, IntuitionColorPalette},
        target::Framebuffer,
    };

    const AREA: Rect = Rect::new(0, 0, 100, 10);

    fn render(stepped: &mut SteppedSlider<(), MouseEvent>) {
        let palette =
            IntuitionColorPalette::new([Color(0x000), Color(0xFFF), Color(0x666), Color(0xAAA)]);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        stepped.render(&(), AREA, &mut Painter::new(&mut screen, &palette));
    }

    fn press(stepped: &mut SteppedSlider<(), MouseEvent>, arrow: usize) {
        let area = stepped.arrows()[arrow].area();
        let pos = [area.x + area.w / 2, area.y + area.h / 2];
        stepped.handle_event(
            &mut (),
            &MouseEvent::Down {
                pos,
                button: MouseButton::Left,
            },
        );
        stepped.handle_event(
            &mut (),
            &MouseEvent::Up {
                pos,
                button: MouseButton::Left,
            },
        );
    }

    #[test]
    fn arrows_step_within_the_range() {
        let mut stepped = SteppedSlider::new(SliderDirection::Horizontal, 1, 10, [6, 5], 18);
        assert_eq!(stepped.update(&mut (), 1), None);
        render(&mut stepped);
        press(&mut stepped, 1);
        assert_eq!(stepped.update(&mut (), 1), Some(2));
        press(&mut stepped, 0);
        assert_eq!(stepped.update(&mut (), 1), None);
        assert_eq!(stepped.arrows()[1].area(), Rect::new(82, 0, 18, 10));
    }

    #[test]
    fn dragging_gives_the_value() {
        let mut stepped: SteppedSlider<(), MouseEvent> =
            SteppedSlider::new(SliderDirection::Vertical, 0, 20, [12, 6], 10);
        stepped.update(&mut (), 3);
        assert_eq!(stepped.slider_mut().value(), 3);
        stepped.slider_mut().set_value(7);
        assert_eq!(stepped.update(&mut (), 3), Some(7));
        assert_eq!(stepped.update(&mut (), 7), None);
        // a value set from outside moves the slider without counting as a drag
        assert_eq!(stepped.update(&mut (), 12), None);
        assert_eq!(stepped.slider_mut().value(), 12);
    }
}
