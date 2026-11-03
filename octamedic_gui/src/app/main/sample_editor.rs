use std::{ops::Range, sync::Arc};

use ggez::input::keyboard::KeyCode;
use octamedic_core::data::instrument::{
    InstrumentId, InstrumentKind, OctamedicSample, SampleOctaves,
};
use rustuition::{
    font::Font,
    gadget::Gadget,
    gadgets::button::{ContentArea, Triangle},
    painter::{Painter, Pen},
    rect::Rect,
    shape::Direction::{self, Down, Left, Right, Up},
};

use crate::{
    app::{
        fonts,
        main::{wave_view::WaveView, waveform_mouse::WaveformMouse},
        pens,
        prompt::Prompt,
        state::AppState,
    },
    framework::{
        event::{Event, MouseButton},
        keymap::{Action, Key, KeyActions, Keymap},
        widgets::{Button, Textbox, symbols::picture},
    },
};

// from the editor's top left. nothing here does anything yet
const WAVEFORM: Rect = Rect::new(0, 0, 640, 64);
const ZERO_LINE_Y: i32 = 32;
// shows which part of the sample is in view, along the waveform's bottom
const SCROLLBAR: Rect = Rect::new(0, 59, 640, 4);
// under the waveform, across the whole editor
const DIVIDER_Y: i32 = 64;
// the sample's length, top right of the waveform
const LENGTH_RIGHT: i32 = 620;
const LENGTH_Y: i32 = 2;
// in Topaz near the top of the waveform when there's no sample, or an empty one. OctaMED has it
// a little left of the middle
const NO_SAMPLE: &str = "No sample loaded.";
const NO_SAMPLE_POSITION: [i32; 2] = [230, 7];

const ROW_HEIGHT: i32 = 11;
const ROWS: [i32; 5] = [65, 76, 87, 98, 109];
const LABEL_Y: i32 = 3;

// right aligned, the last of their ink at the same place
const RIGHT_LABELS: [(&str, usize); 3] = [("BUFFSIZE:", 0), ("RNGSTART:", 1), ("RNGEND:", 2)];
const RIGHT_LABELS_END: i32 = 68;
// the ones squeezed in before their boxes are in the narrow font
const LABELS: [(&str, i32, usize, Font); 5] = [
    ("PITCH", 523, 0, fonts::NARROW_LABEL),
    ("DEST", 523, 1, fonts::LABEL),
    ("RATE:", 334, 4, fonts::NARROW_LABEL),
    ("DECR:", 411, 4, fonts::NARROW_LABEL),
    ("N:", 480, 4, fonts::LABEL),
];

// value, x, width and row of each sunk in number box
const BUFFSIZE_BOX: usize = 0;
const RANGE_START_BOX: usize = 1;
const RANGE_END_BOX: usize = 2;
const RANGE_DIGITS: usize = 8;
// the most BUFFSIZE takes, from the manual
const MAX_BUFFER_SIZE: usize = 131072;
const BUFFER_PROMPT: &str = "Clear or retain sample? (C/R)";
const BOXES: [(&str, i32, i32, usize); 13] = [
    ("0", 74, 60, 0),
    ("0", 74, 60, 1),
    ("0", 74, 60, 2),
    ("428", 554, 44, 0),
    ("428", 554, 44, 1),
    ("200", 432, 34, 2),
    ("200", 485, 35, 2),
    ("16", 270, 36, 3),
    ("1", 306, 36, 3),
    ("20", 252, 36, 4),
    ("400", 365, 44, 4),
    ("0", 442, 36, 4),
    ("2", 492, 28, 4),
];
// the notes PITCH and DEST are, grey rather than black
const NOTE_BOXES: [(i32, i32, usize); 2] = [(598, 32, 0), (598, 32, 1)];

// label, x, width and row
#[rustfmt::skip]
const BUTTONS: [(&str, i32, i32, usize); 32] = [
    ("PLAY DISPLAY", 134, 92, 0),
    ("PLAY RANGE", 226, 92, 0),
    ("S>CBUFF", 318, 62, 0),
    ("MONITOR", 380, 70, 0),
    ("RECORD", 450, 70, 0),

    ("SHOW RANGE", 134, 92, 1),
    ("SHOW ALL", 226, 92, 1),
    ("CBUFF>S", 318, 62, 1),
    ("R>SY", 380, 37, 1),
    ("DEL CB", 417, 43, 1),
    ("REV", 460, 30, 1),
    ("MIX", 490, 30, 1),

    ("ZOOM IN", 134, 92, 2),
    ("ZOOM OUT", 226, 92, 2),
    ("FREEHAND", 318, 62, 2),
    ("CHG.VOL.", 380, 52, 2),
    ("CHANGE RATE", 522, 116, 2),

    ("CLEAR", 6, 64, 3),
    ("DEL", 70, 64, 3),
    ("RANGE ALL", 134, 64, 3),
    ("FILT", 198, 30, 3),
    ("BOOST", 228, 42, 3),
    ("LOOP ON", 342, 68, 3),
    ("S", 410, 16, 3),
    ("E", 426, 14, 3),

    ("CUT", 6, 64, 4),
    ("COPY", 70, 64, 4),
    ("PASTE", 134, 64, 4),
    ("NOISE", 198, 54, 4),
    ("ECHO", 288, 44, 4),
    ("SAVE IFF", 522, 58, 4),
    (D_PIXELS, 580, 58, 4),
];
// S is on: the loop's start is what the arrows move
const SELECTED: &str = "S";
const ZOOM_IN: &str = "ZOOM IN";
const ZOOM_OUT: &str = "ZOOM OUT";
const SHOW_ALL: &str = "SHOW ALL";
const SHOW_RANGE: &str = "SHOW RANGE";
const RANGE_ALL: &str = "RANGE ALL";
// silences the range
const CLEAR: &str = "CLEAR";
// cuts the range out
const DELETE: &str = "DEL";
// the copy buffer: the range into it, cut out to it, or it put in at the range's start
const CUT: &str = "CUT";
const COPY: &str = "COPY";
const PASTE: &str = "PASTE";
// all of the sample into the copy buffer, or the sample swapped for it
const SAMPLE_TO_BUFFER: &str = "S>CBUFF";
const BUFFER_TO_SAMPLE: &str = "CBUFF>S";
// the range backwards
const REVERSE: &str = "REV";
// the copy buffer mixed in from the range's start, only as far as its end if there's a range
const MIX: &str = "MIX";
// dot pixels: each sample on its own, without the line from the one before
const D_PIXELS: &str = "D.PIXELS";
// the buttons that do something so far, the rest are ghosted
const HOOKED_UP: [&str; 15] = [
    REVERSE,
    MIX,
    ZOOM_IN,
    ZOOM_OUT,
    SHOW_ALL,
    SHOW_RANGE,
    RANGE_ALL,
    CLEAR,
    DELETE,
    CUT,
    COPY,
    PASTE,
    SAMPLE_TO_BUFFER,
    BUFFER_TO_SAMPLE,
    D_PIXELS,
];

