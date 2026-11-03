use rustuition::{
    gadget::Gadget,
    gadgets::button::ContentArea,
    painter::{CHAR_HEIGHT, Painter, Pen, text_width},
    rect::Rect,
};

use crate::{
    app::{
        fonts,
        main::list_scroller::{ListScroller, SCROLLER_WIDTH},
        state::AppState,
    },
    framework::{
        event::Event,
        widgets::{Button, symbols::draw_rows},
    },
};

// from the screen's top left. the lists take the height that's left, so they still fit when NTSC
// cuts the screen short. nothing here does anything yet
const MESSAGE: Rect = Rect::new(10, 1, 620, 11);
const MESSAGE_TEXT_X: i32 = 6;
const LISTS_TOP: i32 = 13;
const LISTS_BOTTOM_MARGIN: i32 = 1;
// the directories, then the song list, each with its scroller after it
const LISTS: [(i32, i32, &str); 2] = [(10, 260, "No entries."), (350, 260, "No list.")];
const MIDDLE: [i32; 2] = [290, 60];
const LABELS: [(&str, i32); 4] = [("NAME", 13), ("DIR", 32), ("FILE", 51), ("SAVE LIST", 92)];
const BUTTONS: [(&str, Rect); 7] = [
    ("ADD", Rect::new(290, 19, 30, 11)),
    ("DEL", Rect::new(320, 19, 30, 11)),
    ("ADD", Rect::new(290, 38, 30, 11)),
    ("DEL", Rect::new(320, 38, 30, 11)),
    ("SAVE INST", Rect::new(290, 57, 60, 11)),
    ("SAVE ALL", Rect::new(290, 68, 60, 11)),
    ("DELETE", Rect::new(290, 79, 60, 11)),
];
const ARROW_BUTTONS: [(&str, Rect); 2] = [
    ("CD", Rect::new(290, 98, 30, 11)),
    ("S:", Rect::new(320, 98, 30, 11)),
];
// an arrow pointing right, then the label, inside the button's frame
const ARROW: &[&str] = &[
    "...##....",
    "...####..",
    "#########",
    "...####..",
    "...##....",
];
const ARROW_POSITION: [i32; 2] = [1, 2];
const ARROW_LABEL_X: i32 = 12;

pub struct SongList {
    scrollers: [ListScroller; 2],
    buttons: Vec<(Button, Rect)>,
}

impl SongList {
    pub fn new() -> Self {
        let labelled = BUTTONS
            .into_iter()
            .map(|(label, rect)| (Button::new(label).font(fonts::NARROW_LABEL), rect));
        let arrows = ARROW_BUTTONS
            .into_iter()
            .map(|(label, rect)| (Button::new(arrow_and(label)), rect));
        return Self {
            scrollers: std::array::from_fn(|_| ListScroller::new()),
            buttons: labelled
                .chain(arrows)
                .map(|(button, rect)| (button.enabled(false), rect))
                .collect(),
        };
    }
}

impl Gadget<AppState, Event> for SongList {
    fn update(&mut self, state: &mut AppState) {
        for scroller in &mut self.scrollers {
            scroller.update(state, 0);
        }
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::FILL);
        painter.frame(area, 1);
        let message = MESSAGE.within(area);
        painter.frame_inverted(message, 1);
        let y = message.y + (message.h - CHAR_HEIGHT + 1) / 2;
        let text = "No directories in memory!";
        painter.text(message.x + MESSAGE_TEXT_X, y, text, Pen::BACKGROUND);

        let height = area.h - LISTS_TOP - LISTS_BOTTOM_MARGIN;
        for ((x, width, empty), scroller) in LISTS.into_iter().zip(&mut self.scrollers) {
            let list = Rect::new(x, LISTS_TOP, width, height).within(area);
            let inside = painter.frame_inverted(list, 1);
            let x = inside.x + (inside.w - text_width(empty)) / 2;
            let y = inside.y + (inside.h - CHAR_HEIGHT) / 2;
            painter.text(x, y, empty, Pen::SHADOW);
            scroller.lines_that_fit = (inside.h / CHAR_HEIGHT) as usize;
            let scroller_area = Rect::new(list.right(), list.y, SCROLLER_WIDTH, height);
            scroller.render(state, scroller_area, 0, painter);
        }

        let [middle_x, middle_width] = MIDDLE;
        for (label, y) in LABELS {
            let x = middle_x + (middle_width - fonts::LABEL.width(label)) / 2;
            let at = Rect::new(x, y, 0, 0).within(area);
            painter.text_in_font(at.x, at.y, label, Pen::BACKGROUND, fonts::LABEL);
        }
        for (button, rect) in &mut self.buttons {
            button.render(state, rect.within(area), painter);
        }
    }
}

// an arrow pointing right, then `label`
fn arrow_and(label: &'static str) -> impl Fn(ContentArea, Pen, &mut Painter) {
    return move |area, pen, painter| {
        let inside = area.inside;
        let [x, y] = ARROW_POSITION;
        draw_rows(ARROW, inside.x + x, inside.y + y, pen, painter);
        let text_x = inside.x + ARROW_LABEL_X;
        painter.text_in_font(text_x, inside.y + y, label, pen, fonts::LABEL);
    };
}
