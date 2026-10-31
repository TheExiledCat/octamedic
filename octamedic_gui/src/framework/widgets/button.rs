use std::rc::Rc;

use octamedic_util::primitives::Flip;

use crate::{
    app::state::AppState,
    framework::{
        clickable::{Click, Clickable},
        event::{Event, MouseButton},
        font::Font,
        painter::{Align, Painter, Pen, aligned_text_x, inside_frame, text_y},
        rect::{Point, Rect},
        shape::Direction,
        text::{Line, Span},
        widget::OctamedicWidget,
    },
};

/// What a button's click handler is told.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonClick {
    pub button: MouseButton,
    /// The part of the content under the pointer, if any: for a label, its character, counting
    /// from 0.
    pub char_index: Option<usize>,
}

/// What a button runs when it's clicked: any closure taking the state and the click.
pub trait ButtonHandler: Fn(&mut AppState, ButtonClick) + 'static {}

impl<F: Fn(&mut AppState, ButtonClick) + 'static> ButtonHandler for F {}

/// Where a button's content goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentArea {
    /// Inside the frame.
    pub inside: Rect,
    /// Inside the frame and the padding.
    pub padded: Rect,
    pub align: Align,
    pub font: Font,
    pub bold: bool,
    /// The button's fill, for content that draws a background of its own in the content pen.
    pub fill: Pen,
}

/// What's drawn inside a button's frame.
#[allow(unused_variables)]
pub trait ButtonContent {
    /// Draws the content in `pen`, which is inverted with the button.
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter);

    /// Which part of the content is at `pos`, for `ButtonClick::char_index`.
    fn index_at(&self, area: ContentArea, pos: Point) -> Option<usize> {
        return None;
    }
}

/// The part of `line` that's drawn, and where it starts. It's cut to fit inside the frame and
/// lined up in the padding, but moved into the padding when it's too long for it, as OctaMED
/// does with labels like `CHRD`.
fn place_line<'a>(line: &'a Line, area: ContentArea) -> (Line<'a>, i32) {
    let line = line.fit(area.inside.w);
    let width = line.width();
    let x = aligned_text_x(area.padded, width, area.align);
    // it may only hang a pixel over the end when it doesn't fit otherwise
    let last = area.inside.right() - width;
    return (line, x.min(last).max(area.inside.x));
}

impl ButtonContent for Line<'static> {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        let (line, x) = place_line(self, area);
        painter.line(x, text_y(area.padded, line.height()), &line, pen);
    }

    fn index_at(&self, area: ContentArea, pos: Point) -> Option<usize> {
        let (line, x) = place_line(self, area);
        return line.index_at(pos[0] - x);
    }
}

/// A plain label is a line of one span, in the button's font.
fn label_line(text: &str, area: ContentArea) -> Line<'_> {
    let mut span = Span::new(text).font(area.font);
    span.bold = area.bold;
    return Line::from(span);
}

impl ButtonContent for str {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        let line = label_line(self, area);
        let (line, x) = place_line(&line, area);
        painter.line(x, text_y(area.padded, line.height()), &line, pen);
    }

    fn index_at(&self, area: ContentArea, pos: Point) -> Option<usize> {
        let line = label_line(self, area);
        let (line, x) = place_line(&line, area);
        return line.index_at(pos[0] - x);
    }
}

impl ButtonContent for &'static str {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        (**self).render(area, pen, painter);
    }

    fn index_at(&self, area: ContentArea, pos: Point) -> Option<usize> {
        return (**self).index_at(area, pos);
    }
}

impl ButtonContent for String {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        self.as_str().render(area, pen, painter);
    }

    fn index_at(&self, area: ContentArea, pos: Point) -> Option<usize> {
        return self.as_str().index_at(area, pos);
    }
}

/// A solid triangle pointing one way, `width` pixels by `height` lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Triangle {
    pub direction: Direction,
    pub width: i32,
    pub height: i32,
    /// A bar across the point, as on the buttons that jump to the start or end.
    pub bar: bool,
}

impl Triangle {
    pub fn new(direction: Direction, width: i32, height: i32) -> Self {
        return Self {
            direction,
            width,
            height,
            bar: false,
        };
    }

    const ARROW_WIDTH: i32 = 6;
    const ARROW_HEIGHT: i32 = 5;

    /// A bar is a line tall, or two pixels wide across a left or right pointing triangle, so it
    /// looks as thick on the tall pixels.
    const BAR_LINES: i32 = 1;
    const BAR_PIXELS: i32 = 2;

    pub fn arrow(direction: Direction) -> Self {
        return Self::new(direction, Self::ARROW_WIDTH, Self::ARROW_HEIGHT);
    }

