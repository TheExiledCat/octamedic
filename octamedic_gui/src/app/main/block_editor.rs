use std::array;

use ggez::input::keyboard::{KeyCode, KeyMods};
use mmd::utility::note::SEMITONES_PER_OCTAVE;
use octamedic_core::{
    amiga::paula::SCOPE_CHANNELS,
    data::{
        instrument::InstrumentId,
        note::OctamedicNote,
        pattern::{OctamedicPattern, OctamedicPatternTrack},
        track_count::TrackCount,
    },
};
use octamedic_util::base32;
use rustuition::{
    clickable::{Click, Clickable},
    event::MouseButton,
    font::Font,
    gadget::Gadget,
    painter::{CHAR_HEIGHT, CHAR_WIDTH, Painter, Pen},
    rect::{Point, Rect},
};

use crate::{
    app::{
        block_range::Cell,
        cursor::EditCursor,
        fonts, pens,
        state::{AppState, PlayStart},
    },
    framework::{
        event::Event,
        keymap::{AMIGA, Action, Key, KeyActions, Keymap},
        widgets::audio_view::{Oscilloscope, VuMeter},
    },
};

// measured off OctaMED's 4-track view, in font pixels. a sample per pixel, 2 in from the track
// edge. scopes hang off the highlighted line, meters off the middle of the editor
const SCOPE_OFFSET: i32 = 2;
const SCOPE_SAMPLES: usize = 42;

const METER_OFFSET: i32 = 45;
const METER_WIDTH: i32 = 10;

// 3 digits and a space
const LINE_NUMBER_CHARS: i32 = 4;

const NOTE_CHARS: i32 = 3;
const HOLD_TEXT: &str = "-|-";
// puts in a hold like Enter does
const HOLD_KEY: char = 'a';
const NUMBER_KEYS: [KeyCode; 10] = [
    KeyCode::Key0,
    KeyCode::Key1,
    KeyCode::Key2,
    KeyCode::Key3,
    KeyCode::Key4,
    KeyCode::Key5,
    KeyCode::Key6,
    KeyCode::Key7,
    KeyCode::Key8,
    KeyCode::Key9,
];
const ALT_DIGIT_ADD: u32 = 10;
// the command's first digit, after the note and the instrument
const FIRST_COMMAND_COLUMN: usize = 2;

// track numbers end over the instrument's first digit
const TRACK_NUMBER_END: i32 = NOTE_CHARS + 1;

// blocks with up to this many tracks use the wide font
const WIDE_TRACKS: usize = 4;

const QUARTERS: usize = 4;

// octaves past 9 are written as letters
const OCTAVE_RADIX: u32 = 36;

// as in `C#1 10C20`
const TRACK_CHARS: i32 = 9;

const NOTE_NAMES: [&str; SEMITONES_PER_OCTAVE as usize] = [
    "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorAction {
    Up,
    Down,
    Left,
    Right,
    FirstLine,
    SecondQuarter,
    Middle,
    LastQuarter,
    LastLine,
    TrackLeft,
    TrackRight,
    ScreenLeft,
    ScreenRight,
    // no note, just the instrument, which keeps a held note going
    Hold,
    Highlight,
    // the note and instrument, or the command digit under the cursor
    DeleteNote,
    DeleteCell,
    // in the cursor's track, the rest of it moving up into the gap or down out of the way
    DeleteSpace,
    InsertSpace,
    // the BLOCK panel's DEL.TRK and INS.TRK
    DeleteTrack,
    InsertTrack,
    // the BLOCK panel's TRK: and BLK: buttons
    CutTrack,
    CopyTrack,
    PasteTrack,
    CutBlock,
    CopyBlock,
    PasteBlock,
    // SLIDE: VOL
    VolumeSlide,
    Undo,
    Redo,
    HoldChord,
}

impl Action for EditorAction {}

pub struct BlockEditor {
    keys: Keymap<EditorAction>,
    first_track: usize,
    // as of the last render
    visible_tracks: usize,
    oscilloscopes: [Oscilloscope; SCOPE_CHANNELS],
    meters: [VuMeter; SCOPE_CHANNELS],
    clickable: Clickable,
    area: Rect,
    dragging_range: bool,
}

impl BlockEditor {
    pub fn new() -> Self {
        use EditorAction::*;
        let keys = Keymap::new("editor")
            .bind(Key::plain(KeyCode::Up), Up, "Up")
            .bind(Key::plain(KeyCode::Down), Down, "Down")
            .bind(Key::plain(KeyCode::Left), Left, "Left")
            .bind(Key::plain(KeyCode::Right), Right, "Right")
            .bind(Key::plain(KeyCode::F6), FirstLine, "First line")
            .bind(Key::plain(KeyCode::F7), SecondQuarter, "2nd quarter")
            .bind(Key::plain(KeyCode::F8), Middle, "Middle")
            .bind(Key::plain(KeyCode::F9), LastQuarter, "Last quarter")
            .bind(Key::plain(KeyCode::F10), LastLine, "Last line")
            .bind(Key::alt(KeyCode::Left), TrackLeft, "Track left")
            .bind(Key::alt(KeyCode::Right), TrackRight, "Track right")
            .bind(Key::ctrl(KeyCode::Left), ScreenLeft, "Screen left")
            .bind(Key::ctrl(KeyCode::Right), ScreenRight, "Screen right")
            .bind(Key::plain(KeyCode::Return), Hold, "Hold")
            .bind(Key::plain(KeyCode::NumpadEnter), Hold, "Hold")
            .bind(Key::plain(KeyCode::Tab), Highlight, "Highlight")
            .bind(Key::plain(KeyCode::Delete), DeleteNote, "Del note")
            .bind(Key::shift(KeyCode::Delete), DeleteCell, "Del note+cmd")
            .bind(Key::plain(KeyCode::Back), DeleteSpace, "Del space")
            .bind(Key::shift(KeyCode::Back), InsertSpace, "Ins space")
            .bind(Key::alt(KeyCode::Back), DeleteTrack, "Del track")
            .bind(
                Key::new(KeyCode::Back, KeyMods::SHIFT | KeyMods::ALT),
                InsertTrack,
                "Ins track",
            )
            .bind(Key::amiga(KeyCode::X), CutTrack, "Cut track")
            .bind(Key::amiga(KeyCode::C), CopyTrack, "Copy track")
            .bind(Key::amiga(KeyCode::V), PasteTrack, "Paste track")
            .bind(
                Key::new(KeyCode::X, AMIGA | KeyMods::SHIFT),
                CutBlock,
                "Cut block",
            )
            .bind(
                Key::new(KeyCode::C, AMIGA | KeyMods::SHIFT),
                CopyBlock,
                "Copy block",
            )
            .bind(
                Key::new(KeyCode::V, AMIGA | KeyMods::SHIFT),
                PasteBlock,
                "Paste block",
            )
            .bind(Key::amiga(KeyCode::O), VolumeSlide, "Volume slide")
            .bind(Key::shift(KeyCode::Return), HoldChord, "Hold chord")
            .bind(Key::shift(KeyCode::NumpadEnter), HoldChord, "Hold chord")
            .bind(Key::ctrl(KeyCode::Z), Undo, "Undo")
            .bind(Key::ctrl(KeyCode::Y), Redo, "Redo")
            .bind(
                Key::new(KeyCode::Z, KeyMods::CTRL | KeyMods::SHIFT),
                Redo,
                "Redo",
            );
        return Self {
            keys,
            first_track: 0,
            visible_tracks: 1,
            oscilloscopes: array::from_fn(|_| Oscilloscope::new(SCOPE_SAMPLES)),
            meters: array::from_fn(|_| VuMeter::default()),
            clickable: Clickable::default(),
            area: Rect::default(),
            dragging_range: false,
        };
    }

