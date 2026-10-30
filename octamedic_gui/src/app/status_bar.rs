use octamedic_util::{base32, core_extensions::Boxed, primitives::Flip};

use crate::{
    app::state::AppState,
    framework::{
        event::{Event, MouseButton},
        font::Font,
        painter::{Align, Painter, Pen},
        rect::Rect,
        shape::Direction,
        widget::OctamedicWidget,
        widgets::{
            button::{Button, ButtonHandler, Triangle},
            grid::Grid,
        },
    },
};

/// The boxes from left to right, in pixels; they fill the row.
const WIDTHS: [i32; 12] = [84, 68, 22, 14, 22, 35, 56, 20, 237, 13, 13, 49];

// the boxes, by position in `WIDTHS`
const SONG_POSITION: usize = 0;
const BLOCK: usize = 1;
const EDIT: usize = 3;
const SP: usize = 4;
const CHRD: usize = 5;
const GAUGE: usize = 6;
const INSTRUMENT: usize = 7;
const INSTRUMENT_NAME: usize = 8;
const SONG: usize = 11;

/// The gauge's small boxes, one per track, 7 pixels by 6 lines each. They start a line above
/// the rest of the row.
const GAUGE_COLUMNS: usize = 8;
const GAUGE_ROWS: usize = 2;

/// A row of boxes under the header's widgets: where the song is, the block, the instrument and
/// the song number.
pub struct StatusBar {
    boxes: Vec<Button>,
    gauge: Grid<GaugeBox>,
}

impl StatusBar {
    pub fn new() -> Self {
        // not wired up yet: 12, SP and CHRD show the reference's labels
        let labels = ["", "", "12", "E", "SP", "CHRD", "", "", "", "", "", ""];
        // labels start at the left, as in OctaMED
        let mut boxes: Vec<Button> = labels
            .into_iter()
            .map(|label| Button::new(label).align(Align::Left).font(Font::Interface))
            .collect();

        boxes[9] = Button::new(Triangle::arrow(Direction::Left));
        boxes[10] = Button::new(Triangle::arrow(Direction::Right));
        boxes[SONG] = Button::new("").align(Align::Right).font(Font::Interface);
        let flags: [(usize, &str, i32, Option<Box<dyn ButtonHandler>>); 3] = [
            (
                EDIT,
                "E",
                3,
                Some((|state: &mut AppState, _| state.cursor.edit_mode.flip()).boxed()),
            ),
            (SP, "SP", 2, None),
            (CHRD, "CHRD", 2, None),
        ];
        // the flags are in short capitals, a few pixels further in than the numbers
        for (index, label, padding, handler) in flags {
            let mut button = Button::new(label)
                .align(Align::Left)
                .font(Font::Small)
                .padding_xy(padding, 1);
            if let Some(handler) = handler {
                button = button.on_click(handler);
            }
            boxes[index] = button;
        }
        boxes[INSTRUMENT] = Button::new("")
            .align(Align::Left)
            .font(Font::Interface)
            .on_click(|state, click| {
                let advance = click.button == MouseButton::Left;
                match click.char_index {
                    Some(0) => {
                        state.set_instrument(state.active_instrument.0 + base32::BASE);
                    } //todo jump to instrument base32
                    Some(1) => {
                        if advance {
                            state.next_instrument();
                        } else {
                            state.prev_instrument();
                        }
                    }
                    _ => (),
                }
            });

        let gauge_boxes = (0..GAUGE_COLUMNS * GAUGE_ROWS).map(|_| GaugeBox).collect();
        return Self {
            boxes,
            gauge: Grid::new(GAUGE_COLUMNS, GAUGE_ROWS, gauge_boxes),
        };
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
                    "{:04}⁄{:04}",
                    state.active_sequence_index + 1,
                    song.sequence().len()
                ),
            ),
            (
                BLOCK,
                format!(
                    "{:03}⁄{:03}",
                    state.active_pattern.0,
                    song.patterns().len().saturating_sub(1)
                ),
            ),
            (INSTRUMENT, base32::format(instrument.0 as u32, 2, '0')),
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
        return self.gauge.handle_event(state, event)
            || self
                .boxes
                .iter_mut()
                .any(|button| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        for (index, (button, rect)) in self.boxes.iter_mut().zip(area.columns(&WIDTHS)).enumerate()
        {
            if index == GAUGE {
                let gauge = Rect::new(rect.x, rect.y - 1, rect.w, rect.h + 1);
                self.gauge.render(state, gauge, painter);
            } else {
                button.render(state, rect, painter);
            }
        }
    }
}

/// One of the gauge's small boxes.
struct GaugeBox;

impl OctamedicWidget for GaugeBox {
    fn render(&mut self, _state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::BLACK);
        painter.frame_inverted(area, 1);
    }
}