    pub fn with_bar(mut self) -> Self {
        self.bar = true;
        return self;
    }

    /// The width and height of the triangle and its bar.
    fn size(&self) -> [i32; 2] {
        if !self.bar {
            return [self.width, self.height];
        }
        return match self.direction {
            Direction::Up | Direction::Down => [self.width, self.height + Self::BAR_LINES],
            Direction::Left | Direction::Right => [self.width + Self::BAR_PIXELS, self.height],
        };
    }

    /// In the middle of the inside of the frame, rounding right and down.
    fn area(&self, inside: Rect) -> Rect {
        let [width, height] = self.size();
        return Rect::new(
            inside.x + (inside.w - width + 1) / 2,
            inside.y + (inside.h - height + 1) / 2,
            width,
            height,
        );
    }

    /// `area` split into the bar, at the point, and the triangle.
    fn split_bar(&self, area: Rect) -> (Rect, Rect) {
        return match self.direction {
            Direction::Up => area.split_top(Self::BAR_LINES),
            Direction::Left => area.split_left(Self::BAR_PIXELS),
            Direction::Down => {
                let (triangle, bar) = area.split_top(area.h - Self::BAR_LINES);
                (bar, triangle)
            }
            Direction::Right => {
                let (triangle, bar) = area.split_left(area.w - Self::BAR_PIXELS);
                (bar, triangle)
            }
        };
    }
}

impl ButtonContent for Triangle {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        let mut triangle = self.area(area.inside);
        if self.bar {
            let (bar, rest) = self.split_bar(triangle);
            painter.fill(bar, pen);
            triangle = rest;
        }
        painter.triangle(triangle, Some(pen), None, self.direction);
    }
}

/// A gadget showing a label or anything else that's `ButtonContent`, which changes the state
/// when clicked. A clone shares the original's content and handler.
#[derive(Clone)]
pub struct Button {
    content: Rc<dyn ButtonContent>,
    clickable: Clickable,
    on_click: Option<Rc<dyn ButtonHandler>>,
    /// Drawn inverted, like OctaMED's buttons that are switched on.
    toggled: bool,
    /// The frame's thickness, as `Painter::frame` takes it.
    border: i32,
    /// Pixels between the frame and the content, on every side.
    /// Across and down.
    padding: [i32; 2],
    align: Align,
    font: Font,
    bold: bool,
    /// Where the button was last rendered.
    area: Rect,
    /// The last click, until `clicked` takes it.
    click: Option<ButtonClick>,
}

impl Button {
    pub fn new(content: impl ButtonContent + 'static) -> Self {
        return Self {
            content: Rc::new(content),
            clickable: Clickable::default(),
            on_click: None,
            toggled: false,
            border: 1,
            padding: [1, 1],
            align: Align::Center,
            font: Font::Topaz,
            bold: false,
            area: Rect::default(),
            click: None,
        };
    }

    /// The click since this was last called, if there was one. The widget that owns the button
    /// can check this after passing it events, instead of giving it a handler.
    pub fn clicked(&mut self) -> Option<ButtonClick> {
        return self.click.take();
    }

    pub fn on_click(mut self, handler: impl Fn(&mut AppState, ButtonClick) + 'static) -> Self {
        self.set_on_click(handler);
        return self;
    }

    pub fn set_on_click(&mut self, handler: impl Fn(&mut AppState, ButtonClick) + 'static) {
        self.on_click = Some(Rc::new(handler));
    }

    pub fn border(mut self, border: i32) -> Self {
        self.border = border;
        return self;
    }

    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        return self;
    }

    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        return self;
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        return self;
    }

    pub fn padding(mut self, padding: i32) -> Self {
        self.padding = [padding, padding];
        return self;
    }

    pub fn padding_xy(mut self, x: i32, y: i32) -> Self {
        self.padding = [x, y];
        return self;
    }

    fn content_area(&self, area: Rect) -> ContentArea {
        let inside = inside_frame(area, self.border);
        return ContentArea {
            inside,
            padded: inside.inset(self.padding[0], self.padding[1]),
            align: self.align,
            font: self.font,
            bold: self.bold,
            fill: self.pens().0,
        };
    }

    pub fn set_content(&mut self, content: impl ButtonContent + 'static) {
        self.content = Rc::new(content);
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.set_content(text.into());
    }

    /// Switches between drawn normally and inverted.
    pub fn toggle(&mut self) {
        self.toggled.flip();
    }

    pub fn set_toggled(&mut self, toggled: bool) {
        self.toggled = toggled;
    }

    pub fn is_toggled(&self) -> bool {
        return self.toggled;
    }

    /// The fill and content pens, swapped while it's toggled. Holding it down doesn't change
    /// how it looks; a button that should show it does so through its toggle.
    fn pens(&self) -> (Pen, Pen) {
        let (fill, content) = (Pen::FILL, Pen::BLACK);
        return if self.toggled {
            (content, fill)
        } else {
            (fill, content)
        };
    }
}

