use rustuition::{
    gadget::Gadget,
    gadgets::button::Triangle,
    painter::{Painter, Pen, text_width},
    rect::Rect,
    shape::Direction::{Left, Right},
};

use crate::{
    app::{fonts, pens, state::AppState},
    framework::{
        event::Event,
        widgets::{Button, Textbox},
    },
};

// the active instrument's sample length, 0 if it isn't a plain sample
fn sample_length(state: &AppState) -> usize {
    let instrument = state.song().instrument(state.active_instrument);
    return instrument
        .and_then(|instrument| instrument.sample())
        .map_or(0, |sample| sample.data.len());
}

// from the screen's top left. nothing here does anything yet, and the waveform display itself is
// still empty, to be filled in later. the length reads in the waveform box's own top right corner
// rather than a separate strip above it, which would just be another status line eating space
const LENGTH_MARGIN: [i32; 2] = [4, 2];
const WAVEFORM: Rect = Rect::new(0, 0, 640, 61);
// a position/zoom scrollbar normally lives under the waveform; it's not wired up yet, so it's
// drawn as a translucent bar overlaid on the waveform's own bottom edge rather than a stub that
// would otherwise just push the already-tight viewport shorter. inset so it sits inside the
// waveform's sunken bevel instead of smearing across it
const SCROLLBAR: Rect = Rect::new(2, 55, 636, 4);
const SCROLLBAR_OPACITY: f32 = 0.5;

const ROW_HEIGHT: i32 = 11;
// row tops, pitched 12px apart so there's a 1px gap between them
const ROWS: [i32; 5] = [61, 73, 85, 97, 109];

// label, then its value box, both from the screen's top left. measured off a 1:1 WinUAE capture
// of the real screen (confirmed 640x512, PAL interlaced - double the normal 640x256 screen, which
// is why it needed checking against known Amiga resolutions rather than guessed proportions)
const LABELS: [(&str, Rect); 11] = [
    ("BUFFSIZE:", Rect::new(0, ROWS[0], 80, ROW_HEIGHT)),
    ("RNGSTART:", Rect::new(0, ROWS[1], 75, ROW_HEIGHT)),
    ("RNGEND:", Rect::new(0, ROWS[2], 75, ROW_HEIGHT)),
    ("BOOST", Rect::new(228, ROWS[3], 42, ROW_HEIGHT)),
    ("NOISE", Rect::new(228, ROWS[4], 36, ROW_HEIGHT)),
    ("CHG.VOL:", Rect::new(380, ROWS[2], 53, ROW_HEIGHT)),
    ("RATE:", Rect::new(410, ROWS[4], 28, ROW_HEIGHT)),
    ("DECR:", Rect::new(462, ROWS[4], 26, ROW_HEIGHT)),
    ("N:", Rect::new(504, ROWS[4], 9, ROW_HEIGHT)),
    ("PITCH", Rect::new(520, ROWS[0], 35, ROW_HEIGHT)),
    ("DEST", Rect::new(519, ROWS[1], 36, ROW_HEIGHT)),
];

// default text, then its box, from the screen's top left
const BOXES: [(&str, Rect); 13] = [
    ("0", Rect::new(80, ROWS[0], 55, ROW_HEIGHT)),
    ("0", Rect::new(75, ROWS[1], 60, ROW_HEIGHT)),
    ("0", Rect::new(75, ROWS[2], 60, ROW_HEIGHT)),
    ("16", Rect::new(270, ROWS[3], 36, ROW_HEIGHT)),
    ("1", Rect::new(306, ROWS[3], 36, ROW_HEIGHT)),
    ("20", Rect::new(264, ROWS[4], 78, ROW_HEIGHT)),
    ("200", Rect::new(433, ROWS[2], 33, ROW_HEIGHT)),
    ("200", Rect::new(486, ROWS[2], 35, ROW_HEIGHT)),
    ("400", Rect::new(438, ROWS[4], 24, ROW_HEIGHT)),
    ("0", Rect::new(488, ROWS[4], 16, ROW_HEIGHT)),
    ("2", Rect::new(513, ROWS[4], 8, ROW_HEIGHT)),
    ("428", Rect::new(555, ROWS[0], 44, ROW_HEIGHT)),
    ("428", Rect::new(555, ROWS[1], 43, ROW_HEIGHT)),
];

