use crate::{
    app::{state::AppState, status_bar::StatusBar},
    framework::{
        event::Event,
        painter::{Painter, Pen},
        rect::Rect,
        widget::OctamedicWidget,
    },
};

/// Lines between the top of the frame and the status row: the widgets go there.
const WIDGETS_HEIGHT: i32 = 52;

const STATUS_HEIGHT: i32 = 11;

/// One framed panel, with the block and track tools above and the status row below.
pub struct Header {
    status: StatusBar,
}

impl Header {
    pub fn new() -> Self {
        return Self {
            status: StatusBar::new(),
        };
    }
}

impl OctamedicWidget for Header {
    fn update(&mut self, state: &mut AppState) {
        self.status.update(state);
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.status.handle_event(state, event);
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::FILL);
        painter.frame(area);
        // inside the frame's 2 pixel sides, with a pixel's gap
        let status = Rect::new(
            area.x + 3,
            area.y + WIDGETS_HEIGHT,
            area.w - 6,
            STATUS_HEIGHT,
        );
        self.status.render(state, status, painter);
    }
}