// along row 4, after S and E
const LOOP_ARROWS: [i32; 2] = [440, 454];
const LOOP_ARROW_WIDTH: i32 = 14;
const TO_ZERO: [i32; 2] = [468, 494];
const TO_ZERO_WIDTH: i32 = 26;
// an arrow to the nearest zero crossing, left and right
const LEFT_TO_ZERO: &[&str] = &[
    "....##....###.",
    "..####...##.##",
    "######...##.##",
    "..####...##.##",
    "....##....###.",
];
const RIGHT_TO_ZERO: &[&str] = &[
    ".###....##....",
    "##.##...####..",
    "##.##...######",
    "##.##...####..",
    ".###....##....",
];

const OCTAVES: [i32; 2] = [522, 580];
const OCTAVE_WIDTH: i32 = 58;
const OCTAVE_ARROW: [i32; 2] = [9, 5];
const OCTAVE_ARROW_GAP: i32 = 6;

// between CHG.VOL.'s two boxes
const CHANGE_VOLUME_ARROW: Rect = Rect::new(466, 87, 19, 11);
const CHANGE_VOLUME_ARROW_SIZE: [i32; 2] = [8, 7];

fn active_sample(state: &AppState) -> Option<&OctamedicSample> {
    let instrument = state.song().instrument(state.active_instrument)?;
    return instrument.sample().map(|sample| &**sample);
}

fn sample_length(state: &AppState) -> usize {
    return active_sample(state).map_or(0, |sample| sample.data.len());
}

// every sample from `anchor` to `sample`, whichever way round
fn spanning(anchor: usize, sample: usize) -> Range<usize> {
    return anchor.min(sample)..anchor.max(sample) + 1;
}

// shift clicking keeps the far end of the range and stretches it to `sample`: a sample before it
// moves the start, one after it the end, and one inside it leaves it as it is
fn stretch_anchor(range: Range<usize>, sample: usize) -> Option<usize> {
    if range.is_empty() || sample >= range.end {
        return Some(range.start);
    }
    if sample < range.start {
        return Some(range.end - 1);
    }
    return None;
}

// a new size typed into BUFFSIZE asks whether to clear the sample or keep it
fn ask_buffer_size(state: &mut AppState, typed: &str, length: usize) {
    let Ok(size) = typed.parse::<usize>() else {
        return;
    };
    let size = size.min(MAX_BUFFER_SIZE);
    if size == length {
        return;
    }
    state.open_prompt(Prompt::new(BUFFER_PROMPT, move |state, key| match key {
        KeyCode::C => resize_buffer(state, size, true),
        KeyCode::R => resize_buffer(state, size, false),
        _ => {}
    }));
}

// swaps the active instrument's sample for `edit` of it. one without a sound of its own is edited
// as an empty sample, synths are left alone
fn edit_sample(state: &mut AppState, edit: impl FnOnce(&OctamedicSample) -> OctamedicSample) {
    state.edit_active_instrument(|instrument| {
        let edited = match &instrument.kind {
            InstrumentKind::Sample(sample) => edit(sample),
            InstrumentKind::Empty => edit(&sample_of(Vec::new())),
            _ => return,
        };
        instrument.kind = InstrumentKind::Sample(Arc::new(edited));
    });
}

// a buffer `size` long for the active instrument, silent with `clear` or else the sample there
// was, cut short or padded with silence. an instrument without a sound of its own gets one, but
// synths are left alone
fn resize_buffer(state: &mut AppState, size: usize, clear: bool) {
    edit_sample(state, |sample| {
        let resized = sample.resized(size);
        if clear {
            return resized.cleared(0..size);
        }
        return resized;
    });
}

fn sample_of(data: Vec<i8>) -> OctamedicSample {
    return OctamedicSample {
        data,
        repeat: None,
        octaves: SampleOctaves::One,
    };
}

// what's being typed into `textbox`, while it is
fn typed_number(textbox: &Textbox) -> Option<usize> {
    if !textbox.is_focused() {
        return None;
    }
    return textbox.text().parse().ok();
}

// OctaMED draws the waveform, the zero line and the scroll bar on bitplanes of their own, so
// where they cross the pens combine: the waveform's white and the bar's pink make blush
fn overlap(pens: &[Pen]) -> Pen {
    return Pen(pens.iter().fold(0, |bits, pen| bits | pen.0));
}

// the row of the sample each column shows, one sample a column across the width
fn sample_rows(data: &[i8], waveform: Rect) -> Vec<i32> {
    if data.is_empty() {
        return Vec::new();
    }
    let width = waveform.w as usize;
    return (0..width)
        .map(|column| sample_y(data[column * data.len() / width], waveform))
        .collect();
}

