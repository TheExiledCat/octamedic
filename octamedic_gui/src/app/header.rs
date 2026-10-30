use crate::{
    app::{mode::HeaderMode, mode_buttons::ModeButtons, state::AppState, status_bar::StatusBar},
    framework::{
        event::Event,
        painter::{Painter, Pen, inside_frame},
        rect::Rect,
        widget::OctamedicWidget,
    },
};

/// Lines between the top of the frame and the status row: the widgets go there.
const WIDGETS_HEIGHT: i32 = 52;

const STATUS_HEIGHT: i32 = 11;

/// The recessed panel the header mode's widgets go in, from the header's top left.
const PANEL: Rect = Rect::new(8, 4, 452, 47);

/// The mode buttons, from the header's top left: 3 columns of 58 pixels and 5 rows of 10 lines.
const MODE_BUTTONS: Rect = Rect::new(462, 2, 174, 50);

/// One framed panel, with the block and track tools above and the status row below.
pub struct Header {
    modes: ModeButtons,
    status: StatusBar,
}

impl Header {
    pub fn new() -> Self {
        return Self {
            modes: ModeButtons::new(),
            status: StatusBar::new(),
        };
    }
}

impl OctamedicWidget for Header {
    fn update(&mut self, state: &mut AppState) {
        self.modes.update(state);
        self.status.update(state);
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.modes.handle_event(state, event) || self.status.handle_event(state, event);
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::FILL);
        painter.frame(area, 1);
        let panel = Rect::new(area.x + PANEL.x, area.y + PANEL.y, PANEL.w, PANEL.h);
        painter.frame_inverted(panel, 1);
        let _widgets = inside_frame(panel, 1);
        match state.header_mode() {
            // none of the panels are made yet; they go in `_widgets`
            HeaderMode::Files
            | HeaderMode::Play
            | HeaderMode::Instrument
            | HeaderMode::Block
            | HeaderMode::Edit
            | HeaderMode::Misc
            | HeaderMode::Volume
            | HeaderMode::Track
            | HeaderMode::Range => {}
        }
        let modes = Rect::new(
            area.x + MODE_BUTTONS.x,
            area.y + MODE_BUTTONS.y,
            MODE_BUTTONS.w,
            MODE_BUTTONS.h,
        );
        self.modes.render(state, modes, painter);
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
