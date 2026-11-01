use rustuition::{
    clickable::{Click, Clickable},
    gadget::Gadget,
    painter::{Painter, Pen},
    rect::Rect,
};

use crate::{
    app::{fonts, state::AppState},
    framework::event::{Event, MouseButton},
};

const COPYRIGHT: &str = "OctaMEDIC Professional © 2026 Amano Rosuko.";

const HEIGHT: i32 = 11;

const TEXT_OFFSET: [i32; 2] = [5, 1];

// in OctaMED this gadget sends the screen behind Workbench; here it quits
const GADGET_WIDTH: i32 = 23;

#[rustfmt::skip]
const GADGET_PICTURE: [&str; 9] = [
    ".....................",
    "...sssssssssss.......",
    "...s.........s.......",
    "...s...sssssssssss...",
    "...s...s#########s...",
    "...sssss#########s...",
    ".......s#########s...",
    ".......sssssssssss...",
    ".....................",
];

#[derive(Default)]
pub struct CopyrightBar {
    gadget: Clickable,
}

impl Gadget<AppState, Event> for CopyrightBar {
    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        if !state.displays_off {
            return false;
        }
        match self.gadget.handle(event) {
            Click::Clicked {
                button: MouseButton::Left,
                ..
            } => {
                state.request_exit();
                return true;
            }
            click => return click.handled(),
        }
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        if !state.displays_off {
            self.gadget.set_area(Rect::default());
            return;
        }
        let bar = Rect::new(area.x, area.y, area.w, HEIGHT);
        painter.fill(bar, Pen::SHINE);
        let [x, y] = TEXT_OFFSET;
        painter.text_in_font(
            bar.x + x,
            bar.y + y,
            COPYRIGHT,
            Pen::BACKGROUND,
            fonts::INTERFACE,
        );
        let gadget = Rect::new(bar.right() - GADGET_WIDTH, bar.y, GADGET_WIDTH, HEIGHT);
        self.gadget.set_area(gadget);
        // bevel: light top and left, shadow bottom and right
        painter.fill(gadget, Pen::SHADOW);
        painter.fill(
            Rect::new(gadget.x, gadget.y, gadget.w - 1, gadget.h - 1),
            Pen::SHINE,
        );
        painter.fill(Rect::new(gadget.x, gadget.y, gadget.w, 1), Pen::SHINE);
        let inside = Rect::new(gadget.x + 1, gadget.y + 1, gadget.w - 2, gadget.h - 2);
        painter.fill(inside, Pen::BACKGROUND);
        painter.picture(&GADGET_PICTURE, inside.x, inside.y);
    }
}
