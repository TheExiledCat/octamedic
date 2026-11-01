use rustuition::{
    gadget::Gadget,
    painter::{Painter, Pen},
    rect::Rect,
};

use crate::{app::state::AppState, framework::event::Event};

const INFO_HEIGHT: i32 = 10;

const NOTIFICATION_OFFSET: [i32; 2] = [4, 1];

pub struct Footer;

impl Gadget<AppState, Event> for Footer {
    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::FILL);
        let (info, scopes) = area.split_top(INFO_HEIGHT);
        painter.frame(info, 1);
        if let Some(notification) = &state.notification {
            let [x, y] = NOTIFICATION_OFFSET;
            painter.text(info.x + x, info.y + y, notification, Pen::BACKGROUND);
        }
        painter.frame(scopes, 1);
    }
}
