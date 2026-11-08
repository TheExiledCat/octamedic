pub mod block;
pub mod files;
pub mod instrument;
pub mod labels;
pub mod misc;
pub mod play;
pub mod range;
pub mod transpose;
pub mod volume;
use rustuition::{
    gadget::Gadget,
    painter::{Align::Center, Painter, Pen},
    rect::Rect,
    text::Line,
};

use crate::{
    app::{
        header::{
            block::Block, files::Files, instrument::Instrument, misc::Misc, play::Play,
            range::Range, transpose::Transpose, volume::Volume,
        },
        mode::{HeaderMode, buttons::ModeButtons},
        state::AppState,
        status_bar::StatusBar,
    },
    framework::{event::Event, widgets::Textbox},
};

// from the header's top left
const WIDGETS_HEIGHT: i32 = 52;
const STATUS_HEIGHT: i32 = 11;
const STATUS_INSET: i32 = 3;
const PANEL: Rect = Rect::new(8, 4, 452, 47);
const MODE_BUTTONS: Rect = Rect::new(462, 2, 174, 50);

pub struct HeaderPanels {
    pub files: Files,
    pub instrument: Instrument,
    pub misc: Misc,
    pub play: Play,
    pub volume: Volume,
    pub block: Block,
    pub range: Range,
    pub transpose: Transpose,
}

impl HeaderPanels {
    pub fn new() -> Self {
        return Self {
            files: Files::new(),
            instrument: Instrument::new(),
            misc: Misc::new(),
            play: Play::new(),
            volume: Volume::new(),
            block: Block::new(),
            range: Range::new(),
            transpose: Transpose::new(),
        };
    }

    pub fn panel(&mut self, mode: HeaderMode) -> Option<&mut dyn Gadget<AppState, Event>> {
        return match mode {
            HeaderMode::Files => Some(&mut self.files),
            HeaderMode::Instrument => Some(&mut self.instrument),
            HeaderMode::Misc => Some(&mut self.misc),
            HeaderMode::Play => Some(&mut self.play),
            HeaderMode::Volume => Some(&mut self.volume),
            HeaderMode::Block => Some(&mut self.block),
            HeaderMode::Range => Some(&mut self.range),
            HeaderMode::Transpose => Some(&mut self.transpose),
            HeaderMode::Edit => None,
        };
    }
}

pub struct Header {
    panels: HeaderPanels,
    modes: ModeButtons,
    status: StatusBar,
}

impl Header {
    pub fn new() -> Self {
        return Self {
            panels: HeaderPanels::new(),
            modes: ModeButtons::new(),
            status: StatusBar::new(),
        };
    }
}

impl Gadget<AppState, Event> for Header {
    fn update(&mut self, state: &mut AppState) {
        if let Some(panel) = self.panels.panel(state.header_mode()) {
            panel.update(state);
        }
        self.modes.update(state);
        self.status.update(state);
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        let panel = self.panels.panel(state.header_mode());
        return panel.is_some_and(|panel| panel.handle_event(state, event))
            || self.modes.handle_event(state, event)
            || self.status.handle_event(state, event);
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::FILL);
        painter.frame(area, 1);
        let panel = Rect::new(area.x + PANEL.x, area.y + PANEL.y, PANEL.w, PANEL.h);
        let inside = painter.frame_inverted(panel, 1);
        if let Some(mode_panel) = self.panels.panel(state.header_mode()) {
            mode_panel.render(state, inside, painter);
        } else {
            painter.line_in(
                inside,
                &Line::from("Not implemented yet"),
                Pen::BACKGROUND,
                Center,
            );
        }
        let modes = Rect::new(
            area.x + MODE_BUTTONS.x,
            area.y + MODE_BUTTONS.y,
            MODE_BUTTONS.w,
            MODE_BUTTONS.h,
        );
        self.modes.render(state, modes, painter);
        let status = Rect::new(
            area.x + STATUS_INSET,
            area.y + WIDGETS_HEIGHT,
            area.w - 2 * STATUS_INSET,
            STATUS_HEIGHT,
        );
        self.status.render(state, status, painter);
    }
}

/// A textbox's text on the update it loses focus, from Enter or a click elsewhere.
pub fn finished_edit(textbox: &Textbox, was_focused: &mut bool) -> Option<String> {
    let left = *was_focused && !textbox.is_focused();
    *was_focused = textbox.is_focused();
    return left.then(|| textbox.text().to_owned());
}
