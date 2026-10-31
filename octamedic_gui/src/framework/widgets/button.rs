use std::rc::Rc;

use octamedic_util::primitives::Flip;

use crate::{
    app::state::AppState,
    framework::{
        clickable::{Click, Clickable},
        event::{Event, MouseButton},
        font::Font,
        painter::{Align, Painter, Pen, inside_frame, line_x, text_y},
        rect::{Point, Rect},
        shape::Direction,
        text::{Line, Span},
        widget::OctamedicWidget,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonClick {
    pub button: MouseButton,
    // which char of a label was clicked
    pub char_index: Option<usize>,
}

pub trait ButtonHandler: Fn(&mut AppState, ButtonClick) + 'static {}

impl<F: Fn(&mut AppState, ButtonClick) + 'static> ButtonHandler for F {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContentArea {
    pub inside: Rect,
    pub padded: Rect,
    pub align: Align,
    pub font: Font,
    pub bold: bool,
    // for content that paints its own background
    pub fill: Pen,
}

#[allow(unused_variables)]
pub trait ButtonContent {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter);

    fn index_at(&self, area: ContentArea, pos: Point) -> Option<usize> {
        return None;
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Triangle {
    pub direction: Direction,
    pub width: i32,
    pub height: i32,
    // bar across the tip, like the jump to start/end buttons
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

    pub fn arrow(direction: Direction) -> Self {
        return Self::new(direction, Self::ARROW_WIDTH, Self::ARROW_HEIGHT);
    }

    pub fn with_bar(mut self) -> Self {
        self.bar = true;
        return self;
    }

    const ARROW_WIDTH: i32 = 6;
    const ARROW_HEIGHT: i32 = 5;

    // 2px on left/right triangles, tall pixels
    const BAR_LINES: i32 = 1;
    const BAR_PIXELS: i32 = 2;

    fn size(&self) -> [i32; 2] {
        if !self.bar {
            return [self.width, self.height];
        }
        return match self.direction {
            Direction::Up | Direction::Down => [self.width, self.height + Self::BAR_LINES],
            Direction::Left | Direction::Right => [self.width + Self::BAR_PIXELS, self.height],
        };
    }

    // centred, rounding right and down
    fn area(&self, inside: Rect) -> Rect {
        let [width, height] = self.size();
        return Rect::new(
            inside.x + (inside.w - width + 1) / 2,
            inside.y + (inside.h - height + 1) / 2,
            width,
            height,
        );
    }

    // (bar, triangle)
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

// label then a down arrow, like INS in the play sequence
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrowLabel {
    pub label: &'static str,
}

impl ArrowLabel {
    pub fn new(label: &'static str) -> Self {
        return Self { label };
    }

    // on top of the blank column the font already leaves
    const GAP: i32 = 1;
    // [w, h]
    const STEM: [i32; 2] = [2, 4];
    const HEAD: [i32; 2] = [6, 3];
    const ARROW: [i32; 2] = [Self::HEAD[0], Self::STEM[1] + Self::HEAD[1]];

    // centred together, rounding right like OctaMED. if it doesn't fit the arrow stays in the
    // frame and the label spills out to the left
    fn place(&self, line: &Line, inside: Rect) -> (i32, Rect) {
        // from the first ink, same as line_x
        let (blank, _) = line.blank_sides();
        let [arrow_width, arrow_height] = Self::ARROW;
        let width = line.width() - blank + Self::GAP + arrow_width;
        let centred = inside.x + (inside.w - width + 1) / 2;
        let ink_x = centred.min(inside.right() - width);
        let x = ink_x - blank;
        let arrow_x = x + line.width() + Self::GAP;
        let arrow_y = inside.y + (inside.h - arrow_height + 1) / 2;
        return (x, Rect::new(arrow_x, arrow_y, arrow_width, arrow_height));
    }
}

impl ButtonContent for ArrowLabel {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        let line = label_line(self.label, area);
        let (x, arrow) = self.place(&line, area.inside);
        painter.line(x, text_y(area.padded, line.height()), &line, pen);
        let [stem_width, stem_height] = Self::STEM;
        let stem_x = arrow.x + (arrow.w - stem_width) / 2;
        painter.fill(Rect::new(stem_x, arrow.y, stem_width, stem_height), pen);
        let (_, head) = arrow.split_top(stem_height);
        painter.triangle(head, Some(pen), None, Direction::Down);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IncDec {
    is_increment: bool,
    width: i32,
    height: i32,
    thickness: i32,
}

impl IncDec {
    pub fn new(increment: bool, width: i32, height: i32, thickness: i32) -> Self {
        return Self {
            is_increment: increment,
            width,
            height,
            thickness,
        };
    }

    // rounds right and up, that's where OctaMED's sit
    fn lines(&self, inside: Rect) -> (Rect, Option<Rect>) {
        let x = inside.x + (inside.w - self.width + 1) / 2;
        let y = inside.y + (inside.h - self.height) / 2;
        let across = Rect::new(
            x,
            y + (self.height - self.thickness + 1) / 2,
            self.width,
            self.thickness,
        );
        let down_width = 2 * self.thickness;
        let down = Rect::new(
            x + (self.width - down_width + 1) / 2,
            y,
            down_width,
            self.height,
        );
        return (across, self.is_increment.then_some(down));
    }
}

impl ButtonContent for IncDec {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        let (across, down) = self.lines(area.inside);
        painter.fill(across, pen);
        if let Some(down) = down {
            painter.fill(down, pen);
        }
    }
}

// clones share the content and handler
#[derive(Clone)]
pub struct Button {
    content: Rc<dyn ButtonContent>,
    clickable: Clickable,
    on_click: Option<Rc<dyn ButtonHandler>>,
    // drawn inverted like OctaMED's switched on buttons
    toggled: bool,
    border: i32,
    // [x, y]
    padding: [i32; 2],
    align: Align,
    font: Font,
    bold: bool,
    // from the last render
    area: Rect,
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

    // for owners that poll instead of setting a handler
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

    pub fn set_content(&mut self, content: impl ButtonContent + 'static) {
        self.content = Rc::new(content);
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.set_content(text.into());
    }

    pub fn toggle(&mut self) {
        self.toggled.flip();
    }

    pub fn set_toggled(&mut self, toggled: bool) {
        self.toggled = toggled;
    }

    pub fn is_toggled(&self) -> bool {
        return self.toggled;
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

    // (fill, content), swapped when toggled. holding it down doesn't change the look, buttons
    // that want that use the toggle
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

// a label too long for the padding moves into it, OctaMED does that with CHRD
fn place_line<'a>(line: &'a Line, area: ContentArea) -> (Line<'a>, i32) {
    let line = line.fit(area.inside.w);
    let width = line.width();
    let x = line_x(area.padded, &line, area.align);
    // it may only hang a pixel over the end when it doesn't fit otherwise
    let last = area.inside.right() - width;
    return (line, x.min(last).max(area.inside.x));
}

fn label_line(text: &str, area: ContentArea) -> Line<'_> {
    let mut span = Span::new(text).font(area.font);
    span.bold = area.bold;
    return Line::from(span);
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

    // NEXT centred in 40px starts at x 4
    fn next_button(handler: impl ButtonHandler) -> Button {
        let mut button = Button::new("NEXT").on_click(handler);
        button.area = Rect::new(0, 0, 40, 10);
        button.clickable.set_area(button.area);
        return button;
    }

    #[test]
    fn triangle_bar() {
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
    fn ins_arrow() {
        // play sequence INS, 36 wide
        let button = Button::new("").font(Font::Label);
        let area = button.content_area(Rect::new(56, 32, 36, 11));
        let ins = ArrowLabel::new("INS");
        let line = label_line(ins.label, area);
        let (x, arrow) = ins.place(&line, area.inside);
        assert_eq!((arrow.w, arrow.h), (6, 7));
        assert!(arrow.right() <= area.inside.right());
        assert!(x + line.width() < arrow.x);
    }

    #[test]
    fn plus_minus() {
        let inside = Rect::new(10, 20, 16, 8);
        let (across, down) = IncDec::new(true, 8, 6, 1).lines(inside);
        assert_eq!(across, Rect::new(14, 24, 8, 1));
        assert_eq!(down, Some(Rect::new(17, 21, 2, 6)));
        let (minus, none) = IncDec::new(false, 8, 6, 1).lines(inside);
        assert_eq!((minus, none), (across, None));
    }

    #[test]
    fn toggle_inverts() {
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
    fn long_label_padding() {
        // the status row's CHRD and < boxes
        let place = |button: Button, rect, text: &'static str| {
            let line = Line::from(text);
            let (placed, x) = place_line(&line, button.content_area(rect));
            (placed.0[0].text.to_string(), x)
        };
        let chord = place(Button::new(""), Rect::new(216, 65, 35, 11), "CHRD");
        assert_eq!(chord, ("CHRD".to_string(), 218));
        let arrow = place(Button::new(""), Rect::new(560, 65, 13, 11), "<");
        assert_eq!(arrow, ("<".to_string(), 562));
        let left = Button::new("").align(Align::Left);
        let name = place(left, Rect::new(0, 0, 40, 11), "BreakBeat");
        assert_eq!(name, ("Brea".to_string(), 3));
    }

    #[test]
    fn click_kept() {
        let mut button = next_button(|_, _| {});
        assert_eq!(button.clicked(), None);
        click(&mut button, &mut state(), [5, 5]);
        let clicked = button.clicked().unwrap();
        assert_eq!(clicked.button, MouseButton::Left);
        assert_eq!(clicked.char_index, Some(0));
        assert_eq!(button.clicked(), None);
    }

    #[test]
    fn clone_shares_handler() {
        let mut state = state();
        let template = next_button(|state, _| state.next_pattern());
        let mut copy = template.clone();
        click(&mut copy, &mut state, [5, 5]);
        assert_eq!(state.active_pattern.0, 1);
    }

    #[test]
    fn arrow_position() {
        let inside = inside_frame(Rect::new(561, 65, 13, 11), 1);
        assert_eq!(
            Triangle::arrow(Direction::Left).area(inside),
            Rect::new(565, 68, 6, 5)
        );
    }

    #[test]
    fn content_padding() {
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
    fn click_runs_handler() {
        let mut state = state();
        let mut button = next_button(|state, _| state.next_pattern());
        click(&mut button, &mut state, [5, 5]);
        assert_eq!(state.active_pattern.0, 1);
    }

    #[test]
    fn click_char_index() {
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
