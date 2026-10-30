pub mod misc;

use crate::{
    app::{
        header::misc::Misc, mode::HeaderMode, mode_buttons::ModeButtons, state::AppState,
        status_bar::StatusBar,
    },
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

/// The status row's gap from the panel's sides: inside the frame's 2 pixels, with a pixel more.
const STATUS_INSET: i32 = 3;

/// The recessed panel the header mode's widgets go in, from the header's top left.
const PANEL: Rect = Rect::new(8, 4, 452, 47);

/// The mode buttons, from the header's top left: 3 columns of 58 pixels and 5 rows of 10 lines.
const MODE_BUTTONS: Rect = Rect::new(462, 2, 174, 50);

pub struct HeaderPanels {
    pub misc: Misc,
}

impl HeaderPanels {
    pub fn new() -> Self {
        return Self { misc: Misc::new() };
    }

    /// The panel `mode` shows; `None` for the ones that aren't made yet.
    pub fn panel(&mut self, mode: HeaderMode) -> Option<&mut dyn OctamedicWidget> {
        return match mode {
            HeaderMode::Misc => Some(&mut self.misc),
            HeaderMode::Files
            | HeaderMode::Play
            | HeaderMode::Instrument
            | HeaderMode::Block
            | HeaderMode::Edit
            | HeaderMode::Volume
            | HeaderMode::Transpose
            | HeaderMode::Range => None,
        };
    }
}

/// One framed panel: the header mode's panel and the mode buttons above, the status row below.
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

impl OctamedicWidget for Header {
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