    fn click(&mut self, state: &mut AppState, event: &Event) -> bool {
        if event.button() == Some(MouseButton::Right) {
            return false;
        }
        let click = self.clickable.handle(event);
        if let Click::Clicked {
            pos: Point { x, .. },
            ..
        } = click
        {
            let layout = state
                .active_pattern()
                .map(|pattern| Layout::new(self.area, pattern.track_count()));
            if let Some(index) = layout.and_then(|layout| layout.track_at(x)) {
                state.toggle_track_mute(self.first_track + index);
            }
        }
        return click.handled();
    }

    fn drag_range(&mut self, state: &mut AppState, event: &Event) -> bool {
        match *event {
            Event::MouseDown {
                pos,
                button: MouseButton::Right,
            } => {
                let Some(cell) = self.cell_at(state, pos, false) else {
                    return false;
                };
                self.dragging_range = true;
                state.start_range(cell);
                return true;
            }
            Event::MouseMove { pos } if self.dragging_range => {
                // the button was let go where the release didn't reach the editor
                self.dragging_range = state.right_button_held;
                if let Some(cell) = self
                    .cell_at(state, pos, true)
                    .filter(|_| self.dragging_range)
                {
                    state.extend_range(cell);
                }
                return false;
            }
            Event::MouseUp {
                pos,
                button: MouseButton::Right,
            } if self.dragging_range => {
                self.dragging_range = false;
                if let Some(cell) = self.cell_at(state, pos, true) {
                    state.extend_range(cell);
                }
                return true;
            }
            _ => return false,
        }
    }

    // the cell under `pos`. With `nearest`, the closest one when it's off the tracks or the
    // block, since a drag can go past them
    fn cell_at(&self, state: &AppState, pos: Point, nearest: bool) -> Option<Cell> {
        let pattern = state.active_pattern()?;
        let layout = Layout::new(self.area, pattern.track_count());
        let row = (pos.y - self.area.y).div_euclid(CHAR_HEIGHT);
        let line = state.cursor.line as i32 + row - center_row(self.area);
        let column = (pos.x - layout.x).div_euclid(layout.char_width) - LINE_NUMBER_CHARS;
        let index = column.div_euclid(TRACK_CHARS);
        let last_line = pattern.line_count() as i32 - 1;
        let last_index = layout.tracks as i32 - 1;
        let on_a_cell = self.area.contains(pos)
            && (0..=last_index).contains(&index)
            && (0..=last_line).contains(&line);
        if !on_a_cell && !nearest {
            return None;
        }
        return Some(Cell {
            track: self.first_track + index.clamp(0, last_index) as usize,
            line: line.clamp(0, last_line) as usize,
        });
    }

    // OctaMED fills the range's cells red, which mixes the text over it to rose
    fn render_range(
        &self,
        state: &AppState,
        pattern: &OctamedicPattern,
        layout: &Layout,
        area: Rect,
        highlighted: &[bool],
        painter: &mut Painter,
    ) {
        let Some(range) = state.active_range() else {
            return;
        };
        // the range's tracks that are showing, as indices into the visible ones
        let tracks = range.tracks();
        let first = tracks.start.max(self.first_track) - self.first_track;
        let end = tracks.end.min(self.first_track + layout.tracks);
        if self.first_track + first >= end {
            return;
        }
        let x = layout.track_x(first);
        let width = (end - self.first_track - first) as i32 * TRACK_CHARS * layout.char_width;
        for row in 0..area.h / CHAR_HEIGHT {
            let line = line_at(state.cursor.line, row, area);
            if !line.is_some_and(|line| range.lines().contains(&line)) {
                continue;
            }
            let (fill, text) = if highlighted[row as usize] {
                (pens::highlight(pens::RED), pens::highlight(pens::ROSE))
            } else {
                (pens::RED, pens::ROSE)
            };
            let rect = Rect::new(x, row_rect(area, row).y, width, CHAR_HEIGHT);
            painter.fill(rect, fill);
            painter.clipped(rect, |painter| {
                self.render_row(pattern, state.cursor.line, row, layout, area, text, painter);
            });
        }
    }

    fn scroll_to(&mut self, track: usize, tracks: usize, visible: usize) {
        if track < self.first_track {
            self.first_track = track;
        } else if track >= self.first_track + visible {
            self.first_track = track + 1 - visible;
        }
        self.first_track = self.first_track.min(tracks.saturating_sub(visible));
    }

    fn render_line(
        &self,
        pattern: &OctamedicPattern,
        line: usize,
        layout: &Layout,
        y: i32,
        pen: Pen,
        painter: &mut Painter,
    ) {
        let text = |painter: &mut Painter, column: i32, text: &str| {
            let x = layout.x + column * layout.char_width;
            painter.text_in_font(x, y, text, pen, layout.font);
        };
        text(painter, 0, &format!("{line:03}"));
        let cells = pattern.lines[line].tracks.iter().skip(self.first_track);
        for (index, cell) in cells.take(layout.tracks).enumerate() {
            text(painter, layout.track_column(index), &track_text(cell));
        }
    }

