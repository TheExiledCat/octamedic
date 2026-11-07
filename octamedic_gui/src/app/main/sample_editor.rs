use std::{
    ops::{Range, RangeInclusive},
    sync::Arc,
};

use ggez::input::keyboard::KeyCode;
use octamedic_core::data::{
    instrument::{InstrumentId, InstrumentKind, OctamedicSample, SampleOctaves},
    note::OctamedicNote,
};
use rustuition::{
    font::Font,
    gadget::Gadget,
    gadgets::button::{ContentArea, Triangle},
    painter::{Painter, Pen},
    rect::Rect,
    shape::{
        Direction::{self, Down, Left, Right, Up},
        Sides,
    },
};

use crate::{
    app::{
        fonts,
        main::{wave_view::WaveView, waveform_mouse::WaveformMouse},
        pens,
        prompt::Prompt,
        state::{AppState, InstrumentSaveType},
    },
    framework::{
        event::{Event, MouseButton},
        keymap::{Action, Key, KeyActions, Keymap},
        widgets::{self, Button, Textbox, symbols::picture},
    },
};

// all from the editor's top left
const WAVEFORM: Rect = Rect::new(0, 0, 640, 64);
const ZERO_LINE_Y: i32 = WAVEFORM.h / 2;
const SCROLLBAR: Rect = Rect::new(0, WAVEFORM.h - 5, WAVEFORM.w, 4);
const DIVIDER_Y: i32 = WAVEFORM.bottom();
const LENGTH_END: [i32; 2] = [620, 2];
// in Topaz near the top of the waveform when there's no sample, or an empty one. OctaMED has it
// a little left of the middle
const NO_SAMPLE: &str = "No sample loaded.";
const NO_SAMPLE_POSITION: [i32; 2] = [230, 7];

const FIRST_ROW_Y: i32 = DIVIDER_Y + 1;
const ROW_HEIGHT: i32 = 11;
const LABEL_Y: i32 = 3;

const fn on_row(row: i32, x: i32, width: i32) -> Rect {
    return Rect::new(x, FIRST_ROW_Y + row * ROW_HEIGHT, width, ROW_HEIGHT);
}

const fn label_on(row: i32, x: i32) -> [i32; 2] {
    return [x, on_row(row, x, 0).y + LABEL_Y];
}

// right aligned, the last of their ink at the same place
const RIGHT_LABELS: [(&str, [i32; 2]); 3] = [
    ("BUFFSIZE:", label_on(0, 68)),
    ("RNGSTART:", label_on(1, 68)),
    ("RNGEND:", label_on(2, 68)),
];
// the ones squeezed in before their boxes are in the narrow font
const LABELS: [(&str, [i32; 2], Font); 5] = [
    ("PITCH", label_on(0, 523), fonts::NARROW_LABEL),
    ("DEST", label_on(1, 523), fonts::LABEL),
    ("RATE:", label_on(4, 334), fonts::NARROW_LABEL),
    ("DECR:", label_on(4, 411), fonts::NARROW_LABEL),
    ("N:", label_on(4, 480), fonts::LABEL),
];

// the value each sunk in number box starts with, and where it is
const BOXES: [(&str, Rect); 13] = [
    ("0", on_row(0, 74, 60)),
    ("0", on_row(1, 74, 60)),
    ("0", on_row(2, 74, 60)),
    ("428", on_row(0, 554, 44)),
    ("428", on_row(1, 554, 44)),
    ("200", on_row(2, 432, 34)),
    ("200", on_row(2, 485, 35)),
    ("16", on_row(3, 270, 36)),
    ("1", on_row(3, 306, 36)),
    ("20", on_row(4, 252, 36)),
    ("400", on_row(4, 365, 44)),
    ("0", on_row(4, 442, 36)),
    ("2", on_row(4, 492, 28)),
];
// indices into BOXES
const BUFFSIZE_BOX: usize = 0;
const RANGE_START_BOX: usize = 1;
const RANGE_END_BOX: usize = 2;
// CHG.VOL.'s start and end, in percent
const VOLUME_START_BOX: usize = 5;
const VOLUME_END_BOX: usize = 6;
const VOLUME_DIGITS: usize = 3;
const VOLUMES: RangeInclusive<u32> = 1..=999;
const VOLUME_OUT_OF_RANGE: &str = "Volume has to be in range 1 - 999";
const RANGE_DIGITS: usize = 8;
// the most BUFFSIZE takes, from the manual
const MAX_SAMPLE_SIZE: usize = 131072;
const MULTI_OCTAVE_LENGTH: &str = "Can't change a multi-octave sample's length";
const RESIZE_PROMPT: &str = "Clear or retain sample? (C/R)";
// the notes PITCH and DEST are, grey rather than black
const NOTE_BOXES: [Rect; 2] = [on_row(0, 598, 32), on_row(1, 598, 32)];