impl OctamedicWidget for Button {
    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        let click = self.clickable.handle(event);
        if let Click::Clicked { button, pos } = click {
            let char_index = self.content.index_at(self.content_area(self.area), pos);
            let click = ButtonClick { button, char_index };
            self.click = Some(click);
            if let Some(handler) = &self.on_click {
                handler(state, click);
            }
        }
        return click.handled();
    }

    fn render(&mut self, _state: &AppState, area: Rect, painter: &mut Painter) {
        self.area = area;
        self.clickable.set_area(area);
        let (fill, content) = self.pens();
        painter.fill(area, fill);
        painter.frame(area, self.border);
        self.content
            .render(self.content_area(area), content, painter);
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
    fn next_button(handler: impl ButtonHandler) -> Button {
        let mut button = Button::new("NEXT").on_click(handler);
        button.area = Rect::new(0, 0, 40, 10);
        button.clickable.set_area(button.area);
        return button;
    }

    #[test]
    fn a_bar_goes_across_the_point() {
        let inside = Rect::new(50, 20, 16, 9);
        let up = Triangle::new(Direction::Up, 12, 6).with_bar();
        let area = up.area(inside);
        assert_eq!(area, Rect::new(52, 21, 12, 7));
        assert_eq!(up.split_bar(area).0, Rect::new(52, 21, 12, 1));
        let down = Triangle::new(Direction::Down, 12, 6).with_bar();
        assert_eq!(down.split_bar(area).0, Rect::new(52, 27, 12, 1));
        let right = Triangle::new(Direction::Right, 6, 5).with_bar();
        assert_eq!(right.size(), [8, 5]);
    }

    #[test]
    fn only_toggling_inverts_the_button() {
        let normal = (Pen::FILL, Pen::BLACK);
        let inverted = (Pen::BLACK, Pen::FILL);
        let mut button = next_button(|_, _| {});
        let down = Event::MouseDown {
            pos: [5, 5],
            button: MouseButton::Left,
        };
        button.handle_event(&mut state(), &down);
        assert_eq!(button.pens(), normal);
        button.toggle();
        assert!(button.is_toggled());
        assert_eq!(button.pens(), inverted);
        button.set_toggled(false);
        assert_eq!(button.pens(), normal);
    }

    #[test]
    fn labels_too_long_for_the_padding_move_into_it() {
        // the status row's CHRD and < boxes
        let place = |button: Button, rect, text: &'static str| {
            let line = Line::from(text);
            let (placed, x) = place_line(&line, button.content_area(rect));
            (placed.0[0].text.to_string(), x)
        };
        let chord = place(Button::new(""), Rect::new(216, 65, 35, 11), "CHRD");
        assert_eq!(chord, ("CHRD".to_string(), 218));
        let arrow = place(Button::new(""), Rect::new(560, 65, 13, 11), "<");
        assert_eq!(arrow, ("<".to_string(), 563));
        let left = Button::new("").align(Align::Left);
        let name = place(left, Rect::new(0, 0, 40, 11), "BreakBeat");
        assert_eq!(name, ("Brea".to_string(), 3));
    }

    #[test]
    fn a_click_is_kept_until_it_is_taken() {
        let mut button = next_button(|_, _| {});
        assert_eq!(button.clicked(), None);
        click(&mut button, &mut state(), [5, 5]);
        let clicked = button.clicked().unwrap();
        assert_eq!(clicked.button, MouseButton::Left);
        assert_eq!(clicked.char_index, Some(0));
        assert_eq!(button.clicked(), None);
    }

    #[test]
    fn a_clone_runs_the_same_handler() {
        let mut state = state();
        let template = next_button(|state, _| state.next_pattern());
        let mut copy = template.clone();
        click(&mut copy, &mut state, [5, 5]);
        assert_eq!(state.active_pattern.0, 1);
    }

    #[test]
    fn arrows_sit_where_octamed_has_them() {
        let inside = inside_frame(Rect::new(561, 65, 13, 11), 1);
        assert_eq!(
            Triangle::arrow(Direction::Left).area(inside),
            Rect::new(565, 68, 6, 5)
        );
    }

    #[test]
    fn the_label_sits_inside_the_frame_and_padding() {
        let area = Rect::new(3, 65, 84, 11);
        assert_eq!(
            Button::new("").content_area(area).padded,
            Rect::new(6, 67, 78, 7)
        );
        assert_eq!(
            Button::new("").padding(0).content_area(area).padded,
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