    // drawn over the top two rows, whatever lines are there
    fn render_labels(
        &self,
        pattern: &OctamedicPattern,
        layout: &Layout,
        y: i32,
        painter: &mut Painter,
    ) {
        let mut label = |text: String, end_column: i32, y: i32| {
            let end = layout.x + end_column * layout.char_width;
            let x = end - fonts::INTERFACE.width(&text);
            painter.text_in_font(x, y, &text, pens::PINK, fonts::INTERFACE);
        };
        for index in 0..layout.tracks {
            let number = self.first_track + index;
            label(
                number.to_string(),
                layout.track_column(index) + TRACK_NUMBER_END,
                y,
            );
        }
        let count = TrackCount::holding(pattern.track_count());
        label(
            count.tracks().to_string(),
            LINE_NUMBER_CHARS,
            y + CHAR_HEIGHT,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn render_row(
        &self,
        pattern: &OctamedicPattern,
        cursor_line: usize,
        row: i32,
        layout: &Layout,
        area: Rect,
        pen: Pen,
        painter: &mut Painter,
    ) {
        let line = cursor_line as isize + (row - center_row(area)) as isize;
        let Ok(line) = usize::try_from(line) else {
            return;
        };
        if line < pattern.line_count() {
            let y = area.y + row * CHAR_HEIGHT;
            self.render_line(pattern, line, layout, y, pen, painter);
        }
    }

    // kept clear of the field's text: a line above and below the band, the right side just past
    // the field
    fn cursor_box(&self, cursor: &EditCursor, layout: &Layout, band: Rect) -> Option<CursorBox> {
        let index = cursor.track.checked_sub(self.first_track)?;
        let (offset, width) = match cursor.column {
            0 => (0, NOTE_CHARS),
            digit => (NOTE_CHARS + digit as i32, 1),
        };
        let x = layout.x + (layout.track_column(index) + offset) * layout.char_width;
        let width = width * layout.char_width;
        return Some(CursorBox {
            top: Rect::new(x, band.y - 1, width + 1, 1),
            bottom: Rect::new(x, band.bottom(), width + 1, 1),
            sides: [
                Rect::new(x, band.y, 1, band.h),
                Rect::new(x + width, band.y, 1, band.h),
            ],
        });
    }

    // OctaMED does the cursor by setting the red bitplane, so its colour depends on what's under
    // it. faked here by redrawing the text under each part in the matching colour
    fn render_cursor(
        &self,
        pattern: &OctamedicPattern,
        cursor_line: usize,
        parts: CursorBox,
        layout: &Layout,
        area: Rect,
        painter: &mut Painter,
    ) {
        let lines = [
            (parts.top, center_row(area) - 1, pens::ROSE),
            (parts.bottom, center_row(area) + 1, pens::ROSE),
        ];
        for (rect, row, text) in lines {
            painter.fill(rect, pens::RED);
            painter.clipped(rect, |painter| {
                self.render_row(pattern, cursor_line, row, layout, area, text, painter);
            });
        }
        for side in parts.sides {
            painter.fill(side, pens::PINK);
            painter.clipped(side, |painter| {
                let pen = pens::BLUSH;
                self.render_row(
                    pattern,
                    cursor_line,
                    center_row(area),
                    layout,
                    area,
                    pen,
                    painter,
                );
            });
        }
    }

    // only Paula's four hardware channels, the first four tracks, have scopes
    fn render_scopes(
        &mut self,
        state: &AppState,
        pattern: &OctamedicPattern,
        layout: &Layout,
        area: Rect,
        painter: &mut Painter,
    ) {
        for index in 0..layout.tracks {
            let track = self.first_track + index;
            if track >= SCOPE_CHANNELS {
                break;
            }
            if state.is_playing() && state.active_meters().scopes {
                let x = layout.track_x(index);
                let scope = scope_area(layout, area, index);
                for run in self.oscilloscopes[track].runs(scope) {
                    painter.fill(run, pens::PINK);
                    if let Some(cell) = cell_at(pattern, state.cursor.line, area, run.y, track) {
                        let text_line = ((run.y - area.y) % CHAR_HEIGHT) as usize;
                        blend_text(&track_text(cell), x, text_line, layout.font, run, painter);
                    }
                }
            }
            if state.active_meters().vu {
                self.meters[track].render(state, meter_area(layout, area, index), painter);
            }
        }
    }
}

impl KeyActions for BlockEditor {
    type Action = EditorAction;

    fn keymap(&self) -> &Keymap<EditorAction> {
        return &self.keys;
    }

    fn perform(&mut self, action: EditorAction, state: &mut AppState) {
        use EditorAction::*;
        let (lines, tracks) = state.active_pattern().map_or((0, 0), |pattern| {
            (pattern.line_count(), pattern.track_count())
        });
        let screen = self.visible_tracks as isize;
        let step = state.line_step() as isize;
        let line_before = state.cursor.line;
        let cursor = &mut state.cursor;
        match action {
            Up => cursor.move_lines(-step, lines),
            Down => cursor.move_lines(step, lines),
            Left => cursor.move_columns(-1, tracks),
            Right => cursor.move_columns(1, tracks),
            FirstLine => cursor.set_line(0, lines),
            SecondQuarter => cursor.set_line(lines / QUARTERS, lines),
            Middle => cursor.set_line(lines / 2, lines),
            LastQuarter => cursor.set_line(lines * (QUARTERS - 1) / QUARTERS, lines),
            LastLine => cursor.set_line(lines.saturating_sub(1), lines),
            TrackLeft => cursor.move_tracks(-1, tracks),
            TrackRight => cursor.move_tracks(1, tracks),
            ScreenLeft => cursor.move_tracks_clamped(-screen, tracks),
            ScreenRight => cursor.move_tracks_clamped(screen, tracks),
            Hold => enter_hold(state),
            Highlight => state.toggle_highlight(state.cursor.line),
            DeleteNote => delete(state, false),
            DeleteCell => delete(state, true),
            DeleteSpace | InsertSpace => {
                let EditCursor {
                    line,
                    track,
                    edit_mode,
                    ..
                } = *cursor;
                if !edit_mode {
                    return;
                }
                if action == DeleteSpace {
                    state.pull_up(line, track);
                } else {
                    state.push_down(line, track);
                }
            }
            // like the buttons, these don't need edit mode
            DeleteTrack => state.delete_track(),
            InsertTrack => state.insert_track(),
            CutTrack => state.cut_track(),
            CopyTrack => state.copy_track(),
            PasteTrack => state.paste_track(),
            CutBlock => state.cut_block(),
            CopyBlock => state.copy_block(),
            PasteBlock => state.paste_block(),
            VolumeSlide => state.slide_volume(),
            Undo => state.undo(),
            Redo => state.redo(),
            HoldChord => state.hold_last_chord(),
        }
        let moved_line = matches!(
            action,
            Up | Down | FirstLine | SecondQuarter | Middle | LastQuarter | LastLine
        );
        if moved_line && state.play_on_move && state.cursor.line != line_before {
            state.play_line(state.cursor.line);
        }
    }

    fn describe(action: EditorAction) -> &'static str {
        use EditorAction::*;
        return match action {
            Up => "Up one line",
            Down => "Down one line",
            Left => "Left one space",
            Right => "Right one space",
            FirstLine => "Go to the first line of the block",
            SecondQuarter => "Go to the second quarter of the block",
            Middle => "Go to the middle of the block",
            LastQuarter => "Go to the last quarter of the block",
            LastLine => "Go to the last line of the block",
            TrackLeft => "One track left",
            TrackRight => "One track right",
            ScreenLeft => "One screen left",
            ScreenRight => "One screen right",
            Hold => "Enter a hold: no note, just the instrument",
            Highlight => "Highlight the line, or take its highlight off",
            DeleteNote => "Delete the note, or the digit under the cursor",
            DeleteCell => "Delete the note and its command",
            DeleteSpace => "Delete the note and close the gap in its track",
            InsertSpace => "Insert a space in the track",
            DeleteTrack => "Delete the track, like DEL.TRK",
            InsertTrack => "Insert a track, like INS.TRK",
            CutTrack => "Cut the track into the track buffer",
            CopyTrack => "Copy the track into the track buffer",
            PasteTrack => "Overwrite the track with the track buffer",
            CutBlock => "Cut the block into the block buffer",
            CopyBlock => "Copy the block into the block buffer",
            PasteBlock => "Replace the block with the block buffer",
            VolumeSlide => "Slide the volume between the volume commands around the cursor",
            Undo => "Undo the last change to the blocks",
            HoldChord => "Enter holds on every track of the last chord",
            Redo => "Redo the last change undone",
        };
    }
}

impl Gadget<AppState, Event> for BlockEditor {
    fn update(&mut self, state: &mut AppState) {
        if !state.is_playing() {
            self.meters = array::from_fn(|_| VuMeter::default());
            return;
        }
        if state.displays_off {
            return;
        }
        // the engine's line and block are read apart, so the line can be from the block before
        let lines = state
            .active_pattern()
            .map_or(1, |pattern| pattern.line_count());
        let row = state.engine.status().row();
        state.cursor.line = row.min(lines.saturating_sub(1));
        let scopes = state.engine.status().scopes();
        for channel in 0..SCOPE_CHANNELS {
            let samples = scopes.read(channel);
            self.oscilloscopes[channel].show(&samples);
            let peak = samples.iter().map(|sample| sample.unsigned_abs()).max();
            self.meters[channel].set(peak.unwrap_or(0));
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        if self.drag_range(state, event) || self.dispatch(state, event) || self.click(state, event)
        {
            return true;
        }
        if let Some(digit) = alt_digit(event) {
            let cursor = state.cursor;
            let command_column = cursor.column >= FIRST_COMMAND_COLUMN;
            return cursor.edit_mode && command_column && enter_digit(state, digit);
        }
        let Event::Text {
            character,
            mods,
            repeat,
        } = *event
        else {
            return false;
        };
        // shortcuts type characters too
        if mods.intersects(KeyMods::CTRL | KeyMods::ALT | KeyMods::LOGO) {
            return false;
        }
        if character == ' ' && mods.is_empty() && state.space_deletes {
            delete(state, false);
            return true;
        }
        if state.chord_mode && !character.eq_ignore_ascii_case(&HOLD_KEY) {
            return type_chord_key(state, character, repeat);
        }
        return enter_key(state, character);
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        self.area = area;
        self.clickable.set_area(area);
        painter.fill(area, Pen::BACKGROUND);
        let Some(pattern) = state.active_pattern() else {
            return;
        };
        let cursor = state.cursor;
        let layout = Layout::new(area, pattern.track_count());
        self.visible_tracks = layout.fits;
        self.scroll_to(cursor.track, pattern.track_count(), layout.tracks);
        let center_y = area.y + center_row(area) * CHAR_HEIGHT;
        let band = active_band(area, center_y);
        painter.fill(band, Pen::SHADOW);
        let rows = area.h / CHAR_HEIGHT;
        let highlighted: Vec<bool> = (0..rows)
            .map(|row| {
                let line = line_at(cursor.line, row, area);
                line.and_then(|line| pattern.lines.get(line))
                    .is_some_and(|line| line.highlighted)
            })
            .collect();
        for row in 0..rows {
            let center = row == center_row(area);
            let pen = if center { Pen::FILL } else { Pen::SHINE };
            let pen = if highlighted[row as usize] {
                if center {
                    painter.fill(band, pens::highlight(Pen::SHADOW));
                } else {
                    painter.fill(row_rect(area, row), pens::highlight(Pen::BACKGROUND));
                }
                pens::highlight(pen)
            } else {
                pen
            };
            self.render_row(pattern, cursor.line, row, &layout, area, pen, painter);
        }
        self.render_range(state, pattern, &layout, area, &highlighted, painter);
        // one font pixel wide
        let width = layout.char_width / CHAR_WIDTH;
        for track in 1..layout.tracks {
            let x = layout.track_x(track) - 1;
            painter.fill(Rect::new(x, area.y, width, area.h), pens::PINK);
            for row in (0..rows).filter(|&row| highlighted[row as usize]) {
                let rect = row_rect(area, row);
                painter.fill(
                    Rect::new(x, rect.y, width, rect.h),
                    pens::highlight(pens::PINK),
                );
            }
            // OctaMED draws the band by flipping a bitplane, which turns the pink red
            let pen = if highlighted[center_row(area) as usize] {
                pens::highlight(pens::RED)
            } else {
                pens::RED
            };
            painter.fill(Rect::new(x, band.y, width, band.h), pen);
        }
        self.render_labels(pattern, &layout, area.y, painter);
        self.render_scopes(state, pattern, &layout, area, painter);
        if let Some(parts) = self.cursor_box(&cursor, &layout, band) {
            self.render_cursor(pattern, cursor.line, parts, &layout, area, painter);
        }
    }
}

#[derive(Clone, Copy)]
struct CursorBox {
    top: Rect,
    bottom: Rect,
    sides: [Rect; 2],
}

fn row_rect(area: Rect, row: i32) -> Rect {
    return Rect::new(area.x, area.y + row * CHAR_HEIGHT, area.w, CHAR_HEIGHT);
}

fn line_at(cursor_line: usize, row: i32, area: Rect) -> Option<usize> {
    return usize::try_from(cursor_line as isize + (row - center_row(area)) as isize).ok();
}

fn center_row(area: Rect) -> i32 {
    return area.h / CHAR_HEIGHT / 2;
}

fn active_band(area: Rect, center_y: i32) -> Rect {
    return Rect::new(area.x, center_y - 1, area.w, CHAR_HEIGHT + 1);
}

struct Layout {
    x: i32,
    font: Font,
    char_width: i32,
    tracks: usize,
    fits: usize,
}

impl Layout {
    fn new(area: Rect, track_count: usize) -> Self {
        let font = if track_count <= WIDE_TRACKS {
            fonts::EDITOR
        } else {
            Font::TOPAZ
        };
        let char_width = font.advance('0');
        let columns = area.w / char_width - LINE_NUMBER_CHARS;
        let fits = (columns / TRACK_CHARS).max(1) as usize;
        return Self {
            x: area.x,
            font,
            char_width,
            tracks: track_count.min(fits),
            fits,
        };
    }