// each column's top and bottom row. joined up like OctaMED, the line to the next sample is split
// between the two columns: down to halfway in this one, then on from just past halfway in the
// next, so the halves touch diagonally. with dots it's just the sample
fn column_spans(rows: &[i32], dots: bool) -> Vec<(i32, i32)> {
    return (0..rows.len())
        .map(|column| {
            let row = rows[column];
            let mut span = (row, row);
            let mut reach = |to: i32| span = (span.0.min(to), span.1.max(to));
            if dots {
                return span;
            }
            if let Some(&from) = column.checked_sub(1).map(|previous| &rows[previous]) {
                let half = from + (row - from) / 2;
                reach(if from == row {
                    row
                } else {
                    half + (row - from).signum()
                });
            }
            if let Some(&to) = rows.get(column + 1) {
                reach(row + (to - row) / 2);
            }
            return span;
        })
        .collect();
}

// a sample's value as a line. OctaMED draws them upside down, -128 at the top and 127 at the
// bottom
fn sample_y(value: i8, waveform: Rect) -> i32 {
    let middle = waveform.y + ZERO_LINE_Y;
    return middle + value as i32 * ZERO_LINE_Y / 128;
}

// the bar along the bottom spans the part in view
fn scrollbar(waveform: Rect, visible: &Range<usize>, length: usize) -> Rect {
    let track = SCROLLBAR.within(waveform);
    if length == 0 {
        return track;
    }
    let get_x = |at: usize| track.x + (at * track.w as usize / length) as i32;
    let (left, right) = (get_x(visible.start), get_x(visible.end));
    return Rect::new(left, track.y, (right - left).max(1), track.h);
}

fn render_waveform(
    painter: &mut Painter,
    waveform: Rect,
    sample: Option<&OctamedicSample>,
    visible: Range<usize>,
    // the columns the range covers
    range: Range<i32>,
    dots: bool,
) {
    let data = sample.map_or(&[][..], |sample| &sample.data);
    let bar = scrollbar(waveform, &visible, data.len());
    let zero = Rect::new(waveform.x, waveform.y + ZERO_LINE_Y, waveform.w, 1);
    let ranged = Rect::new(
        waveform.x + range.start,
        waveform.y,
        range.len() as i32,
        waveform.h,
    );
    // the range is highlighted, everything in its columns
    painter.fill(waveform, Pen::BACKGROUND);
    painter.fill(ranged, pens::highlight(Pen::BACKGROUND));
    for (layer, pen) in [(zero, Pen::SHADOW), (bar, pens::PINK)] {
        painter.fill(layer, pen);
        painter.fill(layer.intersection(ranged), pens::highlight(pen));
    }
    let data = data.get(visible).unwrap_or(&[]);
    let rows = sample_rows(data, waveform);
    for (column, (top, bottom)) in column_spans(&rows, dots).into_iter().enumerate() {
        let column = column as i32;
        let line = Rect::new(waveform.x + column, top, 1, bottom - top + 1).intersection(waveform);
        let ink = |pen| {
            if range.contains(&column) {
                return pens::highlight(pen);
            }
            return pen;
        };
        painter.fill(line, ink(Pen::SHINE));
        for (layer, layer_pen) in [(zero, Pen::SHADOW), (bar, pens::PINK)] {
            let crossing = line.intersection(layer);
            if crossing.h > 0 {
                painter.fill(crossing, ink(overlap(&[Pen::SHINE, layer_pen])));
            }
        }
    }
}

fn row_rect(x: i32, width: i32, row: usize) -> Rect {
    return Rect::new(x, ROWS[row], width, ROW_HEIGHT);
}

// the label font, or the narrow one when it wouldn't fit inside the frame
fn label_font(label: &str, width: i32) -> Font {
    let inside = width - 4;
    if fonts::LABEL.width(label) <= inside {
        return fonts::LABEL;
    }
    return fonts::NARROW_LABEL;
}