// label, then its rect, from the screen's top left; OctaMED's order. Rows 0-2 share one column
// grid (confirmed: their label+box widths match to the pixel); row 3 and 4 measured to their own,
// wider grid for CLEAR/DEL/RANGE ALL, which row 4 (CUT/COPY/PASTE etc) then mirrors
#[rustfmt::skip]
const BUTTONS: [(&str, Rect); 33] = [
    ("PLAY DISPLAY", Rect::new(135, ROWS[0], 91, ROW_HEIGHT)),
    ("PLAY RANGE",   Rect::new(226, ROWS[0], 88, ROW_HEIGHT)),
    ("MONITOR",      Rect::new(384, ROWS[0], 66, ROW_HEIGHT)),
    ("RECORD",       Rect::new(450, ROWS[0], 70, ROW_HEIGHT)),
    ("(C-2)",        Rect::new(599, ROWS[0], 41, ROW_HEIGHT)),

    ("SHOW RANGE",   Rect::new(135, ROWS[1], 91, ROW_HEIGHT)),
    ("SHOW ALL",     Rect::new(226, ROWS[1], 88, ROW_HEIGHT)),
    ("R>SY",         Rect::new(384, ROWS[1], 40, ROW_HEIGHT)),
    ("DEL CB",       Rect::new(424, ROWS[1], 40, ROW_HEIGHT)),
    ("REV",          Rect::new(464, ROWS[1], 30, ROW_HEIGHT)),
    ("MIX",          Rect::new(494, ROWS[1], 25, ROW_HEIGHT)),
    ("(C-2)",        Rect::new(598, ROWS[1], 42, ROW_HEIGHT)),

    ("ZOOM IN",      Rect::new(135, ROWS[2], 91, ROW_HEIGHT)),
    ("ZOOM OUT",     Rect::new(226, ROWS[2], 92, ROW_HEIGHT)),
    ("CHANGE RATE",  Rect::new(521, ROWS[2], 119, ROW_HEIGHT)),

    ("CLEAR",        Rect::new(0, ROWS[3], 70, ROW_HEIGHT)),
    ("DEL",          Rect::new(70, ROWS[3], 64, ROW_HEIGHT)),
    ("RANGE ALL",    Rect::new(134, ROWS[3], 64, ROW_HEIGHT)),
    ("FILT",         Rect::new(198, ROWS[3], 30, ROW_HEIGHT)),
    ("LOOP ON",      Rect::new(342, ROWS[3], 68, ROW_HEIGHT)),
    ("S",            Rect::new(410, ROWS[3], 16, ROW_HEIGHT)),
    ("E",            Rect::new(426, ROWS[3], 14, ROW_HEIGHT)),
    ("OCT UP",       Rect::new(521, ROWS[3], 59, ROW_HEIGHT)),
    ("OCT DOWN",     Rect::new(580, ROWS[3], 60, ROW_HEIGHT)),

    ("CUT",          Rect::new(0, ROWS[4], 70, ROW_HEIGHT)),
    ("COPY",         Rect::new(70, ROWS[4], 64, ROW_HEIGHT)),
    ("PASTE",        Rect::new(134, ROWS[4], 94, ROW_HEIGHT)),
    ("SAVE IFF",     Rect::new(521, ROWS[4], 59, ROW_HEIGHT)),
    ("D.PIXELS",     Rect::new(580, ROWS[4], 60, ROW_HEIGHT)),

    ("S>CBUFF",      Rect::new(314, ROWS[0], 70, ROW_HEIGHT)),
    ("CBUFF>S",      Rect::new(314, ROWS[1], 70, ROW_HEIGHT)),
    ("FREEHAND",     Rect::new(318, ROWS[2], 62, ROW_HEIGHT)),
    ("ECHO",         Rect::new(342, ROWS[4], 68, ROW_HEIGHT)),
];

// LABEL and NARROW_LABEL only cover the letters OctaMED's status row needs, no J, Z or '>', so
// these fall back to the fuller mode-button font instead
const FULL_ALPHABET: [&str; 6] = [
    "BUFFSIZE:",
    "ZOOM IN",
    "ZOOM OUT",
    "S>CBUFF",
    "CBUFF>S",
    "R>SY",
];

// the chg.vol arrow, then the 4 loop nudge buttons: step left/right, jump to start/end
fn arrows() -> [(Rect, Triangle); 5] {
    return [
        (
            Rect::new(466, ROWS[2], 20, ROW_HEIGHT),
            Triangle::arrow(Right),
        ),
        (
            Rect::new(440, ROWS[3], 14, ROW_HEIGHT),
            Triangle::arrow(Left),
        ),
        (
            Rect::new(454, ROWS[3], 14, ROW_HEIGHT),
            Triangle::arrow(Right),
        ),
        (
            Rect::new(468, ROWS[3], 26, ROW_HEIGHT),
            Triangle::arrow(Left).with_bar(),
        ),
        (
            Rect::new(494, ROWS[3], 27, ROW_HEIGHT),
            Triangle::arrow(Right).with_bar(),
        ),
    ];
}

