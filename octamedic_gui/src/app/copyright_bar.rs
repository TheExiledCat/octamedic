use octamedic_util::primitives::Flip;

use crate::{
    app::state::AppState,
    framework::{
        clickable::{Click, Clickable},
        event::{Event, MouseButton},
        font::Font,
        painter::{Painter, Pen},
        rect::Rect,
        widget::OctamedicWidget,
    },
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
    visible: bool,
    gadget: Clickable,
}

impl CopyrightBar {
    pub fn toggle(&mut self) {
        self.visible.flip();
    }

    pub fn is_visible(&self) -> bool {
        return self.visible;
    }
}

impl OctamedicWidget for CopyrightBar {
    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        if !self.visible {
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

    fn render(&mut self, _state: &AppState, area: Rect, painter: &mut Painter) {
        if !self.visible {
            self.gadget.set_area(Rect::default());
            return;
        }
        let bar = Rect::new(area.x, area.y, area.w, HEIGHT);
        painter.fill(bar, Pen::LIGHT);
        let [x, y] = TEXT_OFFSET;
        painter.text_in_font(bar.x + x, bar.y + y, COPYRIGHT, Pen::BLACK, Font::Interface);
        let gadget = Rect::new(bar.right() - GADGET_WIDTH, bar.y, GADGET_WIDTH, HEIGHT);
        self.gadget.set_area(gadget);
        // bevel: light top and left, shadow bottom and right
        painter.fill(gadget, Pen::SHADOW);
        painter.fill(
            Rect::new(gadget.x, gadget.y, gadget.w - 1, gadget.h - 1),
            Pen::LIGHT,
        );
        painter.fill(Rect::new(gadget.x, gadget.y, gadget.w, 1), Pen::LIGHT);
        let inside = Rect::new(gadget.x + 1, gadget.y + 1, gadget.w - 2, gadget.h - 2);
        painter.fill(inside, Pen::BLACK);
        painter.picture(&GADGET_PICTURE, inside.x, inside.y);
    }
}
