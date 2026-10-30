use crate::{
    app::state::AppState,
    framework::{
        event::Event,
        font::Font,
        painter::{CHAR_HEIGHT, CHAR_WIDTH, Painter, Pen, text_width},
        rect::Rect,
        text::{Line, Span},
        widget::OctamedicWidget,
        widgets::button::Button,
    },
};

const TITLE: &str = "+- OctaMEDIC Professional";

/// After the title, without its shadow.
const VERSION: &str = "(v4.00)";

const AUTHOR: &str = "By Amano Rosuko";

/// The author's font, across and down; the reference draws it smaller than the rest.
const AUTHOR_SCALE: [f32; 2] = [1.0, 1.0];

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
        let text_y = area.y + (area.h - CHAR_HEIGHT) / 2;
        // a line lower than Topaz, as in the reference
        painter.text_in_font(area.x + 3, text_y + 1, "TIMER", Pen::LIGHT, Font::Panel);
        // where the reference has it, across the top 12 lines
        let timer_x = area.x + 50;
        self.timer
            .render(state, Rect::new(timer_x, area.y, 48, area.h - 1), painter);
        let title_x = timer_x + 8 * CHAR_WIDTH;
        let title = Line(vec![
            Span::new(TITLE).shadowed(),
            Span::new(" "),
            Span::new(VERSION),
        ]);
        painter.line(title_x, text_y, &title, Pen::LIGHT);
        let author_width = (text_width(AUTHOR) as f32 * AUTHOR_SCALE[0]) as i32;
        let author_x = area.right() - author_width - 4;
        painter.text_scaled(author_x, text_y, AUTHOR, Pen::LIGHT, AUTHOR_SCALE);
    }
}
