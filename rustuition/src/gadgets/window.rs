use crate::{
    event::PointerEvent,
    gadget::Gadget,
    gadgets::button::{Button, ContentArea},
    painter::{CHAR_HEIGHT, Painter, Pen, inside_frame, text_y},
    rect::Rect,
};

const TITLE_BAR_HEIGHT: i32 = CHAR_HEIGHT + 3;
const TITLE_BAR_FILL: Pen = Pen::SHADOW;
const TITLE_TEXT: Pen = Pen::SHINE;
const TITLE_TEXT_GAP: i32 = 4;
const CLOSE_GADGET_WIDTH: i32 = 20;
const CLOSE_GADGET_KNOB: [i32; 2] = [8, 4];
const FRAME_THICKNESS: i32 = 1;

/// A Workbench style window: a frame, and a title bar with a close gadget. It only draws itself,
/// the contents go in [`Window::content_area`].
pub struct Window<S, E> {
    title: String,
    close_gadget: Option<Button<S, E>>,
    close_clicked: bool,
}

impl<S, E> Window<S, E> {
    pub fn new(title: impl Into<String>) -> Self {
        return Self {
            title: title.into(),
            close_gadget: Some(Button::new(close_gadget)),
            close_clicked: false,
        };
    }

    /// With a close gadget or not. New windows have one.
    pub fn closable(mut self, closable: bool) -> Self {
        self.close_gadget = closable.then(|| Button::new(close_gadget));
        return self;
    }

    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    /// Whether the close gadget was clicked since the last time this was asked.
    pub fn take_close_click(&mut self) -> bool {
        return std::mem::take(&mut self.close_clicked);
    }

    pub fn height_for(content_height: i32) -> i32 {
        return 2 * FRAME_THICKNESS + TITLE_BAR_HEIGHT + content_height;
    }

    pub fn content_area(area: Rect) -> Rect {
        let (_, content) = inside_frame(area, FRAME_THICKNESS).split_top(TITLE_BAR_HEIGHT);
        return content;
    }
}

impl<S, E: PointerEvent> Window<S, E> {
    fn render_title_bar(&mut self, state: &S, bar: Rect, painter: &mut Painter) {
        painter.fill(bar, TITLE_BAR_FILL);
        let title = match &mut self.close_gadget {
            Some(close_gadget) => {
                let (close, title) = bar.split_left(CLOSE_GADGET_WIDTH);
                close_gadget.render(state, close, painter);
                title
            }
            None => bar,
        };
        let x = title.x + TITLE_TEXT_GAP;
        painter.clipped(title, |painter| {
            painter.text(x, text_y(title, CHAR_HEIGHT), &self.title, TITLE_TEXT);
        });
    }
}

impl<S, E: PointerEvent> Gadget<S, E> for Window<S, E> {
    /// Only the close gadget takes events, the rest of the window lets them through.
    fn handle_event(&mut self, state: &mut S, event: &E) -> bool {
        let Some(close_gadget) = &mut self.close_gadget else {
            return false;
        };
        if !close_gadget.handle_event(state, event) {
            return false;
        }
        if close_gadget.clicked().is_some() {
            self.close_clicked = true;
        }
        return true;
    }

    fn render(&mut self, state: &S, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::FILL);
        let inside = painter.frame(area, FRAME_THICKNESS);
        let (bar, _) = inside.split_top(TITLE_BAR_HEIGHT);
        self.render_title_bar(state, bar, painter);
    }
}

// the small square knob in the middle
fn close_gadget(area: ContentArea, pen: Pen, painter: &mut Painter) {
    let [w, h] = CLOSE_GADGET_KNOB;
    let inside = area.inside;
    let knob = Rect::new(
        inside.x + (inside.w - w) / 2,
        inside.y + (inside.h - h) / 2,
        w,
        h,
    );
    painter.fill(knob, pen);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        event::{MouseButton, MouseEvent},
        palette::{Color, IntuitionColorPalette},
        rect::Point,
        target::Framebuffer,
    };

    const AREA: Rect = Rect::new(100, 50, 200, 80);

    fn render(window: &mut Window<(), MouseEvent>) -> Framebuffer {
        let palette =
            IntuitionColorPalette::new([Color(0x000), Color(0xFFF), Color(0x666), Color(0xAAA)]);
        let mut screen = Framebuffer::new(640, 256);
        window.render(&(), AREA, &mut Painter::new(&mut screen, &palette));
        return screen;
    }

    fn press(pos: Point) -> MouseEvent {
        return MouseEvent::Down {
            pos,
            button: MouseButton::Left,
        };
    }

    #[test]
    fn content_area_is_under_the_title_bar() {
        let content = Window::<(), MouseEvent>::content_area(AREA);
        assert_eq!(
            content,
            Rect::new(102, 51 + TITLE_BAR_HEIGHT, 196, 78 - TITLE_BAR_HEIGHT)
        );
        assert_eq!(Window::<(), MouseEvent>::height_for(content.h), AREA.h);
    }

    #[test]
    fn close_gadget_click_is_taken_once() {
        let mut window = Window::new("Test");
        render(&mut window);
        assert!(window.handle_event(&mut (), &press(Point::new(AREA.x + 8, AREA.y + 5))));
        assert!(window.take_close_click());
        assert!(!window.take_close_click());
    }

    #[test]
    fn lets_other_clicks_through() {
        let mut window = Window::new("Test");
        render(&mut window);
        assert!(!window.handle_event(&mut (), &press(Point::new(AREA.x + 100, AREA.y + 40))));
        assert!(!window.take_close_click());
    }

    #[test]
    fn no_close_gadget() {
        let mut window = Window::new("Test").closable(false);
        render(&mut window);
        assert!(!window.handle_event(&mut (), &press(Point::new(AREA.x + 8, AREA.y + 5))));
    }

    #[test]
    fn title_bar_is_filled() {
        let mut window = Window::new("");
        let screen = render(&mut window);
        let shadow = [0x66, 0x66, 0x66];
        assert_eq!(screen.pixel(AREA.x + 100, AREA.y + 3), Some(shadow));
    }
}