    fn track_column(&self, track: usize) -> i32 {
        return LINE_NUMBER_CHARS + track as i32 * TRACK_CHARS;
    }

    fn track_x(&self, track: usize) -> i32 {
        return self.x + self.track_column(track) * self.char_width;
    }

    fn track_at(&self, x: i32) -> Option<usize> {
        let column = (x - self.x) / self.char_width - LINE_NUMBER_CHARS;
        let track = usize::try_from(column / TRACK_CHARS).ok()?;
        return (column >= 0 && track < self.tracks).then_some(track);
    }
}

// in edit mode a note key also writes the note and moves down by the line add, and so does a
// digit in the instrument, command and value columns
fn enter_key(state: &mut AppState, key: char) -> bool {
    let cursor = state.cursor;
    if cursor.column != 0 {
        return cursor.edit_mode && enter_digit(state, key);
    }
    if !cursor.edit_mode {
        return state.play_key(cursor.track, key).is_some();
    }
    if key.eq_ignore_ascii_case(&HOLD_KEY) {
        enter_hold(state);
        return true;
    }
    if state.enter_key(cursor.track, cursor.line, key).is_none() {
        return false;
    }
    if state.play_on_note && !state.is_playing() {
        state.start(PlayStart::Block);
    }
    state.advance_after_note();
    return true;
}

fn type_chord_key(state: &mut AppState, key: char, repeat: bool) -> bool {
    let cursor = state.cursor;
    if cursor.column != 0 {
        return cursor.edit_mode && enter_digit(state, key);
    }
    // a held key's repeats aren't more notes
    if repeat {
        return state.key_to_note(key).is_some();
    }
    if !cursor.edit_mode {
        return state.play_chord_key(key);
    }
    if !state.enter_chord_key(key) {
        return false;
    }
    if state.play_on_note && !state.is_playing() {
        state.start(PlayStart::Block);
    }
    return true;
}

// Del clears the note and instrument, on the instrument column just the instrument and on a
// command column just that digit; with shift the whole cell. Then down by the line add, like
// entering something
fn delete(state: &mut AppState, whole_cell: bool) {
    let cursor = state.cursor;
    if !cursor.edit_mode {
        return;
    }
    state.edit_track(cursor.line, cursor.track, |cell| {
        if whole_cell {
            *cell = OctamedicPatternTrack::new();
        } else if cursor.column == 0 {
            cell.note = OctamedicNote::new();
            cell.instrument_id = InstrumentId(0);
        } else if let Some(cleared) = type_digit(*cell, cursor.column, '0') {
            *cell = cleared;
        }
    });
    if !whole_cell && cursor.column >= FIRST_COMMAND_COLUMN {
        state.advance_after_digit();
    } else {
        state.advance_after_note();
    }
}

fn enter_hold(state: &mut AppState) {
    let cursor = state.cursor;
    if !cursor.edit_mode || cursor.column != 0 {
        return;
    }
    state.enter_note(cursor.track, cursor.line, OctamedicNote { value: 0 });
    state.advance_after_note();
}

// Alt and a number key types 10 more, so A to F go in from the number row like OctaMED
fn alt_digit(event: &Event) -> Option<char> {
    let Event::Key { key, mods, .. } = *event else {
        return None;
    };
    if mods != KeyMods::ALT {
        return None;
    }
    let number = NUMBER_KEYS.iter().position(|&code| code == key)? as u32;
    return char::from_digit(number + ALT_DIGIT_ADD, 16);
}

fn enter_digit(state: &mut AppState, key: char) -> bool {
    let cursor = state.cursor;
    let Some(&cell) = state
        .active_pattern()
        .and_then(|pattern| pattern.track(cursor.line, cursor.track))
    else {
        return false;
    };
    let Some(typed) = type_digit(cell, cursor.column, key) else {
        return false;
    };
    state.edit_track(cursor.line, cursor.track, |cell| *cell = typed);
    if cursor.column >= FIRST_COMMAND_COLUMN {
        state.advance_after_digit();
    } else {
        state.advance_after_note();
    }
    return true;
}

// the instrument's right base 32 digit, then the command's and the value's two hex digits each.
// like OctaMED the instrument's left digit can't be typed, so typing gives 0 to V and higher ones
// come from entering a note with that instrument selected. None if `key` isn't a digit there
fn type_digit(
    mut cell: OctamedicPatternTrack,
    column: usize,
    key: char,
) -> Option<OctamedicPatternTrack> {
    match column {
        1 => cell.instrument_id = InstrumentId(base32::digit_value(key)? as u8),
        2..=5 => {
            let digit = key.to_digit(16)? as u8;
            let high = column % 2 == 0;
            let byte = match column {
                2 | 3 => &mut cell.command_id.0,
                _ => &mut cell.command_value,
            };
            *byte = if high {
                digit << 4 | *byte & 0x0F
            } else {
                *byte & 0xF0 | digit
            };
        }
        _ => return None,
    }
    return Some(cell);
}

pub fn note_text(note: u8) -> String {
    if note == 0 {
        return "---".to_string();
    }
    let index = note as usize - 1;
    let semitones = SEMITONES_PER_OCTAVE as usize;
    let octave = std::char::from_digit((index / semitones + 1) as u32, OCTAVE_RADIX).unwrap_or('?');
    return format!(
        "{}{}",
        NOTE_NAMES[index % semitones],
        octave.to_ascii_uppercase()
    );
}

fn instrument_text(instrument: u8) -> String {
    return base32::format(instrument as u32, 2, ' ');
}

fn scope_area(layout: &Layout, area: Rect, index: usize) -> Rect {
    let pixel = layout.char_width / CHAR_WIDTH;
    let x = layout.track_x(index) + SCOPE_OFFSET * pixel;
    let top = active_band(area, area.y + center_row(area) * CHAR_HEIGHT).y;
    return Rect::new(x, top, SCOPE_SAMPLES as i32 * pixel, area.y + area.h - top);
}

fn meter_area(layout: &Layout, area: Rect, index: usize) -> Rect {
    let pixel = layout.char_width / CHAR_WIDTH;
    let x = layout.track_x(index) + METER_OFFSET * pixel;
    let top = area.h / 2;
    return Rect::new(x, area.y + top, METER_WIDTH * pixel, area.h - top);
}

// None if `y` is above or below the block
fn cell_at(
    pattern: &OctamedicPattern,
    cursor_line: usize,
    area: Rect,
    y: i32,
    track: usize,
) -> Option<&OctamedicPatternTrack> {
    let row = (y - area.y) / CHAR_HEIGHT;
    let line = usize::try_from(cursor_line as isize + (row - center_row(area)) as isize).ok()?;
    return pattern.lines.get(line)?.tracks.get(track);
}

// OctaMED ORs the scope bitplanes into the screen, dots over text go the paler pink like the
// cursor
fn blend_text(text: &str, x: i32, line: usize, font: Font, run: Rect, painter: &mut Painter) {
    for (start, width) in font.ink(text, line) {
        let left = (x + start).max(run.x);
        let right = (x + start + width).min(run.x + run.w);
        if left < right {
            painter.fill(Rect::new(left, run.y, right - left, run.h), pens::BLUSH);
        }
    }
}

// no note but an instrument is a hold, which keeps the note before going
fn note_column_text(cell: &OctamedicPatternTrack) -> String {
    if cell.note.value == 0 && cell.instrument_id.0 != 0 {
        return HOLD_TEXT.to_owned();
    }
    return note_text(cell.note.value);
}

fn track_text(cell: &OctamedicPatternTrack) -> String {
    return format!(
        "{}{}{:02X}{:02X}",
        note_column_text(cell),
        instrument_text(cell.instrument_id.0),
        cell.command_id.0,
        cell.command_value
    );
}

#[cfg(test)]
mod tests {
    use ggez::input::keyboard::KeyMods;
    use octamedic_core::{
        data::{
            command::{CommandId, SET_VOLUME},
            instrument::InstrumentId,
            note::OctamedicNote,
            project::OctamedicProject,
        },
        engine::handle::EngineHandle,
    };
    use rustuition::target::{RenderTarget, Rgba};

