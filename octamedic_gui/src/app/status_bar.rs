use crate::{
    app::state::AppState,
    framework::{
        event::Event,
        painter::{Painter, Pen},
        rect::Rect,
        widget::OctamedicWidget,
        widgets::button::Button,
    },
};

/// The boxes from left to right, in pixels; they fill the row.
const WIDTHS: [i32; 12] = [84, 68, 22, 14, 22, 35, 54, 22, 237, 13, 13, 49];

// the boxes, by position in `WIDTHS`
const SONG_POSITION: usize = 0;
const BLOCK: usize = 1;
const EDIT: usize = 3;
const GAUGE: usize = 6;
const INSTRUMENT: usize = 7;
const INSTRUMENT_NAME: usize = 8;
const SONG: usize = 11;

/// The small boxes in the gauge.
const GAUGE_BOXES: i32 = 8;

/// A row of boxes under the header's widgets: where the song is, the block, the instrument and
/// the song number.
pub struct StatusBar {
    boxes: Vec<Button>,
}

impl StatusBar {
    pub fn new() -> Self {
        // not wired up yet: 12, SP and CHRD show the reference's labels
        let labels = ["", "", "12", "E", "SP", "CHRD", "", "", "", "<", ">", ""];
        let mut boxes: Vec<Button> = labels.into_iter().map(Button::new).collect();
        boxes[9] = Button::new("<").on_click(|state, _| {
            state.prev_instrument();
        });
        boxes[10] = Button::new(">").on_click(|state, _| {
            state.next_instrument();
        });
        return Self { boxes };
    }
}

impl OctamedicWidget for StatusBar {
    fn update(&mut self, state: &mut AppState) {
        let song = state.song();
        let instrument = state.active_instrument;
        let name = song
            .instrument(instrument)
            .map_or(String::new(), |instrument| instrument.name.clone());
        let texts = [
            (
                SONG_POSITION,
                format!(
                    "{:04}/{:04}",
                    state.active_sequence_index + 1,
                    song.sequence().len()
                ),
            ),
            (
                BLOCK,
                format!(
                    "{:03}/{:03}",
                    state.active_pattern.0,
                    song.patterns().len().saturating_sub(1)
                ),
            ),
            (INSTRUMENT, format!("{:02}", instrument.0)),
            (INSTRUMENT_NAME, name),
            (
                SONG,
                format!(
                    "{:02}/{:02}",
                    state.active_song.0 + 1,
                    state.project().song_count()
                ),
            ),
        ];
        for (index, text) in texts {
            self.boxes[index].set_text(text);
        }
        self.boxes[EDIT].set_toggled(state.cursor.edit_mode);
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self
            .boxes
            .iter_mut()
            .any(|button| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        for (index, (button, rect)) in self.boxes.iter_mut().zip(area.columns(&WIDTHS)).enumerate()
        {
            if index == GAUGE {
                render_gauge(rect, painter);
            } else {
                button.render(state, rect, painter);
            }
        }
    }
}

/// A row of small boxes, 7 pixels apart.
fn render_gauge(area: Rect, painter: &mut Painter) {
    let width = area.w / GAUGE_BOXES;
    for small in area.columns(&[width; GAUGE_BOXES as usize]) {
        painter.fill(small, Pen::BLACK);
        painter.frame(small);
    }
}