// OCT then an arrow pointing `direction`
fn octave_label(direction: Direction) -> impl Fn(ContentArea, Pen, &mut Painter) {
    return move |area, pen, painter| {
        let [width, height] = OCTAVE_ARROW;
        let text_width = fonts::LABEL.width("OCT");
        let inside = area.inside;
        let x = inside.x + (inside.w - text_width - OCTAVE_ARROW_GAP - width) / 2;
        let y = inside.y + (inside.h - height + 1) / 2;
        painter.text_in_font(x, y, "OCT", pen, fonts::LABEL);
        let arrow = Rect::new(x + text_width + OCTAVE_ARROW_GAP, y, width, height);
        painter.triangle(arrow, Some(pen), None, direction);
    };
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SampleEditorAction {
    ScrollEarlier,
    ScrollLater,
}

impl Action for SampleEditorAction {}

pub struct SampleEditor {
    keys: Keymap<SampleEditorAction>,
    boxes: Vec<(Textbox, Rect)>,
    note_boxes: Vec<(Button, Rect)>,
    buttons: Vec<(Button, Rect)>,
    view: WaveView,
    // the instrument the view is of, so picking another starts it over
    viewed: InstrumentId,
    dot_pixels: bool,
    // the range is the samples from its start up to, not including, its end
    range_start: usize,
    range_end: usize,
    // while dragging out the range, the sample that stays put
    range_anchor: Option<usize>,
    // Enter was pressed in BUFFSIZE, rather than it being left some other way
    buffer_size_entered: bool,
    mouse: WaveformMouse,
}

impl SampleEditor {
    pub fn new() -> Self {
        let boxes = BOXES
            .into_iter()
            .enumerate()
            .map(|(index, (text, x, width, row))| {
                let mut textbox = Textbox::new()
                    .font(fonts::INTERFACE)
                    .pens(Pen::BACKGROUND, Pen::SHINE, Pen::FILL, pens::RED)
                    .padding(1)
                    .max_length(RANGE_DIGITS);
                textbox.set_text(text);
                textbox
                    .set_enabled([BUFFSIZE_BOX, RANGE_START_BOX, RANGE_END_BOX].contains(&index));
                return (textbox, row_rect(x, width, row));
            })
            .collect();
        let note_boxes = NOTE_BOXES
            .into_iter()
            .map(|(x, width, row)| {
                let note = Button::new("C-2").font(fonts::INTERFACE);
                return (note, row_rect(x, width, row));
            })
            .collect();
        let labelled = BUTTONS.into_iter().map(|(label, x, width, row)| {
            let mut button = Button::new(label)
                .font(label_font(label, width))
                .enabled(HOOKED_UP.contains(&label));
            button.set_toggled(label == SELECTED);
            return (button, row_rect(x, width, row));
        });
        let loop_arrows = LOOP_ARROWS
            .into_iter()
            .zip([Left, Right])
            .map(|(x, direction)| {
                let button = Button::new(Triangle::arrow(direction));
                return (button, row_rect(x, LOOP_ARROW_WIDTH, 3));
            });
        let to_zero = TO_ZERO
            .into_iter()
            .zip([LEFT_TO_ZERO, RIGHT_TO_ZERO])
            .map(|(x, rows)| (Button::new(picture(rows)), row_rect(x, TO_ZERO_WIDTH, 3)));
        let octaves = OCTAVES.into_iter().zip([Up, Down]).map(|(x, direction)| {
            let button = Button::new(octave_label(direction));
            return (button, row_rect(x, OCTAVE_WIDTH, 3));
        });
        let pictures = loop_arrows
            .chain(to_zero)
            .chain(octaves)
            .map(|(button, rect)| (button.enabled(false), rect));
        let buttons = labelled.chain(pictures).collect();
        let keys = Keymap::new("sample editor")
            .bind(
                Key::plain(KeyCode::Left),
                SampleEditorAction::ScrollEarlier,
                "Scroll left",
            )
            .bind(
                Key::plain(KeyCode::Right),
                SampleEditorAction::ScrollLater,
                "Scroll right",
            );
        return Self {
            keys,
            boxes,
            note_boxes,
            buttons,
            view: WaveView::default(),
            viewed: InstrumentId(0),
            dot_pixels: false,
            range_start: 0,
            range_end: 0,
            range_anchor: None,
            buffer_size_entered: false,
            mouse: WaveformMouse::default(),
        };
    }
}

impl KeyActions for SampleEditor {
    type Action = SampleEditorAction;

    fn keymap(&self) -> &Keymap<SampleEditorAction> {
        return &self.keys;
    }

    fn perform(&mut self, action: SampleEditorAction, _state: &mut AppState) {
        self.view.scroll(action == SampleEditorAction::ScrollLater);
    }

    fn describe(action: SampleEditorAction) -> &'static str {
        return match action {
            SampleEditorAction::ScrollEarlier => "Show half a view further towards the start",
            SampleEditorAction::ScrollLater => "Show half a view further towards the end",
        };
    }
}

impl SampleEditor {
    fn button(&mut self, label: &str) -> &mut Button {
        let index = BUTTONS.iter().position(|&(name, ..)| name == label);
        return &mut self.buttons[index.expect("a button of the sample editor's")].0;
    }

    fn clicked(&mut self, label: &str) -> bool {
        return self.button(label).clicked().is_some();
    }
}

impl SampleEditor {
    fn range(&self) -> Range<usize> {
        return self.range_start.min(self.range_end)..self.range_start.max(self.range_end);
    }

    fn set_range(&mut self, range: Range<usize>) {
        (self.range_start, self.range_end) = (range.start, range.end);
    }

    fn keep_range_inside(&mut self, length: usize) {
        self.range_start = self.range_start.min(length);
        self.range_end = self.range_end.min(length);
    }

    fn sample_at(&self, column: i32) -> usize {
        return self.view.sample_at(column, WAVEFORM.w);
    }

    // a right press starts a range there, like OctaMED, or with shift stretches the one there is
    fn press_waveform(&mut self, button: MouseButton, length: usize) {
        if button != MouseButton::Right || length == 0 {
            return;
        }
        let sample = self.sample_at(self.mouse.press_column());
        self.range_anchor = if self.mouse.shifted() {
            stretch_anchor(self.range(), sample)
        } else {
            Some(sample)
        };
    }

    // the buttons that change the sample or the copy buffer
    fn edit_range(&mut self, state: &mut AppState) {
        let range = self.range();
        let data = active_sample(state).map_or(&[][..], |sample| &sample.data[..]);
        let ranged = data.get(range.clone()).unwrap_or(&[]).to_vec();
        let whole = data.to_vec();
        if self.clicked(CLEAR) && !ranged.is_empty() {
            edit_sample(state, |sample| sample.cleared(range.clone()));
        }
        if self.clicked(COPY) && !ranged.is_empty() {
            state.copy_buffer = ranged.clone();
        }
        let cut = self.clicked(CUT) && !ranged.is_empty();
        if cut {
            state.copy_buffer = ranged.clone();
        }
        let delete = self.clicked(DELETE);
        if (delete || cut) && !ranged.is_empty() {
            edit_sample(state, |sample| sample.without(range.clone()));
            self.view.resized(sample_length(state));
            self.set_range(range.start..range.start);
        }
        if self.clicked(PASTE) && !state.copy_buffer.is_empty() {
            let buffer = state.copy_buffer.clone();
            edit_sample(state, |sample| sample.with_inserted(range.start, &buffer));
            self.view.resized(sample_length(state));
            self.set_range(range.start..range.start + buffer.len());
        }
        if self.clicked(SAMPLE_TO_BUFFER) && !whole.is_empty() {
            state.copy_buffer = whole;
            state.notify("Sample copied");
        }
        if self.clicked(REVERSE) && !ranged.is_empty() {
            edit_sample(state, |sample| sample.reversed(range.clone()));
        }
        if self.clicked(MIX) && !state.copy_buffer.is_empty() {
            let mut buffer = state.copy_buffer.clone();
            if !range.is_empty() {
                buffer.truncate(range.len());
            }
            edit_sample(state, |sample| sample.mixed(range.start, &buffer));
            self.view.resized(sample_length(state));
        }
        if self.clicked(BUFFER_TO_SAMPLE) && !state.copy_buffer.is_empty() {
            let buffer = state.copy_buffer.clone();
            edit_sample(state, |_| sample_of(buffer));
            self.set_range(0..0);
        }
    }