    use super::*;
    use crate::app::{DEFAULT_VIDEO, screen, state::screen_palette};

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    #[derive(Default)]
    struct Fills(Vec<Rect>);

    impl RenderTarget for Fills {
        fn fill(&mut self, rect: Rect, _color: Rgba) {
            self.0.push(rect);
        }

        fn get_clip(&self) -> Rect {
            return screen(DEFAULT_VIDEO).rect();
        }

        fn set_clip(&mut self, _rect: Rect) -> bool {
            return true;
        }
    }

    #[test]
    fn scope_meter_positions() {
        let area = Rect::new(0, 77, 640, 120);
        let layout = Layout::new(area, 4);
        // two pixels a sample, starting 4 pixels into the track, from the highlighted line down
        assert_eq!(scope_area(&layout, area, 0), Rect::new(68, 132, 84, 65));
        // 90 pixels in, from the editor's middle down
        assert_eq!(meter_area(&layout, area, 1), Rect::new(298, 137, 20, 60));
    }

    #[test]
    fn silent_meter_bottom() {
        let (mut editor, state) = (BlockEditor::new(), state());
        let area = Rect::new(0, 77, 640, 120);
        let layout = Layout::new(area, 4);
        let mut fills = Fills::default();
        let mut painter = Painter::new(&mut fills, &state.palette);
        let pattern = state.active_pattern().unwrap().clone();
        editor.render_scopes(&state, &pattern, &layout, area, &mut painter);
        assert!(
            fills.0.contains(&Rect::new(156, 196, 16, 1)),
            "{:?}",
            fills.0
        );
    }

    #[test]
    fn right_drag_makes_a_range() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        let area = Rect::new(0, 77, 640, 120);
        let mut fills = Fills::default();
        editor.render(&state, area, &mut Painter::new(&mut fills, &state.palette));
        // 16px chars, the tracks 9 apart after the line numbers' 4, and line 0 on the middle row
        let at = |track: i32, line: i32| Point::new(64 + 144 * track + 10, 77 + 8 * (7 + line) + 4);
        let right = MouseButton::Right;
        let line_numbers = Event::MouseDown {
            pos: Point::new(10, 133),
            button: right,
        };
        assert!(!editor.handle_event(&mut state, &line_numbers));
        let press = Event::MouseDown {
            pos: at(1, 2),
            button: right,
        };
        assert!(editor.handle_event(&mut state, &press));
        state.right_button_held = true;
        let moved = Event::MouseMove { pos: at(2, 4) };
        assert!(!editor.handle_event(&mut state, &moved));
        let release = Event::MouseUp {
            pos: at(2, 4),
            button: right,
        };
        assert!(editor.handle_event(&mut state, &release));
        let range = state.active_range().unwrap();
        assert_eq!((range.tracks(), range.lines()), (1..3, 2..5));

