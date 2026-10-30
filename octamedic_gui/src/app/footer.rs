use crate::{
    app::state::AppState,
    framework::{
        painter::{Painter, Pen},
        rect::Rect,
        widget::OctamedicWidget,
    },
};

const INFO_HEIGHT: i32 = 10;

/// The message line and, below it, the scopes.
pub struct Footer;

impl OctamedicWidget for Footer {
    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::FILL);
        let (info, scopes) = area.split_top(INFO_HEIGHT);
        painter.frame(info, 1);
        if let Some(notification) = &state.notification {
            painter.text(info.x + 4, info.y + 1, notification, Pen::BLACK);
        }
        painter.frame(scopes, 1);
    }
}