/// The SMPED screen: so far just where everything goes. The waveform itself, and the buttons'
/// actions, are still to come.
pub struct SampleEditor {
    labelled_boxes: Vec<(Textbox, Rect)>,
    buttons: Vec<(Button, Rect)>,
    arrows: Vec<(Button, Rect)>,
}

impl SampleEditor {
    pub fn new() -> Self {
        let labelled_boxes = BOXES
            .into_iter()
            .map(|(text, rect)| {
                let mut textbox = Textbox::new()
                    .font(fonts::INTERFACE)
                    .pens(Pen::BACKGROUND, Pen::SHINE, Pen::FILL, pens::RED)
                    .padding(1);
                textbox.set_text(text);
                textbox.set_enabled(false);
                return (textbox, rect);
            })
            .collect();
        let buttons = BUTTONS
            .into_iter()
            .map(|(label, rect)| {
                let font = if FULL_ALPHABET.contains(&label) {
                    fonts::PANEL
                } else {
                    fonts::NARROW_LABEL
                };
                return (Button::new(label).font(font).enabled(false), rect);
            })
            .collect();
        let arrows = arrows()
            .into_iter()
            .map(|(rect, triangle)| (Button::new(triangle).enabled(false), rect))
            .collect();
        return Self {
            labelled_boxes,
            buttons,
            arrows,
        };
    }
}

impl Gadget<AppState, Event> for SampleEditor {
    fn update(&mut self, state: &mut AppState) {
        // BOXES[0] is BUFFSIZE's; not editable yet, so it just tracks the loaded sample's length
        self.labelled_boxes[0]
            .0
            .set_text(sample_length(state).to_string());
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::FILL);
        painter.frame(area, 1);

        let waveform = WAVEFORM.within(area);
        painter.fill(waveform, Pen::BACKGROUND);
        painter.frame_inverted(waveform, 1);
        // the scroll/zoom bar, overlaid on the waveform rather than its own row so it blends with
        // whatever's drawn there instead of just being a stub that eats into a cramped viewport
        painter.fill_translucent(SCROLLBAR.within(area), pens::ROSE, SCROLLBAR_OPACITY);

        let length = sample_length(state).to_string();
        let [margin_x, margin_y] = LENGTH_MARGIN;
        let x = waveform.right() - text_width(&length) - margin_x;
        painter.text(x, waveform.y + margin_y, &length, pens::RED);

        for (label, rect) in LABELS {
            let rect = rect.within(area);
            let font = if FULL_ALPHABET.contains(&label) {
                fonts::PANEL
            } else {
                fonts::LABEL
            };
            let y = rect.y + (rect.h - font.height() + 1) / 2;
            painter.text_in_font(rect.x, y, label, Pen::BACKGROUND, font);
        }
        for (textbox, rect) in &mut self.labelled_boxes {
            let rect = rect.within(area);
            textbox.render(state, rect, painter);
            painter.frame_inverted(rect, 1);
        }
        for (button, rect) in self.buttons.iter_mut().chain(&mut self.arrows) {
            button.render(state, rect.within(area), painter);
        }
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::{painter::Painter, target::Framebuffer};

    use super::*;

    const AREA: Rect = Rect::new(0, 0, 640, 120);

    #[test]
    fn renders_without_overlap_past_the_area() {
        let state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut editor = SampleEditor::new();
        let mut screen = Framebuffer::new(640, 120);
        let mut painter = Painter::new(&mut screen, &state.palette);
        editor.render(&state, AREA, &mut painter);
        for (_, rect) in &editor.buttons {
            assert!(rect.right() <= AREA.w);
            assert!(rect.bottom() <= AREA.h);
        }
    }

    fn sample_instrument(bytes: usize) -> octamedic_core::data::instrument::OctamedicInstrument {
        use std::sync::Arc;

        use octamedic_core::data::instrument::{
            InstrumentKind, OctamedicInstrument, OctamedicSample, SampleOctaves,
        };

        let mut instrument = OctamedicInstrument::new("sample");
        instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data: vec![0; bytes],
            repeat: None,
            octaves: SampleOctaves::One,
        }));
        return instrument;
    }

    #[test]
    fn buffsize_and_length_track_the_loaded_sample() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let active = state.active_instrument;
        state
            .song_mut()
            .set_instrument(active, Some(sample_instrument(22040)));
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        assert_eq!(editor.labelled_boxes[0].0.text(), "22040");
    }
}