        let mut fills = Fills::default();
        editor.render(&state, area, &mut Painter::new(&mut fills, &state.palette));
        let first_row = Rect::new(64 + 144, 77 + 8 * 9, 2 * 144, 8);
        assert!(fills.0.contains(&first_row), "{:?}", fills.0);
    }

    #[test]
    fn a_drag_let_go_elsewhere_stops() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        let area = Rect::new(0, 77, 640, 120);
        let mut fills = Fills::default();
        editor.render(&state, area, &mut Painter::new(&mut fills, &state.palette));
        let press = Event::MouseDown {
            pos: Point::new(74, 133),
            button: MouseButton::Right,
        };
        editor.handle_event(&mut state, &press);
        // let go where the editor didn't hear it
        let moved = Event::MouseMove {
            pos: Point::new(400, 180),
        };
        editor.handle_event(&mut state, &moved);
        let range = state.active_range().unwrap();
        assert_eq!((range.tracks(), range.lines()), (0..1, 0..1));
        assert!(!editor.dragging_range);
    }

    #[test]
    fn e2_hides_meters() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.meters.vu = false;
        let area = Rect::new(0, 77, 640, 120);
        let layout = Layout::new(area, 4);
        let mut fills = Fills::default();
        let mut painter = Painter::new(&mut fills, &state.palette);
        let pattern = state.active_pattern().unwrap().clone();
        editor.render_scopes(&state, &pattern, &layout, area, &mut painter);
        assert!(fills.0.is_empty(), "{:?}", fills.0);
    }

    #[test]
    fn scope_blends_with_text() {
        let mut fills = Fills::default();
        let palette = screen_palette(&OctamedicProject::new().palette());
        let mut painter = Painter::new(&mut fills, &palette);
        let ink: Vec<(i32, i32)> = fonts::EDITOR.ink("0", 0).collect();
        blend_text(
            "0",
            100,
            0,
            fonts::EDITOR,
            Rect::new(100, 50, 16, 1),
            &mut painter,
        );
        let expected: Vec<Rect> = ink
            .iter()
            .map(|&(x, w)| Rect::new(100 + x, 50, w, 1))
            .collect();
        assert!(!expected.is_empty());
        assert_eq!(fills.0, expected);
    }

    fn press(editor: &mut BlockEditor, state: &mut AppState, key: KeyCode, mods: KeyMods) {
        let event = Event::Key {
            key,
            mods,
            repeat: false,
        };
        assert!(editor.handle_event(state, &event), "{key:?} {mods:?}");
    }

    #[test]
    fn fkeys_jump() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        // the default block has 64 lines
        let keys = [
            (KeyCode::F7, 16),
            (KeyCode::F8, 32),
            (KeyCode::F9, 48),
            (KeyCode::F10, 63),
            (KeyCode::F6, 0),
        ];
        for (key, line) in keys {
            press(&mut editor, &mut state, key, KeyMods::NONE);
            assert_eq!(state.cursor.line, line, "{key:?}");
        }
    }

    #[test]
    fn spacing_moves_by_the_line_add() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.line_add = 4;
        press(&mut editor, &mut state, KeyCode::Down, KeyMods::NONE);
        assert_eq!(state.cursor.line, 1);
        state.spacing = true;
        press(&mut editor, &mut state, KeyCode::Down, KeyMods::NONE);
        assert_eq!(state.cursor.line, 5);
        press(&mut editor, &mut state, KeyCode::Up, KeyMods::NONE);
        assert_eq!(state.cursor.line, 1);
        state.cursor.edit_mode = true;
        editor.handle_event(&mut state, &Event::text('z'));
        assert_eq!(state.cursor.line, 5);
        // never 0, even if it's been set that way
        state.line_add = 0;
        assert_eq!(state.line_step(), 1);
    }

    #[test]
    fn notes_need_edit_mode() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        assert!(editor.handle_event(&mut state, &Event::text('z')));
        assert!(state.is_saved());
        state.cursor.edit_mode = true;
        state.line_add = 2;
        state.spacing = true;
        assert!(editor.handle_event(&mut state, &Event::text('z')));
        let cell = *state.active_pattern().unwrap().track(0, 0).unwrap();
        // z is C on the bottom row, octave 1 to start with
        assert_eq!(cell.note.value, 1);
        assert_eq!(state.cursor.line, 2);
        state.cursor.column = 1;
        assert!(!editor.handle_event(&mut state, &Event::text('z')));
    }

    #[test]
    fn digits_go_where_the_cursor_is() {
        let cell = OctamedicPatternTrack::new();
        let typed = |cell, column, key| type_digit(cell, column, key).unwrap();
        let cell = typed(cell, 1, 'v');
        assert_eq!(cell.instrument_id, InstrumentId(31));
        let cell = typed(cell, 2, 'c');
        let cell = typed(cell, 5, '7');
        assert_eq!(
            (cell.command_id, cell.command_value),
            (CommandId(0xC0), 0x07)
        );
        let cell = typed(cell, 3, 'F');
        let cell = typed(cell, 4, '4');
        assert_eq!(
            (cell.command_id, cell.command_value),
            (CommandId(0xCF), 0x47)
        );
        // an instrument without a note is a hold
        assert_eq!(track_text(&cell), "-|- VCF47");
    }

    #[test]
    fn digits_that_dont_fit() {
        let mut cell = OctamedicPatternTrack::new();
        // 0 to 9 and A to V for the instrument
        assert!(type_digit(cell, 1, 'w').is_none());
        // typing replaces a higher instrument
        cell.instrument_id = InstrumentId(40);
        let typed = type_digit(cell, 1, '3').unwrap();
        assert_eq!(typed.instrument_id, InstrumentId(3));
        assert!(type_digit(cell, 2, 'g').is_none());
        assert!(type_digit(cell, 6, '1').is_none());
        assert!(type_digit(cell, 0, '1').is_none());
    }

    #[test]
    fn tab_highlights_the_line() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.cursor.line = 3;
        press(&mut editor, &mut state, KeyCode::Tab, KeyMods::NONE);
        assert!(state.active_pattern().unwrap().lines[3].highlighted);
        assert!(!state.is_saved());
        press(&mut editor, &mut state, KeyCode::Tab, KeyMods::NONE);
        assert!(!state.active_pattern().unwrap().lines[3].highlighted);
        assert_eq!(pens::highlight(Pen::BACKGROUND), Pen::SHINE);
        assert_eq!(pens::highlight(pens::PINK), pens::BLUSH);
    }

    #[test]
    fn alt_numbers_type_a_to_f() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.cursor.edit_mode = true;
        state.cursor.column = 2;
        press(&mut editor, &mut state, KeyCode::Key3, KeyMods::ALT);
        let cell = *state.active_pattern().unwrap().track(0, 0).unwrap();
        assert_eq!(cell.command_id, CommandId(0xD0));
        // past F there's no digit, and the instrument column doesn't take it
        let alt = |key| Event::Key {
            key,
            mods: KeyMods::ALT,
            repeat: false,
        };
        state.cursor.column = 3;
        assert!(!editor.handle_event(&mut state, &alt(KeyCode::Key7)));
        state.cursor.column = 1;
        assert!(!editor.handle_event(&mut state, &alt(KeyCode::Key1)));
    }

    #[test]
    fn typing_commands_in_edit_mode() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.cursor.column = 2;
        assert!(!editor.handle_event(&mut state, &Event::text('9')));
        state.cursor.edit_mode = true;
        assert!(editor.handle_event(&mut state, &Event::text('9')));
        let cell = *state.active_pattern().unwrap().track(0, 0).unwrap();
        assert_eq!(cell.command_id, CommandId(0x90));
        assert_eq!(state.cursor.line, 1);
        assert!(!state.is_saved());
    }

    #[test]
    fn enter_puts_in_a_hold() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.set_instrument(5);
        press(&mut editor, &mut state, KeyCode::Return, KeyMods::NONE);
        assert!(state.is_saved());
        state.cursor.edit_mode = true;
        press(&mut editor, &mut state, KeyCode::Return, KeyMods::NONE);
        let cell = *state.active_pattern().unwrap().track(0, 0).unwrap();
        assert_eq!((cell.note.value, cell.instrument_id), (0, InstrumentId(5)));
        assert_eq!(track_text(&cell), "-|- 50000");
        assert_eq!(state.cursor.line, 1);
        // A does it too
        assert!(editor.handle_event(&mut state, &Event::text('A')));
        assert_eq!(state.cursor.line, 2);
        // only in the note column
        state.cursor.column = 2;
        press(&mut editor, &mut state, KeyCode::Return, KeyMods::NONE);
        assert_eq!(state.cursor.line, 2);
    }

    #[test]
    fn typing_an_instrument_alone_makes_a_hold() {
        let cell = type_digit(OctamedicPatternTrack::new(), 1, '3').unwrap();
        assert_eq!(track_text(&cell), "-|- 30000");
        assert_eq!(track_text(&OctamedicPatternTrack::new()), "--- 00000");
    }

    #[test]
    fn play_on_note_starts_block() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.play_on_note = true;
        editor.handle_event(&mut state, &Event::text('z'));
        assert_eq!(state.started(), None);
        state.cursor.edit_mode = true;
        editor.handle_event(&mut state, &Event::text('z'));
        assert_eq!(state.started(), Some(PlayStart::Block));
    }

    #[test]
    fn held_keys() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.cursor.edit_mode = true;
        let held = Event::Text {
            character: 'z',
            mods: KeyMods::NONE,
            repeat: true,
        };
        // repeats like a new press, like OctaMED
        assert!(editor.handle_event(&mut state, &held));
        assert!(editor.handle_event(&mut state, &held));
        assert_eq!(state.cursor.line, 2);
        let pattern = state.active_pattern().unwrap();
        assert_eq!(pattern.track(1, 0).unwrap().note.value, 1);
        let with_super = Event::Text {
            character: 'z',
            mods: KeyMods::LOGO,
            repeat: false,
        };
        assert!(!editor.handle_event(&mut state, &with_super));
        state.cursor.column = 1;
        assert!(!editor.handle_event(&mut state, &held));
    }

    #[test]
    fn typing_a_chord() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.chord_mode = true;
        state.cursor.edit_mode = true;
        let mut type_key = |state: &mut AppState, character, repeat| {
            let event = Event::Text {
                character,
                mods: KeyMods::NONE,
                repeat,
            };
            return editor.handle_event(state, &event);
        };
        assert!(type_key(&mut state, 'q', false));
        assert!(type_key(&mut state, 'q', true));
        assert!(type_key(&mut state, 'e', false));
        state.end_chord();
        let notes = |state: &AppState, line: usize| {
            let tracks = &state.active_pattern().unwrap().lines[line].tracks;
            return tracks
                .iter()
                .map(|cell| cell.note.value)
                .collect::<Vec<_>>();
        };
        let [c, e] = ['q', 'e'].map(|key| state.key_to_note(key).unwrap().value);
        assert_eq!(notes(&state, 0), [c, e, 0, 0]);
        state.cursor = EditCursor {
            edit_mode: true,
            line: 3,
            ..EditCursor::default()
        };
        press(&mut editor, &mut state, KeyCode::Return, KeyMods::SHIFT);
        assert_eq!(notes(&state, 3), [0; 4]);
        let instrument = state.active_instrument;
        let held = &state.active_pattern().unwrap().lines[3].tracks;
        assert!(
            held[..2]
                .iter()
                .all(|cell| cell.instrument_id == instrument)
        );
    }

    #[test]
    fn entering_follows_the_advance() {
        use crate::app::advance::{CommandAdvance, NoteAdvance};
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.cursor.edit_mode = true;
        state.note_advance = NoteAdvance::TrackRight;
        assert!(editor.handle_event(&mut state, &Event::text('z')));
        assert_eq!((state.cursor.line, state.cursor.track), (0, 1));
        state.cursor.column = 2;
        state.command_advance = Some(CommandAdvance::Right);
        assert!(editor.handle_event(&mut state, &Event::text('1')));
        assert_eq!((state.cursor.track, state.cursor.column), (1, 3));
    }

    #[test]
    fn space_deletes_with_space_del() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.cursor.edit_mode = true;
        state.enter_note(0, 0, OctamedicNote { value: 13 });
        assert!(!editor.handle_event(&mut state, &Event::text(' ')));
        state.space_deletes = true;
        assert!(editor.handle_event(&mut state, &Event::text(' ')));
        let pattern = state.active_pattern().unwrap();
        assert_eq!(pattern.lines[0].tracks[0].note.value, 0);
        assert_eq!(state.cursor.line, 1);
    }

    #[test]
    fn ctrl_z_and_ctrl_y_undo_and_redo() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.enter_note(0, 0, OctamedicNote { value: 13 });
        state.update();
        let note = |state: &AppState| state.active_pattern().unwrap().lines[0].tracks[0].note;
        press(&mut editor, &mut state, KeyCode::Z, KeyMods::CTRL);
        assert_eq!(note(&state).value, 0);
        press(&mut editor, &mut state, KeyCode::Y, KeyMods::CTRL);
        assert_eq!(note(&state).value, 13);
        press(&mut editor, &mut state, KeyCode::Z, KeyMods::CTRL);
        press(
            &mut editor,
            &mut state,
            KeyCode::Z,
            KeyMods::CTRL | KeyMods::SHIFT,
        );
        assert_eq!(note(&state).value, 13);
    }

    #[test]
    fn arrows_wrap() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        press(&mut editor, &mut state, KeyCode::Up, KeyMods::NONE);
        assert_eq!(state.cursor.line, 63);
        press(&mut editor, &mut state, KeyCode::Left, KeyMods::NONE);
        assert_eq!((state.cursor.track, state.cursor.column), (3, 5));
        press(&mut editor, &mut state, KeyCode::Right, KeyMods::ALT);
        assert_eq!((state.cursor.track, state.cursor.column), (0, 5));
        editor.visible_tracks = 4;
        press(&mut editor, &mut state, KeyCode::Right, KeyMods::CTRL);
        assert_eq!(state.cursor.track, 3);
    }

    #[test]
    fn view_follows_cursor() {
        let mut editor = BlockEditor::new();
        editor.scroll_to(10, 16, 8);
        assert_eq!(editor.first_track, 3);
        editor.scroll_to(1, 16, 8);
        assert_eq!(editor.first_track, 1);
        editor.scroll_to(15, 16, 8);
        assert_eq!(editor.first_track, 8);
    }

    #[test]
    fn note_names() {
        assert_eq!(note_text(0), "---");
        assert_eq!(note_text(1), "C-1");
        assert_eq!(note_text(14), "C#2");
        assert_eq!(note_text(36), "B-3");
    }

    #[test]
    fn instrument_base32() {
        assert_eq!(instrument_text(0), " 0");
        assert_eq!(instrument_text(9), " 9");
        assert_eq!(instrument_text(10), " A");
        assert_eq!(instrument_text(31), " V");
        assert_eq!(instrument_text(32), "10");
        assert_eq!(instrument_text(63), "1V");
    }

    #[test]
    fn cell_width() {
        let cell = OctamedicPatternTrack {
            note: OctamedicNote { value: 14 },
            instrument_id: InstrumentId(1),
            command_id: CommandId(0x0C),
            command_value: 0x20,
        };
        assert_eq!(track_text(&cell), "C#2 10C20");
        assert_eq!(track_text(&OctamedicPatternTrack::new()), "--- 00000");
    }

    fn mouse_down(x: i32, button: MouseButton) -> Event {
        return Event::MouseDown {
            pos: Point::new(x, 100),
            button,
        };
    }

    #[test]
    fn track_under_pointer() {
        let layout = Layout::new(Rect::new(0, 77, 640, 120), 4);
        assert_eq!(layout.track_at(layout.x), None);
        assert_eq!(layout.track_at(layout.track_x(0)), Some(0));
        assert_eq!(layout.track_at(layout.track_x(1) - 1), Some(0));
        assert_eq!(layout.track_at(layout.track_x(3) + 5), Some(3));
        assert_eq!(layout.track_at(layout.track_x(4)), None);
    }

    #[test]
    fn click_toggles_mute() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        editor.area = Rect::new(0, 77, 640, 120);
        editor.clickable.set_area(editor.area);
        let x = Layout::new(editor.area, 4).track_x(2) + 3;
        assert!(editor.handle_event(&mut state, &mouse_down(x, MouseButton::Left)));
        assert!(state.is_track_muted(2));
        assert!(!state.is_track_muted(1));
        editor.handle_event(
            &mut state,
            &Event::MouseUp {
                pos: Point::new(x, 100),
                button: MouseButton::Left,
            },
        );
        editor.handle_event(&mut state, &mouse_down(x, MouseButton::Left));
        assert!(!state.is_track_muted(2));
    }

    #[test]
    fn right_click_left_alone() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        editor.area = Rect::new(0, 77, 640, 120);
        editor.clickable.set_area(editor.area);
        let x = Layout::new(editor.area, 4).track_x(0);
        assert!(!editor.handle_event(&mut state, &mouse_down(x, MouseButton::Right)));
        assert!(!state.is_track_muted(0));
    }

    fn cell_at(state: &AppState, line: usize, track: usize) -> OctamedicPatternTrack {
        return *state.active_pattern().unwrap().track(line, track).unwrap();
    }

    fn filled(note: u8) -> OctamedicPatternTrack {
        return OctamedicPatternTrack {
            note: OctamedicNote { value: note },
            instrument_id: InstrumentId(2),
            command_id: CommandId(0x0C),
            command_value: 0x34,
        };
    }

    // lines 0 to 3 of track 0 hold notes 1 to 4, all with an instrument and a command
    fn filled_state() -> AppState {
        let mut state = state();
        for line in 0..4 {
            state.edit_track(line, 0, |cell| *cell = filled(line as u8 + 1));
        }
        state.cursor.edit_mode = true;
        return state;
    }

    #[test]
    fn del_clears_the_note_or_a_digit() {
        let (mut editor, mut state) = (BlockEditor::new(), filled_state());
        press(&mut editor, &mut state, KeyCode::Delete, KeyMods::NONE);
        let cleared = OctamedicPatternTrack {
            note: OctamedicNote::new(),
            instrument_id: InstrumentId(0),
            ..filled(1)
        };
        assert_eq!(cell_at(&state, 0, 0), cleared);
        assert_eq!(state.cursor.line, 1, "down like entering a note");
        // on the instrument column only the instrument goes
        state.cursor.column = 1;
        press(&mut editor, &mut state, KeyCode::Delete, KeyMods::NONE);
        assert_eq!(cell_at(&state, 1, 0).instrument_id, InstrumentId(0));
        assert_eq!(cell_at(&state, 1, 0).note.value, 2);
        // on the value's low digit only that digit goes
        state.cursor.column = 5;
        press(&mut editor, &mut state, KeyCode::Delete, KeyMods::NONE);
        assert_eq!(cell_at(&state, 2, 0).command_value, 0x30);
        assert_eq!(cell_at(&state, 2, 0).note.value, 3);
        // shift takes the whole cell, whichever column
        press(&mut editor, &mut state, KeyCode::Delete, KeyMods::SHIFT);
        assert_eq!(cell_at(&state, 3, 0), OctamedicPatternTrack::new());
    }

    #[test]
    fn backspace_closes_and_opens_gaps_in_the_track() {
        let (mut editor, mut state) = (BlockEditor::new(), filled_state());
        state.edit_track(2, 1, |cell| *cell = filled(9));
        state.cursor.line = 1;
        press(&mut editor, &mut state, KeyCode::Back, KeyMods::NONE);
        let notes = |state: &AppState| {
            return (0..4)
                .map(|line| cell_at(state, line, 0).note.value)
                .collect::<Vec<_>>();
        };
        assert_eq!(notes(&state), [1, 3, 4, 0]);
        assert_eq!(cell_at(&state, 2, 1), filled(9), "other tracks stay put");
        assert_eq!(state.cursor.line, 1);
        press(&mut editor, &mut state, KeyCode::Back, KeyMods::SHIFT);
        assert_eq!(notes(&state), [1, 0, 3, 4]);
        assert_eq!(state.active_pattern().unwrap().line_count(), 64);
    }

    #[test]
    fn deleting_needs_edit_mode() {
        let (mut editor, mut state) = (BlockEditor::new(), filled_state());
        state.cursor.edit_mode = false;
        for (key, mods) in [
            (KeyCode::Delete, KeyMods::NONE),
            (KeyCode::Delete, KeyMods::SHIFT),
            (KeyCode::Back, KeyMods::NONE),
            (KeyCode::Back, KeyMods::SHIFT),
        ] {
            press(&mut editor, &mut state, key, mods);
        }
        assert_eq!(cell_at(&state, 0, 0), filled(1));
        assert_eq!(state.cursor.line, 0);
    }

    #[test]
    fn alt_backspace_deletes_and_inserts_tracks() {
        let (mut editor, mut state) = (BlockEditor::new(), filled_state());
        state.cursor.edit_mode = false;
        press(
            &mut editor,
            &mut state,
            KeyCode::Back,
            KeyMods::SHIFT | KeyMods::ALT,
        );
        assert_eq!(cell_at(&state, 0, 0), OctamedicPatternTrack::new());
        assert_eq!(cell_at(&state, 0, 1), filled(1));
        press(&mut editor, &mut state, KeyCode::Back, KeyMods::ALT);
        assert_eq!(cell_at(&state, 0, 0), filled(1));
        assert_eq!(state.active_pattern().unwrap().track_count(), 4);
    }

    #[test]
    fn amiga_o_slides_the_volume() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        for (line, argument) in [(0, 0x60), (4, 0x20)] {
            let cell = &mut state.active_pattern_mut().unwrap().lines[line].tracks[0];
            cell.command_id = CommandId(SET_VOLUME);
            cell.command_value = argument;
        }
        state.cursor.line = 2;
        press(&mut editor, &mut state, KeyCode::O, AMIGA);
        // decimal 60 down to 20 over 4 lines
        let arguments: Vec<u8> = (0..=4)
            .map(|line| cell_at(&state, line, 0).command_value)
            .collect();
        assert_eq!(arguments, [0x60, 0x50, 0x40, 0x30, 0x20]);
    }
}