const PLAY_DISPLAY: &str = "PLAY DISPLAY";
const PLAY_RANGE: &str = "PLAY RANGE";
// C-2, what the PITCH box shows until it's hooked up
const PLAY_NOTE: OctamedicNote = OctamedicNote { value: 13 };
// S is on: the loop's start is what the arrows move
const SELECTED: &str = "S";
const ZOOM_IN: &str = "ZOOM IN";
const ZOOM_OUT: &str = "ZOOM OUT";
const SHOW_ALL: &str = "SHOW ALL";
const SHOW_RANGE: &str = "SHOW RANGE";
const RANGE_ALL: &str = "RANGE ALL";
const CLEAR: &str = "CLEAR";
const DELETE: &str = "DEL";
const CUT: &str = "CUT";
const COPY: &str = "COPY";
const PASTE: &str = "PASTE";
const SAMPLE_TO_CLIPBOARD: &str = "S>CBUFF";
const CLIPBOARD_TO_SAMPLE: &str = "CBUFF>S";
const CLEAR_CLIPBOARD: &str = "DEL CB";
const REVERSE: &str = "REV";
// fades the range from VOLUME_START_BOX's percent to VOLUME_END_BOX's
const CHANGE_VOLUME: &str = "CHG.VOL.";
// the clipboard mixed in from the range's start, only as far as its end if there's a range
const MIX: &str = "MIX";
const FREEHAND: &str = "FREEHAND";
// what FREEHAND zooms to, a sample for every column
const FREEHAND_SAMPLES: usize = WAVEFORM.w as usize;
// dot pixels: each sample on its own, without the line from the one before
const D_PIXELS: &str = "D.PIXELS";
// only picks the format the Files panel saves instruments in, nothing's saved here. its label
// changes as it cycles, this is the one it starts with and how it's found in BUTTONS
const SAVE_TYPE: &str = "SAVE RAW";

#[rustfmt::skip]
const BUTTONS: [(&str, Rect); 32] = [
    (PLAY_DISPLAY, on_row(0, 134, 92)),
    (PLAY_RANGE, on_row(0, 226, 92)),
    (SAMPLE_TO_CLIPBOARD, on_row(0, 318, 62)),
    ("MONITOR", on_row(0, 380, 70)),
    ("RECORD", on_row(0, 450, 70)),

    (SHOW_RANGE, on_row(1, 134, 92)),
    (SHOW_ALL, on_row(1, 226, 92)),
    (CLIPBOARD_TO_SAMPLE, on_row(1, 318, 62)),
    ("R>SY", on_row(1, 380, 37)),
    (CLEAR_CLIPBOARD, on_row(1, 417, 43)),
    (REVERSE, on_row(1, 460, 30)),
    (MIX, on_row(1, 490, 30)),

    (ZOOM_IN, on_row(2, 134, 92)),
    (ZOOM_OUT, on_row(2, 226, 92)),
    (FREEHAND, on_row(2, 318, 62)),
    (CHANGE_VOLUME, on_row(2, 380, 52)),
    ("CHANGE RATE", on_row(2, 522, 116)),

    (CLEAR, on_row(3, 6, 64)),
    (DELETE, on_row(3, 70, 64)),
    (RANGE_ALL, on_row(3, 134, 64)),
    ("FILT", on_row(3, 198, 30)),
    ("BOOST", on_row(3, 228, 42)),
    ("LOOP ON", on_row(3, 342, 68)),
    (SELECTED, on_row(3, 410, 16)),
    ("E", on_row(3, 426, 14)),

    (CUT, on_row(4, 6, 64)),
    (COPY, on_row(4, 70, 64)),
    (PASTE, on_row(4, 134, 64)),
    ("NOISE", on_row(4, 198, 54)),
    ("ECHO", on_row(4, 288, 44)),
    (SAVE_TYPE, on_row(4, 522, 58)),
    (D_PIXELS, on_row(4, 580, 58)),
];
// the buttons that do something so far, the rest are ghosted
const HOOKED_UP: &[&str] = &[
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
    SAMPLE_TO_CLIPBOARD,
    CLIPBOARD_TO_SAMPLE,
    D_PIXELS,
    FREEHAND,
    SAVE_TYPE,
    PLAY_DISPLAY,
    PLAY_RANGE,
    CLEAR_CLIPBOARD,
    CHANGE_VOLUME,
];

