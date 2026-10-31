use crate::{
    app::state::AppState,
    framework::{
        event::Event,
        font::Font,
        painter::{Painter, Pen},
        rect::Rect,
        text::{Line, Span},
        widget::OctamedicWidget,
        widgets::button::Button,
    },
};

const TITLE: &str = "OctaMEDIC Professional";

const VERSION: &str = "(v4.00)";

const AUTHOR: &str = "By Amano Rosuko";

// positions measured from OctaMED's title bar
const TIMER_LABEL: [i32; 2] = [3, 3];
const TIMER: Rect = Rect::new(50, 0, 48, 12);
const STAR_X: i32 = 100;
const TITLE_POSITION: [i32; 2] = [125, 1];
const VERSION_GAP: i32 = 8;
const AUTHOR_Y: i32 = 6;
const AUTHOR_RIGHT_MARGIN: i32 = 8;
const AUTHOR_STAR_OFFSET: i32 = 30;
const TAIL_Y: i32 = 4;

#[rustfmt::skip]
const STAR: [&str; 12] = [
    "...........",
    "......ss...",
    "......ff...",
    "......#f...",
    "..sssf##fss",
    "ssf#f####f#",
    "..sssf##fss",
    "......f#...",
    "......ff...",
    "......ss...",
    "...........",
    "...........",
];

// its right arm runs on under the author's first letters
#[rustfmt::skip]
const AUTHOR_STAR: [&str; 13] = [
    "....................s......................",
    "....................ss.....................",
    "....................fs.....................",
    "....................#f.....................",
    "...................f#fs....................",
    "...................f##f....................",
    "...................sf#s....................",
    "...................s#fs....................",
    "..............ssssff##ffss.................",
    "s.ssssfffff#####f########f#f#f#ffsffffsssss",
    ".............sssssff##ffsss................",
    "..................ss#fss...................",
    "...................sf#s....................",
];

pub struct TitleBar {
    timer: Button,
}

impl TitleBar {
    pub fn new() -> Self {
        return Self {
            timer: Button::new("00:00")
                .border(2)
                .on_click(|state, _| state.restart_timer()),
        };
    }
}

impl OctamedicWidget for TitleBar {
    fn update(&mut self, state: &mut AppState) {
        let seconds = state.timer().as_secs();
        self.timer
            .set_text(format!("{:02}:{:02}", seconds / 60, seconds % 60));
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.timer.handle_event(state, event);
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::BLACK);
        let [x, y] = TIMER_LABEL;
        painter.text_in_font(area.x + x, area.y + y, "TIMER", Pen::LIGHT, Font::Panel);
        let timer = Rect::new(area.x + TIMER.x, area.y + TIMER.y, TIMER.w, TIMER.h);
        self.timer.render(state, timer, painter);
        let [x, y] = TITLE_POSITION;
        let (title_x, title_y) = (area.x + x, area.y + y);
        let title = Line::from(Span::new(TITLE).font(Font::Title).shadowed());
        let title_end = title_x + title.width();
        let version_x = title_end + VERSION_GAP;
        let version_end = version_x + Font::Title.width(VERSION);
        // tails go behind the text: star to title, between words, and version to the edge
        let tail_y = area.y + TAIL_Y;
        let star_end = area.x + STAR_X + STAR[0].len() as i32;
        render_ornament_tail(
            painter,
            star_end,
            tail_y,
            title_x - 1 - star_end,
            Taper::Right,
        );
        let mut char_x = title_x;
        for c in TITLE.chars() {
            let advance = Font::Title.advance(c);
            if c == ' ' {
                render_ornament_tail(painter, char_x, tail_y, advance - 1, Taper::Both);
            }
            char_x += advance;
        }
        // starts back over the last letter's trailing gap
        let gap_x = title_end - 2;
        render_ornament_tail(painter, gap_x, tail_y, version_x - 1 - gap_x, Taper::Both);
        render_ornament_tail(
            painter,
            version_end,
            tail_y,
            area.right() - version_end,
            Taper::Both,
        );
        painter.picture(&STAR, area.x + STAR_X, area.y);
        painter.line(title_x, title_y, &title, Pen::LIGHT);
        painter.text_in_font(version_x, title_y, VERSION, Pen::LIGHT, Font::Title);
        let author_x = area.right() - AUTHOR_RIGHT_MARGIN - Font::Author.width(AUTHOR);
        painter.picture(&AUTHOR_STAR, author_x - AUTHOR_STAR_OFFSET, area.y);
        painter.text_in_font(
            author_x,
            area.y + AUTHOR_Y,
            AUTHOR,
            Pen::LIGHT,
            Font::Author,
        );
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Taper {
    Right,
    Both,
}

// a fill line between shadow lines; at a tapered end the shadow lines stop short and the fill
// line ends in shadow
fn render_ornament_tail(painter: &mut Painter, x: i32, y: i32, width: i32, taper: Taper) {
    const TAPER: i32 = 2;
    let left = if taper == Taper::Both { TAPER } else { 0 };
    let inner = Rect::new(x + left, y, width - left - TAPER, 1);
    if inner.w <= 0 {
        return;
    }
    painter.fill(inner, Pen::SHADOW);
    painter.fill(Rect::new(inner.x, y + 2, inner.w, 1), Pen::SHADOW);
    painter.fill(Rect::new(x, y + 1, width, 1), Pen::SHADOW);
    painter.fill(Rect::new(inner.x, y + 1, inner.w, 1), Pen::FILL);
}