    fn drag_range(&mut self) {
        let Some(anchor) = self.range_anchor else {
            return;
        };
        self.set_range(spanning(anchor, self.sample_at(self.mouse.column())));
        if self.mouse.held().is_none() {
            self.range_anchor = None;
        }
    }

    // typing in RNGSTART or RNGEND moves that end, and otherwise they show the range
    fn follow_range_boxes(&mut self, length: usize) {
        if let Some(start) = typed_number(&self.boxes[RANGE_START_BOX].0) {
            self.range_start = start;
        }
        if let Some(end) = typed_number(&self.boxes[RANGE_END_BOX].0) {
            self.range_end = end;
        }
        self.keep_range_inside(length);
        for (index, value) in [
            (RANGE_START_BOX, self.range_start),
            (RANGE_END_BOX, self.range_end),
        ] {
            let textbox = &mut self.boxes[index].0;
            if !textbox.is_focused() {
                textbox.set_text(value.to_string());
            }
        }
    }
}

impl Gadget<AppState, Event> for SampleEditor {
    fn update(&mut self, state: &mut AppState) {
        if state.active_instrument != self.viewed {
            self.viewed = state.active_instrument;
            self.view = WaveView::default();
            self.set_range(0..0);
            self.range_anchor = None;
        }
        // the sample can have been changed elsewhere since, by BUFFSIZE's prompt or a load
        self.keep_range_inside(sample_length(state));
        // before the length's looked at, as DEL changes it
        self.edit_range(state);
        let length = sample_length(state);
        self.view.follow(length);
        if let Some(button) = self.mouse.take_press() {
            self.press_waveform(button, length);
        }
        self.mouse.take_release();
        self.drag_range();
        if self.clicked(RANGE_ALL) {
            self.set_range(0..length);
        }
        self.follow_range_boxes(length);
        if self.clicked(ZOOM_IN) {
            self.view.zoom_in();
        }
        if self.clicked(ZOOM_OUT) {
            self.view.zoom_out();
        }
        // like OctaMED, SHOW ALL lets go of the range too, down to its start
        if self.clicked(SHOW_ALL) {
            self.view.show_all();
            self.range_end = self.range_start;
        }
        if self.clicked(SHOW_RANGE) {
            let range = self.range();
            self.view.show(range);
        }
        if self.clicked(D_PIXELS) {
            self.dot_pixels = !self.dot_pixels;
        }
        let dot_pixels = self.dot_pixels;
        self.button(D_PIXELS).set_toggled(dot_pixels);
        if std::mem::take(&mut self.buffer_size_entered) {
            ask_buffer_size(state, self.boxes[BUFFSIZE_BOX].0.text(), length);
        }
        let buffer_size = &mut self.boxes[BUFFSIZE_BOX].0;
        if !buffer_size.is_focused() {
            buffer_size.set_text(length.to_string());
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        let enter = matches!(
            event,
            Event::Key {
                key: KeyCode::Return | KeyCode::NumpadEnter,
                ..
            }
        );
        if enter && self.boxes[BUFFSIZE_BOX].0.is_focused() {
            self.buffer_size_entered = true;
        }
        // every box sees every event, so a click anywhere else lets go of the one being typed in
        let mut typed = false;
        for (textbox, _) in &mut self.boxes {
            typed |= textbox.handle_event(state, event);
        }
        return typed
            || self.mouse.handle_event(event, state.mouse_mods)
            || self.dispatch(state, event)
            || self
                .buttons
                .iter_mut()
                .any(|(button, _)| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::FILL);
        let waveform = WAVEFORM.within(area);
        self.mouse.place(waveform);
        let visible = self.view.visible();
        let range = self.range();
        let range_columns = self.view.column_of(range.start, WAVEFORM.w)
            ..self.view.column_of(range.end, WAVEFORM.w);
        render_waveform(
            painter,
            waveform,
            active_sample(state),
            visible,
            range_columns,
            self.dot_pixels,
        );
        if sample_length(state) == 0 {
            let [x, y] = NO_SAMPLE_POSITION;
            painter.text(waveform.x + x, waveform.y + y, NO_SAMPLE, Pen::SHINE);
        }
        let divider = Rect::new(area.x, area.y + DIVIDER_Y, area.w, 1);
        painter.fill(divider, Pen::SHINE);

        let shown = self.view.visible().len().to_string();
        let x = area.x + LENGTH_RIGHT - fonts::INTERFACE.width(&shown);
        painter.text_in_font(x, area.y + LENGTH_Y, &shown, pens::PINK, fonts::INTERFACE);

        for (label, row) in RIGHT_LABELS {
            let x = area.x + RIGHT_LABELS_END - fonts::LABEL.width(label);
            let y = area.y + ROWS[row] + LABEL_Y;
            painter.text_in_font(x, y, label, Pen::BACKGROUND, fonts::LABEL);
        }
        for (label, x, row, font) in LABELS {
            let y = area.y + ROWS[row] + LABEL_Y;
            painter.text_in_font(area.x + x, y, label, Pen::BACKGROUND, font);
        }
        for (textbox, rect) in &mut self.boxes {
            let rect = rect.within(area);
            textbox.render(state, rect, painter);
            painter.frame_inverted(rect, 1);
        }
        for (note, rect) in &mut self.note_boxes {
            let rect = rect.within(area);
            note.render(state, rect, painter);
            painter.frame_inverted(rect, 1);
        }
        for (button, rect) in &mut self.buttons {
            button.render(state, rect.within(area), painter);
        }

        let slot = CHANGE_VOLUME_ARROW.within(area);
        let [width, height] = CHANGE_VOLUME_ARROW_SIZE;
        let arrow = Rect::new(
            slot.x + (slot.w - width) / 2,
            slot.y + (slot.h - height + 1) / 2,
            width,
            height,
        );
        painter.triangle(arrow, Some(Pen::BACKGROUND), None, Right);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use octamedic_core::{
        data::{
            instrument::{InstrumentKind, OctamedicInstrument, SampleOctaves},
            project::OctamedicProject,
        },
        engine::handle::EngineHandle,
    };
    use rustuition::{
        painter::{CHAR_HEIGHT, text_width},
        target::Framebuffer,
    };

    use super::*;

    const AREA: Rect = Rect::new(0, 0, 640, 120);

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    #[test]
    fn everything_fits_the_editor() {
        let state = state();
        let mut editor = SampleEditor::new();
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        let rects = editor.buttons.iter().map(|(_, rect)| *rect);
        for rect in rects.chain(editor.boxes.iter().map(|(_, rect)| *rect)) {
            assert!(
                rect.right() <= AREA.w && rect.bottom() <= AREA.h,
                "{rect:?}"
            );
        }
    }

    #[test]
    fn labels_fit_their_buttons() {
        for (label, _, width, _) in BUTTONS {
            let font = label_font(label, width);
            assert!(font.width(label) <= width - 2, "{label}");
        }
    }

    #[test]
    fn labels_stop_before_their_boxes() {
        for (label, x, row, font) in LABELS {
            let end = x + font.width(label);
            let next = BOXES
                .iter()
                .filter(|&&(_, box_x, _, box_row)| box_row == row && box_x >= x)
                .map(|&(_, box_x, _, _)| box_x)
                .min();
            assert!(next.is_none_or(|next| end <= next), "{label}");
        }
    }

    #[test]
    fn drawn_upside_down() {
        let waveform = Rect::new(0, 0, 640, 64);
        assert_eq!(sample_y(-128, waveform), 0);
        assert_eq!(sample_y(0, waveform), ZERO_LINE_Y);
        assert!(sample_y(100, waveform) > ZERO_LINE_Y);
    }

    #[test]
    fn crossings_mix_their_pens() {
        assert_eq!(overlap(&[Pen::SHINE, pens::PINK]), pens::BLUSH);
        assert_eq!(overlap(&[Pen::SHINE, Pen::SHADOW]), Pen::FILL);
    }

    #[test]
    fn lines_split_between_the_columns() {
        // 10 down to 16: 10 to 13 in the first column, 14 to 16 in the second
        assert_eq!(column_spans(&[10, 16], false), [(10, 13), (14, 16)]);
        // a step of one touches diagonally
        assert_eq!(column_spans(&[10, 11], false), [(10, 10), (11, 11)]);
        // up works the same way
        assert_eq!(column_spans(&[16, 10], false), [(13, 16), (10, 12)]);
        // a column takes the second half of the line in and the first half of the line out
        assert_eq!(column_spans(&[0, 10, 0], false)[1], (5, 10));
        assert_eq!(column_spans(&[10, 16], true), [(10, 10), (16, 16)]);
    }

    #[test]
    fn a_sample_a_column() {
        let waveform = Rect::new(0, 0, 3, 64);
        let rows = sample_rows(&[0, 64, -64, 1, 2, 3], waveform);
        assert_eq!(rows, [32, 16, 32]);
        assert!(sample_rows(&[], waveform).is_empty());
    }

    fn with_sample(state: &mut AppState, length: usize) {
        let mut instrument = OctamedicInstrument::new("sample");
        instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data: vec![0; length],
            repeat: None,
            octaves: SampleOctaves::One,
        }));
        let active = state.active_instrument;
        state.song_mut().set_instrument(active, Some(instrument));
    }

    fn click(editor: &mut SampleEditor, state: &mut AppState, label: &str) {
        let index = BUTTONS
            .iter()
            .position(|&(name, ..)| name == label)
            .unwrap();
        let area = editor.buttons[index].0.area();
        let pos = [area.x + area.w / 2, area.y + area.h / 2];
        let down = Event::MouseDown {
            pos,
            button: crate::framework::event::MouseButton::Left,
        };
        let up = Event::MouseUp {
            pos,
            button: crate::framework::event::MouseButton::Left,
        };
        editor.handle_event(state, &down);
        editor.handle_event(state, &up);
        editor.update(state);
    }

    #[test]
    fn zoom_buttons() {
        let mut state = state();
        with_sample(&mut state, 1000);
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        click(&mut editor, &mut state, ZOOM_IN);
        click(&mut editor, &mut state, ZOOM_IN);
        assert_eq!(editor.view.visible(), 375..625);
        click(&mut editor, &mut state, ZOOM_OUT);
        assert_eq!(editor.view.visible(), 250..750);
        click(&mut editor, &mut state, SHOW_ALL);
        assert_eq!(editor.view.visible(), 0..1000);
        // BUFFSIZE stays the whole sample
        click(&mut editor, &mut state, ZOOM_IN);
        assert_eq!(editor.boxes[0].0.text(), "1000");
        // another instrument starts over, even one as long
        state.set_instrument(2);
        with_sample(&mut state, 1000);
        editor.update(&mut state);
        assert_eq!(editor.view.visible(), 0..1000);
    }

    #[test]
    fn shift_clicking_stretches_the_range() {
        let mut state = state();
        with_sample(&mut state, 1000);
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        (editor.range_start, editor.range_end) = (300, 600);
        state.mouse_mods = ggez::input::keyboard::KeyMods::SHIFT;
        let waveform = WAVEFORM.within(AREA);
        let mut click_column = |state: &mut AppState, column: i32| {
            let pos = [waveform.x + column, waveform.y + 10];
            let button = crate::framework::event::MouseButton::Right;
            editor.handle_event(state, &Event::MouseDown { pos, button });
            editor.handle_event(state, &Event::MouseUp { pos, button });
            editor.update(state);
            return (editor.range_start, editor.range_end);
        };
        // a column is 1000 / 640 samples
        assert_eq!(click_column(&mut state, 64), (100, 600));
        assert_eq!(click_column(&mut state, 448), (100, 701));
        assert_eq!(click_column(&mut state, 320), (100, 701));
        // without shift a click is just the sample under it
        state.mouse_mods = ggez::input::keyboard::KeyMods::NONE;
        assert_eq!(click_column(&mut state, 320), (500, 501));
        // the left button doesn't select
        let pos = [waveform.x + 64, waveform.y + 10];
        let button = crate::framework::event::MouseButton::Left;
        editor.handle_event(&mut state, &Event::MouseDown { pos, button });
        editor.handle_event(&mut state, &Event::MouseUp { pos, button });
        editor.update(&mut state);
        assert_eq!(editor.range(), 500..501);
    }

    #[test]
    fn dragging_out_a_range() {
        let mut state = state();
        with_sample(&mut state, 1000);
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        let waveform = WAVEFORM.within(AREA);
        let at = |column: i32| [waveform.x + column, waveform.y + 10];
        let button = crate::framework::event::MouseButton::Right;
        editor.handle_event(
            &mut state,
            &Event::MouseDown {
                pos: at(320),
                button,
            },
        );
        editor.handle_event(&mut state, &Event::MouseMove { pos: at(64) });
        editor.update(&mut state);
        assert_eq!(editor.range(), 100..501);
        // past the edge it stops at the first sample in view, and letting go keeps it
        editor.handle_event(&mut state, &Event::MouseMove { pos: [-50, 0] });
        editor.handle_event(
            &mut state,
            &Event::MouseUp {
                pos: [-50, 0],
                button,
            },
        );
        editor.update(&mut state);
        assert_eq!(editor.range(), 0..501);
        assert_eq!(editor.boxes[RANGE_START_BOX].0.text(), "0");
        assert_eq!(editor.boxes[RANGE_END_BOX].0.text(), "501");
        editor.handle_event(&mut state, &Event::MouseMove { pos: at(600) });
        editor.update(&mut state);
        assert_eq!(editor.range(), 0..501);
    }

    #[test]
    fn typing_in_the_range_boxes() {
        let mut state = state();
        with_sample(&mut state, 1000);
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        let end_box = editor.boxes[RANGE_END_BOX].1.within(AREA);
        let pos = [end_box.x + 2, end_box.y + 2];
        let button = crate::framework::event::MouseButton::Left;
        editor.handle_event(&mut state, &Event::MouseDown { pos, button });
        editor.handle_event(&mut state, &Event::MouseUp { pos, button });
        let textbox = &mut editor.boxes[RANGE_END_BOX].0;
        assert!(textbox.is_focused());
        textbox.set_text("250");
        editor.update(&mut state);
        assert_eq!(editor.range(), 0..250);
        // past the end of the sample is the end
        editor.boxes[RANGE_END_BOX].0.set_text("5000");
        editor.update(&mut state);
        assert_eq!(editor.range(), 0..1000);
        // the piano doesn't get what's typed
        assert!(editor.handle_event(&mut state, &Event::text('5')));
    }

    #[test]
    fn range_buttons() {
        let mut state = state();
        state.edit_active_instrument(|instrument| {
            instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
                data: vec![1; 1000],
                repeat: None,
                octaves: SampleOctaves::One,
            }));
        });
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        click(&mut editor, &mut state, RANGE_ALL);
        assert_eq!(editor.range(), 0..1000);

        // SHOW ALL lets go of the range
        click(&mut editor, &mut state, SHOW_ALL);
        assert_eq!(editor.range(), 0..0);

        editor.set_range(100..200);
        click(&mut editor, &mut state, SHOW_RANGE);
        assert_eq!(editor.view.visible(), 100..200);
        click(&mut editor, &mut state, CLEAR);
        let data = &active_sample(&state).unwrap().data;
        assert_eq!((data[99], data[100], data[199], data[200]), (1, 0, 0, 1));

        click(&mut editor, &mut state, DELETE);
        assert_eq!(sample_length(&state), 900);
        // the view stays put, what was after the range moving up into it
        assert_eq!(editor.view.visible(), 100..200);
        assert!(
            active_sample(&state)
                .unwrap()
                .data
                .iter()
                .all(|&value| value == 1)
        );
        assert_eq!(editor.range(), 100..100);
        assert_eq!(editor.boxes[BUFFSIZE_BOX].0.text(), "900");
        // with nothing in range they leave it be
        click(&mut editor, &mut state, DELETE);
        assert_eq!(sample_length(&state), 900);
    }

    #[test]
    fn buffer_size_asks_to_clear_or_retain() {
        let mut state = state();
        state.edit_active_instrument(|instrument| {
            instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
                data: vec![1; 1000],
                repeat: None,
                octaves: SampleOctaves::One,
            }));
        });
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        let key = |key| Event::Key {
            key,
            mods: ggez::input::keyboard::KeyMods::NONE,
            repeat: false,
        };
        let mut enter_size = |state: &mut AppState, size: &str, finish: KeyCode| {
            let rect = editor.boxes[BUFFSIZE_BOX].1.within(AREA);
            let pos = [rect.x + 2, rect.y + 2];
            let button = crate::framework::event::MouseButton::Left;
            editor.handle_event(state, &Event::MouseDown { pos, button });
            editor.handle_event(state, &Event::MouseUp { pos, button });
            editor.boxes[BUFFSIZE_BOX].0.set_text(size);
            editor.handle_event(state, &key(finish));
            editor.update(state);
        };

        // leaving it any other way than Enter changes nothing
        enter_size(&mut state, "1500", KeyCode::Escape);
        assert!(state.prompt.is_none());
        enter_size(&mut state, "1500", KeyCode::Return);
        assert_eq!(
            state.prompt.as_ref().map(|prompt| prompt.text()),
            Some(BUFFER_PROMPT)
        );
        crate::app::prompt::handle_event(&mut state, &key(KeyCode::R));
        let data = &active_sample(&state).unwrap().data;
        assert_eq!((data.len(), data[999], data[1000]), (1500, 1, 0));

        enter_size(&mut state, "500", KeyCode::Return);
        crate::app::prompt::handle_event(&mut state, &key(KeyCode::C));
        let data = &active_sample(&state).unwrap().data;
        assert!(data.len() == 500 && data.iter().all(|&value| value == 0));

        // escape answers by leaving it, and it's never past the limit
        enter_size(&mut state, "999999", KeyCode::Return);
        crate::app::prompt::handle_event(&mut state, &key(KeyCode::Escape));
        assert_eq!(sample_length(&state), 500);
        enter_size(&mut state, "999999", KeyCode::NumpadEnter);
        crate::app::prompt::handle_event(&mut state, &key(KeyCode::R));
        assert_eq!(sample_length(&state), MAX_BUFFER_SIZE);
        editor.update(&mut state);
        assert_eq!(editor.boxes[BUFFSIZE_BOX].0.text(), "131072");
    }

    #[test]
    fn says_when_theres_no_sample() {
        let mut state = state();
        let mut editor = SampleEditor::new();
        let mut written_at_the_top = |state: &mut AppState| {
            editor.update(state);
            let mut screen = Framebuffer::new(AREA.w, AREA.h);
            editor.render(state, AREA, &mut Painter::new(&mut screen, &state.palette));
            // nothing else is drawn up there but the black
            let black = screen.pixel(0, 0);
            let [x, y] = NO_SAMPLE_POSITION;
            let text = Rect::new(x, y, text_width(NO_SAMPLE), CHAR_HEIGHT);
            return (text.y..text.bottom())
                .flat_map(|y| (text.x..text.right()).map(move |x| (x, y)))
                .any(|(x, y)| screen.pixel(x, y) != black);
        };
        assert!(written_at_the_top(&mut state));
        with_sample(&mut state, 0);
        assert!(written_at_the_top(&mut state));
        with_sample(&mut state, 1000);
        assert!(!written_at_the_top(&mut state));
    }

    #[test]
    fn copy_buffer() {
        let mut state = state();
        state.edit_active_instrument(|instrument| {
            instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
                data: (0..100).collect(),
                repeat: None,
                octaves: SampleOctaves::One,
            }));
        });
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        let data = |state: &AppState| active_sample(state).unwrap().data.clone();

        editor.set_range(10..13);
        click(&mut editor, &mut state, COPY);
        assert_eq!(state.copy_buffer, [10, 11, 12]);
        assert_eq!(data(&state).len(), 100);

        editor.set_range(0..2);
        click(&mut editor, &mut state, CUT);
        assert_eq!(state.copy_buffer, [0, 1]);
        assert_eq!(data(&state)[..3], [2, 3, 4]);
        assert_eq!(editor.range(), 0..0);

        // pasted in at the range's start, and then it's the range
        editor.set_range(3..5);
        click(&mut editor, &mut state, PASTE);
        assert_eq!(data(&state)[..6], [2, 3, 4, 0, 1, 5]);
        assert_eq!(editor.range(), 3..5);
        assert_eq!(sample_length(&state), 100);

        click(&mut editor, &mut state, SAMPLE_TO_BUFFER);
        assert_eq!(state.copy_buffer.len(), 100);
        assert_eq!(state.notification.as_deref(), Some("Sample copied"));
        state.copy_buffer = vec![7; 5];
        click(&mut editor, &mut state, BUFFER_TO_SAMPLE);
        assert_eq!(data(&state), [7; 5]);
        state.copy_buffer = vec![7; 5];

        // mixed in from the range's start, only over the range when there is one
        editor.set_range(4..4);
        click(&mut editor, &mut state, MIX);
        assert_eq!(data(&state), [7, 7, 7, 7, 7, 3, 3, 3, 3]);
        editor.set_range(1..3);
        state.copy_buffer = vec![-7; 5];
        click(&mut editor, &mut state, MIX);
        assert_eq!(data(&state)[..4], [7, 0, 0, 7]);
        editor.set_range(2..6);
        click(&mut editor, &mut state, REVERSE);
        assert_eq!(data(&state), [7, 0, 3, 7, 7, 0, 3, 3, 3]);

        // an empty slot gets a sample from what's pasted
        state.set_instrument(2);
        editor.update(&mut state);
        click(&mut editor, &mut state, PASTE);
        assert_eq!(data(&state), [-7; 5]);
    }

    #[test]
    fn d_pixels_switches_dots_on_and_off() {
        let mut state = state();
        with_sample(&mut state, 1000);
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        click(&mut editor, &mut state, D_PIXELS);
        assert!(editor.dot_pixels && editor.button(D_PIXELS).is_toggled());
        click(&mut editor, &mut state, D_PIXELS);
        assert!(!editor.dot_pixels);
    }

    #[test]
    fn arrow_keys_scroll_the_view() {
        let mut state = state();
        with_sample(&mut state, 1000);
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        editor.view.zoom_in();
        editor.view.zoom_in();
        let mut press = |state: &mut AppState, key| {
            let event = Event::Key {
                key,
                mods: ggez::input::keyboard::KeyMods::NONE,
                repeat: false,
            };
            assert!(editor.handle_event(state, &event));
            return editor.view.visible();
        };
        assert_eq!(press(&mut state, KeyCode::Left), 250..500);
        assert_eq!(press(&mut state, KeyCode::Right), 375..625);
    }

    #[test]
    fn the_bar_spans_whats_in_view() {
        let waveform = Rect::new(0, 0, 640, 64);
        assert_eq!(scrollbar(waveform, &(0..1000), 1000).w, 640);
        let half = scrollbar(waveform, &(250..750), 1000);
        assert_eq!((half.x, half.w), (160, 320));
    }

    #[test]
    fn buffsize_follows_the_sample() {
        let mut state = state();
        let mut instrument = OctamedicInstrument::new("sample");
        instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data: vec![0; 22040],
            repeat: None,
            octaves: SampleOctaves::One,
        }));
        let active = state.active_instrument;
        state.song_mut().set_instrument(active, Some(instrument));
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        assert_eq!(editor.boxes[0].0.text(), "22040");
    }
}