const LOOP_ARROWS: [Rect; 2] = [on_row(3, 440, 14), on_row(3, 454, 14)];
const TO_ZERO: [Rect; 2] = [on_row(3, 468, 26), on_row(3, 494, 26)];
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

const OCTAVES: [Rect; 2] = [on_row(3, 522, 58), on_row(3, 580, 58)];
const OCTAVE_ARROW: [i32; 2] = [9, 5];
const OCTAVE_ARROW_GAP: i32 = 6;

// between CHG.VOL.'s two boxes
const CHANGE_VOLUME_ARROW: Rect = on_row(2, 466, 19);
const CHANGE_VOLUME_ARROW_SIZE: [i32; 2] = [8, 7];

fn active_sample(state: &AppState) -> Option<&OctamedicSample> {
    let instrument = state.song().instrument(state.active_instrument)?;
    return instrument.sample().map(|sample| &**sample);
}

fn sample_length(state: &AppState) -> usize {
    return active_sample(state).map_or(0, |sample| sample.data.len());
}

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

fn ask_sample_size(state: &mut AppState, typed: &str, length: usize) {
    let Ok(size) = typed.parse::<usize>() else {
        return;
    };
    let size = size.min(MAX_SAMPLE_SIZE);
    if size == length {
        return;
    }
    state.open_prompt(Prompt::new(RESIZE_PROMPT, move |state, key| match key {
        KeyCode::C => resize_sample(state, size, true),
        KeyCode::R => resize_sample(state, size, false),
        _ => {}
    }));
}

// an instrument without a sound of its own is edited as an empty sample, synths are left alone
// a multi-octave sample's copies are laid out by its length and its loop is in the shortest
// copy, so an edit that changes its length is turned down
// TODO: REV over a range that crosses from one octave copy into another moves data between the
// copies and scrambles them too, even though the length stays the same
fn edit_sample(state: &mut AppState, edit: impl FnOnce(&OctamedicSample) -> OctamedicSample) {
    let instrument = state.song().instrument(state.active_instrument);
    let edited = match instrument.map(|instrument| &instrument.kind) {
        Some(InstrumentKind::Sample(sample)) => {
            let edited = edit(sample);
            let resized = edited.data.len() != sample.data.len();
            if resized && edited.octaves.copies() > 1 {
                state.notify(MULTI_OCTAVE_LENGTH);
                return;
            }
            edited
        }
        None | Some(InstrumentKind::Empty) => edit(&sample_of(Vec::new())),
        _ => return,
    };
    state.edit_active_instrument(|instrument| {
        instrument.kind = InstrumentKind::Sample(Arc::new(edited));
    });
}

// an instrument without a sound of its own gets one, synths are left alone
fn resize_sample(state: &mut AppState, size: usize, clear: bool) {
    edit_sample(state, |sample| {
        let resized = sample.resized(size);
        if clear {
            return resized.cleared(0..size);
        }
        return resized;
    });
}

fn can_play(state: &AppState, part: &Range<usize>) -> bool {
    let length = active_sample(state).map_or(0, |sample| sample.data.len());
    return !part.is_empty() && part.end <= length;
}

// once, without the loop. the engine plays its own copy of the instrument so the play position
// follows it
fn play_part(state: &AppState, part: Range<usize>) {
    if can_play(state, &part) {
        state
            .engine
            .play_sample(state.active_instrument, part, PLAY_NOTE);
    }
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

// the value a row on the waveform stands for, the other way round from sample_y
fn row_value(row: i32) -> i8 {
    let value = (row - ZERO_LINE_Y) * 128 / ZERO_LINE_Y;
    return value.clamp(i8::MIN as i32, i8::MAX as i32) as i8;
}

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

fn save_type_label(save_type: InstrumentSaveType) -> &'static str {
    return match save_type {
        InstrumentSaveType::Raw => "SAVE RAW",
        InstrumentSaveType::Iff => "SAVE IFF",
        InstrumentSaveType::Wav => "SAVE WAV",
    };
}

