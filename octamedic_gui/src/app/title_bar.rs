use crate::{
    app::state::AppState,
    framework::{
        event::Event,
        painter::{CHAR_HEIGHT, CHAR_WIDTH, Painter, Pen, text_width},
        rect::Rect,
        widget::OctamedicWidget,
        widgets::button::Button,
    },
};

const TITLE: &str = "OctaMEDIC Professional (v4.00)";

const AUTHOR: &str = "By Amano Rosuko";

pub struct TitleBar {
    timer: Button,
}

impl TitleBar {
    pub fn new() -> Self {
        return Self {
            timer: Button::new("00:00").on_click(|state, _| state.restart_timer()),
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
        painter.text(area.x + 4, text_y, "TIMER", Pen::LIGHT);
        // where the reference has it, across the top 12 lines
        let timer_x = area.x + 50;
        self.timer
            .render(state, Rect::new(timer_x, area.y, 48, area.h - 1), painter);
        let title_x = timer_x + 8 * CHAR_WIDTH;
        painter.text(title_x + 1, text_y + 1, TITLE, Pen::SHADOW);
        painter.text(title_x, text_y, TITLE, Pen::FILL);
        let author_x = area.right() - text_width(AUTHOR) - 4;
        painter.text(author_x, text_y, AUTHOR, Pen::LIGHT);
    }
}