// the label font, or the narrow one when it wouldn't fit inside the frame
fn label_font(label: &str, width: i32) -> Font {
    let inside = width - 4;
    if fonts::LABEL.width(label) <= inside {
        return fonts::LABEL;
    }
    return fonts::NARROW_LABEL;
}

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
    free_hand: bool,
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
            .map(|(index, (text, rect))| {
                let volume = [VOLUME_START_BOX, VOLUME_END_BOX].contains(&index);
                let digits = if volume { VOLUME_DIGITS } else { RANGE_DIGITS };
                let mut textbox = widgets::textbox()
                    .font(fonts::INTERFACE)
                    .padding(1)
                    .max_length(digits);
                textbox.set_text(text);
                textbox.set_enabled(
                    volume || [BUFFSIZE_BOX, RANGE_START_BOX, RANGE_END_BOX].contains(&index),
                );
                return (textbox, rect);
            })
            .collect();
        let note_boxes = NOTE_BOXES
            .into_iter()
            .map(|rect| (Button::new("C-2").font(fonts::INTERFACE), rect))
            .collect();
        let labelled = BUTTONS.into_iter().map(|(label, rect)| {
            let mut button = Button::new(label)
                .font(label_font(label, rect.w))
                .enabled(HOOKED_UP.contains(&label));
            button.set_toggled(label == SELECTED);
            return (button, rect);
        });
        let loop_arrows = LOOP_ARROWS
            .into_iter()
            .zip([Left, Right])
            .map(|(rect, direction)| (Button::new(Triangle::arrow(direction)), rect));
        let to_zero = TO_ZERO
            .into_iter()
            .zip([LEFT_TO_ZERO, RIGHT_TO_ZERO])
            .map(|(rect, rows)| (Button::new(picture(rows)), rect));
        let octaves = OCTAVES
            .into_iter()
            .zip([Up, Down])
            .map(|(rect, direction)| (Button::new(octave_label(direction)), rect));
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
            free_hand: false,
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

    fn press_waveform(&mut self, button: MouseButton, sample_length: usize) {
        let pos = self.mouse.press_pos();
        let index = self.sample_at(pos.x);
        if button != MouseButton::Right || sample_length == 0 {
            return;
        }
        self.range_anchor = if self.mouse.shifted() {
            stretch_anchor(self.range(), index)
        } else {
            Some(index)
        };
    }
    // only the sample under the mouse each frame, so a fast drag leaves gaps like OctaMED's
    fn draw_waveform(&mut self, state: &mut AppState) {
        if let Some(button) = self.mouse.held()
            && self.free_hand
            && button == MouseButton::Left
        {
            let pos = self.mouse.pos();
            let index = self.sample_at(pos.x);
            let value = row_value(pos.y);
            // held still it'd copy the whole sample every frame for nothing
            let current = active_sample(state).and_then(|sample| sample.data.get(index));
            if current.is_some_and(|&current| current != value) {
                edit_sample(state, |sample| sample.with_value(index, value));
            }
        }
    }
    fn edit_range(&mut self, state: &mut AppState) {
        let range = self.range();
        let data = active_sample(state).map_or(&[][..], |sample| &sample.data[..]);
        let ranged = data.get(range.clone()).unwrap_or(&[]).to_vec();
        let whole = data.to_vec();
        let length = whole.len();
        if self.clicked(CLEAR) && !ranged.is_empty() {
            edit_sample(state, |sample| sample.cleared(range.clone()));
        }
        if self.clicked(COPY) && !ranged.is_empty() {
            state.clipboard.sample = ranged.clone();
        }
        let cut = self.clicked(CUT) && !ranged.is_empty();
        if cut {
            state.clipboard.sample = ranged.clone();
        }
        let delete = self.clicked(DELETE);
        if (delete || cut) && !ranged.is_empty() {
            edit_sample(state, |sample| sample.without(range.clone()));
            self.view.resized(sample_length(state));
            self.set_range(range.start..range.start);
        }
        if self.clicked(PASTE) && !state.clipboard.sample.is_empty() {
            let clip = state.clipboard.sample.clone();
            edit_sample(state, |sample| sample.with_inserted(range.start, &clip));
            self.view.resized(sample_length(state));
            self.set_range(range.start..range.start + clip.len());
        }
        if self.clicked(SAMPLE_TO_CLIPBOARD) && !whole.is_empty() {
            state.clipboard.sample = whole;
            state.notify("Sample copied");
        }
        if self.clicked(CLEAR_CLIPBOARD) {
            state.clipboard.sample.clear();
        }

        if self.clicked(REVERSE) && !ranged.is_empty() {
            edit_sample(state, |sample| sample.reversed(range.clone()));
        }
        // without a range it's just the selected sample, at the start percentage
        let changed = match range.is_empty() {
            true => range.start..range.start + 1,
            false => range.clone(),
        };
        if self.clicked(CHANGE_VOLUME) && changed.end <= length {
            let percent = |index: usize| {
                let typed = self.boxes[index].0.text().parse::<u32>().ok();
                return typed.filter(|percent| VOLUMES.contains(percent));
            };
            match (percent(VOLUME_START_BOX), percent(VOLUME_END_BOX)) {
                (Some(from), Some(to)) => edit_sample(state, |sample| {
                    sample.with_volume_changed(changed.clone(), from, to)
                }),
                _ => state.notify(VOLUME_OUT_OF_RANGE),
            }
        }
        if self.clicked(MIX) && !state.clipboard.sample.is_empty() {
            let mut clip = state.clipboard.sample.clone();
            if !range.is_empty() {
                clip.truncate(range.len());
            }
            edit_sample(state, |sample| sample.mixed(range.start, &clip));
            self.view.resized(sample_length(state));
            state.notify("Sample mixed");
        }
        if self.clicked(CLIPBOARD_TO_SAMPLE) && !state.clipboard.sample.is_empty() {
            let clip = state.clipboard.sample.clone();
            edit_sample(state, |_| sample_of(clip));
            self.set_range(0..0);
        }
    }

    fn drag_range(&mut self) {
        let Some(anchor) = self.range_anchor else {
            return;
        };
        self.set_range(spanning(anchor, self.sample_at(self.mouse.pos().x)));
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
        self.draw_waveform(state);
        if self.clicked(RANGE_ALL) {
            self.set_range(0..length);
        }
        self.follow_range_boxes(length);
        let shown = self.view.visible().len();
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
        if self.view.visible().len() > shown {
            self.free_hand = false;
        }
        if self.clicked(PLAY_DISPLAY) {
            play_part(state, self.view.visible());
        }
        if self.clicked(PLAY_RANGE) {
            play_part(state, self.range());
        }
        if self.clicked(D_PIXELS) {
            self.dot_pixels = !self.dot_pixels;
        }
        if self.clicked(FREEHAND) {
            self.free_hand = !self.free_hand;
            if self.free_hand {
                self.view.show_around_middle(FREEHAND_SAMPLES);
            }
        }
        if let Some(click) = self.button(SAVE_TYPE).clicked() {
            let chosen = state.instrument_save_type;
            state.instrument_save_type = match click.button {
                MouseButton::Right => chosen.previous(),
                _ => chosen.next(),
            };
        }
        let dot_pixels = self.dot_pixels;
        let free_hand = self.free_hand;
        self.button(D_PIXELS).set_toggled(dot_pixels);
        self.button(FREEHAND).set_toggled(free_hand);
        let label = save_type_label(state.instrument_save_type);
        self.button(SAVE_TYPE).set_text(label);
        if std::mem::take(&mut self.buffer_size_entered) {
            ask_sample_size(state, self.boxes[BUFFSIZE_BOX].0.text(), length);
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
            visible.clone(),
            range_columns,
            self.dot_pixels || self.free_hand,
        );
        let playing = state.engine.status().sample_position();
        if let Some(pos) = playing.filter(|pos| visible.contains(pos)) {
            let column = self.view.column_of(pos, WAVEFORM.w);
            painter.rect(
                Rect::new(waveform.x + column, waveform.y, 1, waveform.h),
                Some(Pen::SHADOW),
                None,
                Sides::NONE,
            );
        }
        if sample_length(state) == 0 {
            let [x, y] = NO_SAMPLE_POSITION;
            painter.text(waveform.x + x, waveform.y + y, NO_SAMPLE, Pen::SHINE);
        }
        let divider = Rect::new(area.x, area.y + DIVIDER_Y, area.w, 1);
        painter.fill(divider, Pen::SHINE);

        let shown = self.view.visible().len().to_string();
        let [end, y] = LENGTH_END;
        let x = area.x + end - fonts::INTERFACE.width(&shown);
        painter.text_in_font(x, area.y + y, &shown, pens::PINK, fonts::INTERFACE);

        for (label, [end, y]) in RIGHT_LABELS {
            let x = area.x + end - fonts::LABEL.width(label);
            painter.text_in_font(x, area.y + y, label, Pen::BACKGROUND, fonts::LABEL);
        }
        for (label, [x, y], font) in LABELS {
            painter.text_in_font(area.x + x, area.y + y, label, Pen::BACKGROUND, font);
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
        rect::Point,
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
        for (label, rect) in BUTTONS {
            let font = label_font(label, rect.w);
            assert!(font.width(label) <= rect.w - 2, "{label}");
        }
    }

    #[test]
    fn labels_stop_before_their_boxes() {
        for (label, [x, y], font) in LABELS {
            let end = x + font.width(label);
            let next = BOXES
                .iter()
                .map(|&(_, rect)| rect)
                .filter(|rect| rect.y + LABEL_Y == y && rect.x >= x)
                .map(|rect| rect.x)
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
    fn a_drawn_value_lands_on_its_row() {
        let waveform = Rect::new(0, 0, 640, 64);
        for row in 0..waveform.h {
            assert_eq!(sample_y(row_value(row), waveform), row, "{row}");
        }
        assert_eq!(row_value(0), i8::MIN);
        assert_eq!(row_value(ZERO_LINE_Y), 0);
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
        assert_eq!(column_spans(&[10, 11], false), [(10, 10), (11, 11)]);
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
        click_with(editor, state, label, MouseButton::Left);
    }

    fn click_with(
        editor: &mut SampleEditor,
        state: &mut AppState,
        label: &str,
        button: MouseButton,
    ) {
        let index = BUTTONS
            .iter()
            .position(|&(name, ..)| name == label)
            .unwrap();
        let area = editor.buttons[index].0.area();
        let pos = Point::new(area.x + area.w / 2, area.y + area.h / 2);
        editor.handle_event(state, &Event::MouseDown { pos, button });
        let up = Event::MouseUp { pos, button };
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
        click(&mut editor, &mut state, ZOOM_IN);
        assert_eq!(editor.boxes[0].0.text(), "1000");
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
            let pos = Point::new(waveform.x + column, waveform.y + 10);
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
        state.mouse_mods = ggez::input::keyboard::KeyMods::NONE;
        assert_eq!(click_column(&mut state, 320), (500, 501));
        let pos = Point::new(waveform.x + 64, waveform.y + 10);
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
        let at = |column: i32| Point::new(waveform.x + column, waveform.y + 10);
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
        editor.handle_event(
            &mut state,
            &Event::MouseMove {
                pos: Point::new(-50, 0),
            },
        );
        editor.handle_event(
            &mut state,
            &Event::MouseUp {
                pos: Point::new(-50, 0),
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
        let pos = Point::new(end_box.x + 2, end_box.y + 2);
        let button = crate::framework::event::MouseButton::Left;
        editor.handle_event(&mut state, &Event::MouseDown { pos, button });
        editor.handle_event(&mut state, &Event::MouseUp { pos, button });
        let textbox = &mut editor.boxes[RANGE_END_BOX].0;
        assert!(textbox.is_focused());
        textbox.set_text("250");
        editor.update(&mut state);
        assert_eq!(editor.range(), 0..250);
        editor.boxes[RANGE_END_BOX].0.set_text("5000");
        editor.update(&mut state);
        assert_eq!(editor.range(), 0..1000);
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
            let pos = Point::new(rect.x + 2, rect.y + 2);
            let button = crate::framework::event::MouseButton::Left;
            editor.handle_event(state, &Event::MouseDown { pos, button });
            editor.handle_event(state, &Event::MouseUp { pos, button });
            editor.boxes[BUFFSIZE_BOX].0.set_text(size);
            editor.handle_event(state, &key(finish));
            editor.update(state);
        };

        enter_size(&mut state, "1500", KeyCode::Escape);
        assert!(state.prompt.is_none());
        enter_size(&mut state, "1500", KeyCode::Return);
        assert_eq!(
            state.prompt.as_ref().map(|prompt| prompt.text()),
            Some(RESIZE_PROMPT)
        );
        crate::app::prompt::handle_event(&mut state, &key(KeyCode::R));
        let data = &active_sample(&state).unwrap().data;
        assert_eq!((data.len(), data[999], data[1000]), (1500, 1, 0));

        enter_size(&mut state, "500", KeyCode::Return);
        crate::app::prompt::handle_event(&mut state, &key(KeyCode::C));
        let data = &active_sample(&state).unwrap().data;
        assert!(data.len() == 500 && data.iter().all(|&value| value == 0));

        enter_size(&mut state, "999999", KeyCode::Return);
        crate::app::prompt::handle_event(&mut state, &key(KeyCode::Escape));
        assert_eq!(sample_length(&state), 500);
        enter_size(&mut state, "999999", KeyCode::NumpadEnter);
        crate::app::prompt::handle_event(&mut state, &key(KeyCode::R));
        assert_eq!(sample_length(&state), MAX_SAMPLE_SIZE);
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
    fn sample_clipboard() {
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
        assert_eq!(state.clipboard.sample, [10, 11, 12]);
        assert_eq!(data(&state).len(), 100);

        editor.set_range(0..2);
        click(&mut editor, &mut state, CUT);
        assert_eq!(state.clipboard.sample, [0, 1]);
        assert_eq!(data(&state)[..3], [2, 3, 4]);
        assert_eq!(editor.range(), 0..0);

        editor.set_range(3..5);
        click(&mut editor, &mut state, PASTE);
        assert_eq!(data(&state)[..6], [2, 3, 4, 0, 1, 5]);
        assert_eq!(editor.range(), 3..5);
        assert_eq!(sample_length(&state), 100);

        click(&mut editor, &mut state, SAMPLE_TO_CLIPBOARD);
        assert_eq!(state.clipboard.sample.len(), 100);
        assert_eq!(state.notification.as_deref(), Some("Sample copied"));
        state.clipboard.sample = vec![7; 5];
        click(&mut editor, &mut state, CLIPBOARD_TO_SAMPLE);
        assert_eq!(data(&state), [7; 5]);
        state.clipboard.sample = vec![7; 5];

        editor.set_range(4..4);
        click(&mut editor, &mut state, MIX);
        assert_eq!(data(&state), [7, 7, 7, 7, 7, 3, 3, 3, 3]);
        editor.set_range(1..3);
        state.clipboard.sample = vec![-7; 5];
        click(&mut editor, &mut state, MIX);
        assert_eq!(data(&state)[..4], [7, 0, 0, 7]);
        editor.set_range(2..6);
        click(&mut editor, &mut state, REVERSE);
        assert_eq!(data(&state), [7, 0, 3, 7, 7, 0, 3, 3, 3]);

        state.set_instrument(2);
        editor.update(&mut state);
        click(&mut editor, &mut state, PASTE);
        assert_eq!(data(&state), [-7; 5]);
    }

    #[test]
    fn chg_vol_fades_the_range() {
        let mut state = state();
        state.edit_active_instrument(|instrument| {
            instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
                data: vec![i8::MAX / 2; 100],
                repeat: None,
                octaves: SampleOctaves::One,
            }));
        });
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        let data = |state: &AppState| active_sample(state).unwrap().data.clone();

        editor.boxes[VOLUME_START_BOX].0.set_text("200");
        editor.boxes[VOLUME_END_BOX].0.set_text("1");
        // without a range only the selected sample changes, at the start percentage
        editor.set_range(5..5);
        click(&mut editor, &mut state, CHANGE_VOLUME);
        assert_eq!(data(&state)[4..7], [63, 126, 63]);

        state.edit_active_instrument(|instrument| {
            instrument.kind = InstrumentKind::Sample(Arc::new(sample_of(vec![i8::MAX / 2; 100])));
        });
        editor.set_range(0..100);
        click(&mut editor, &mut state, CHANGE_VOLUME);
        let faded = data(&state);
        assert_eq!((faded[0], faded[99]), (126, 1));
        assert!(faded.windows(2).all(|pair| pair[0] >= pair[1]));

        for typed in ["0", ""] {
            editor.boxes[VOLUME_END_BOX].0.set_text(typed);
            state.notification = None;
            click(&mut editor, &mut state, CHANGE_VOLUME);
            assert_eq!(state.notification.as_deref(), Some(VOLUME_OUT_OF_RANGE));
            assert_eq!(data(&state), faded);
        }
    }

    #[test]
    fn freehand_zooms_to_a_sample_a_column() {
        let mut state = state();
        with_sample(&mut state, 1000);
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        click(&mut editor, &mut state, FREEHAND);
        assert!(editor.free_hand);
        assert_eq!(editor.view.visible(), 180..820);
        // zooming in keeps it, any zoom out lets go of it
        click(&mut editor, &mut state, ZOOM_IN);
        assert!(editor.free_hand);
        click(&mut editor, &mut state, ZOOM_OUT);
        assert!(!editor.free_hand);
        click(&mut editor, &mut state, FREEHAND);
        click(&mut editor, &mut state, SHOW_ALL);
        assert!(!editor.free_hand && !editor.button(FREEHAND).is_toggled());
        // turning it off leaves the view alone
        click(&mut editor, &mut state, FREEHAND);
        click(&mut editor, &mut state, FREEHAND);
        assert_eq!(editor.view.visible(), 180..820);
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
    fn only_a_part_thats_there_plays() {
        let mut state = state();
        assert!(!can_play(&state, &(0..10)));
        with_sample(&mut state, 100);
        assert!(can_play(&state, &(20..30)));
        assert!(can_play(&state, &(0..100)));
        assert!(!can_play(&state, &(40..40)));
        assert!(!can_play(&state, &(90..110)));
    }

    // FREEHAND on, a left click at `row` of the middle column
    fn draw_at(editor: &mut SampleEditor, state: &mut AppState, row: i32) {
        editor.free_hand = true;
        editor.update(state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(state, AREA, &mut Painter::new(&mut screen, &state.palette));
        let waveform = WAVEFORM.within(AREA);
        let pos = Point::new(waveform.x + waveform.w / 2, waveform.y + row);
        let button = MouseButton::Left;
        editor.handle_event(state, &Event::MouseDown { pos, button });
        editor.update(state);
        editor.handle_event(state, &Event::MouseUp { pos, button });
    }

    #[test]
    fn freehand_on_an_empty_slot_draws_nothing() {
        let mut state = state();
        draw_at(&mut SampleEditor::new(), &mut state, 0);
        assert_eq!(sample_length(&state), 0);
    }

    #[test]
    fn freehand_only_writes_a_change() {
        let mut state = state();
        with_sample(&mut state, 1000);
        let before = active_instrument_sample(&state);
        let mut editor = SampleEditor::new();
        // silence already sits on the zero line
        draw_at(&mut editor, &mut state, ZERO_LINE_Y);
        assert!(Arc::ptr_eq(&before, &active_instrument_sample(&state)));
        draw_at(&mut editor, &mut state, 0);
        assert!(active_instrument_sample(&state).data.contains(&i8::MIN));
    }

    #[test]
    fn a_multi_octave_samples_length_stays_put() {
        let mut state = state();
        let octaves = OctamedicSample {
            data: vec![0; 7],
            repeat: Some((0, 1)),
            octaves: SampleOctaves::Three,
        };
        state.edit_active_instrument(|instrument| {
            instrument.kind = InstrumentKind::Sample(Arc::new(octaves));
        });
        edit_sample(&mut state, |sample| sample.without(0..1));
        assert_eq!(sample_length(&state), 7);
        assert_eq!(state.notification.as_deref(), Some(MULTI_OCTAVE_LENGTH));
        edit_sample(&mut state, |sample| sample.cleared(0..7));
        assert_eq!(sample_length(&state), 7);
    }

    fn active_instrument_sample(state: &AppState) -> Arc<OctamedicSample> {
        let instrument = state.song().instrument(state.active_instrument).unwrap();
        return instrument.sample().unwrap().clone();
    }

    #[test]
    fn save_type_cycles_through_the_formats() {
        use InstrumentSaveType::{Iff, Raw, Wav};
        let mut state = state();
        let mut editor = SampleEditor::new();
        editor.update(&mut state);
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        editor.render(&state, AREA, &mut Painter::new(&mut screen, &state.palette));
        assert_eq!(state.instrument_save_type, Raw);
        let mut chosen = Vec::new();
        for _ in 0..3 {
            click(&mut editor, &mut state, SAVE_TYPE);
            chosen.push(state.instrument_save_type);
        }
        assert_eq!(chosen, [Iff, Wav, Raw]);
        click_with(&mut editor, &mut state, SAVE_TYPE, MouseButton::Right);
        assert_eq!(state.instrument_save_type, Wav);
    }

    #[test]
    fn every_save_type_fits_the_button() {
        let (_, rect) = BUTTONS[BUTTONS
            .iter()
            .position(|&(name, _)| name == SAVE_TYPE)
            .unwrap()];
        // the font's picked for the label it starts with
        let font = label_font(SAVE_TYPE, rect.w);
        for save_type in InstrumentSaveType::ALL {
            let label = save_type_label(save_type);
            assert!(font.width(label) <= rect.w - 2, "{label}");
        }
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
