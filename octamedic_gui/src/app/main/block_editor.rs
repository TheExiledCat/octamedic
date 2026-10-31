use std::array;

use ggez::input::keyboard::{KeyCode, KeyMods};
use octamed::utility::note::SEMITONES_PER_OCTAVE;
use octamedic_core::{
    amiga::paula::SCOPE_CHANNELS,
    data::{
        pattern::{OctamedicPattern, OctamedicPatternTrack},
        track_count::TrackCount,
    },
};
use octamedic_util::{base32, primitives::Flip};

use crate::{
    app::{cursor::EditCursor, state::AppState},
    framework::{
        event::Event,
        font::Font,
        keymap::{Action, Key, KeyActions, Keymap},
        painter::{CHAR_HEIGHT, CHAR_WIDTH, Painter, Pen},
        rect::Rect,
        widget::OctamedicWidget,
        widgets::audio_view::{Oscilloscope, VuMeter},
    },
};

// odd, so there is a middle row for the cursor's line
const ROWS: i32 = 15;

// measured from OctaMED's 4-track view, in font pixels: one sample each, starting 2 into the
// track, over the editor's bottom 64 lines
const SCOPE_OFFSET: i32 = 2;
const SCOPE_SAMPLES: usize = 42;
const SCOPE_HEIGHT: i32 = 64;

// the same, for the meters, which reach up to the editor's middle when full
const METER_OFFSET: i32 = 45;
const METER_WIDTH: i32 = 10;

const CENTER_ROW: i32 = ROWS / 2;

// 3 digits and a space
const LINE_NUMBER_CHARS: i32 = 4;

const NOTE_CHARS: i32 = 3;

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
    FirstBlock,
    LastBlock,
    PreviousBlock,
    NextBlock,
    FirstLine,
    SecondQuarter,
    Middle,
    LastQuarter,
    LastLine,
    TrackLeft,
    TrackRight,
    ScreenLeft,
    ScreenRight,
    ToggleEdit,
    // the bottom row of note keys; the top row plays the octave above
    Octave(u8),
}

impl Action for EditorAction {
    fn repeats(self) -> bool {
        return self != EditorAction::ToggleEdit;
    }
}

pub struct BlockEditor {
    keys: Keymap<EditorAction>,
    first_track: usize,
    // as of the last render
    visible_tracks: usize,
    oscilloscopes: [Oscilloscope; SCOPE_CHANNELS],
    meters: [VuMeter; SCOPE_CHANNELS],
}

impl BlockEditor {
    pub fn new() -> Self {
        use EditorAction::*;
        let keys = Keymap::new("editor")
            .bind(Key::plain(KeyCode::Up), Up, "Up")
            .bind(Key::plain(KeyCode::Down), Down, "Down")
            .bind(Key::plain(KeyCode::Left), Left, "Left")
            .bind(Key::plain(KeyCode::Right), Right, "Right")
            .bind(Key::alt(KeyCode::Up), FirstBlock, "First block")
            .bind(Key::alt(KeyCode::Down), LastBlock, "Last block")
            .bind(Key::shift(KeyCode::Up), PreviousBlock, "Prev block")
            .bind(Key::shift(KeyCode::Down), NextBlock, "Next block")
            .bind(Key::plain(KeyCode::F6), FirstLine, "First line")
            .bind(Key::plain(KeyCode::F7), SecondQuarter, "2nd quarter")
            .bind(Key::plain(KeyCode::F8), Middle, "Middle")
            .bind(Key::plain(KeyCode::F9), LastQuarter, "Last quarter")
            .bind(Key::plain(KeyCode::F10), LastLine, "Last line")
            .bind(Key::alt(KeyCode::Left), TrackLeft, "Track left")
            .bind(Key::alt(KeyCode::Right), TrackRight, "Track right")
            .bind(Key::ctrl(KeyCode::Left), ScreenLeft, "Screen left")
            .bind(Key::ctrl(KeyCode::Right), ScreenRight, "Screen right")
            .bind(Key::plain(KeyCode::Escape), ToggleEdit, "Edit")
            .bind(Key::plain(KeyCode::F1), Octave(1), "Octave 1")
            .bind(Key::plain(KeyCode::F2), Octave(2), "Octave 2")
            .bind(Key::plain(KeyCode::F3), Octave(3), "Octave 3")
            .bind(Key::plain(KeyCode::F4), Octave(4), "Octave 4")
            .bind(Key::plain(KeyCode::F5), Octave(5), "Octave 5");
        return Self {
            keys,
            first_track: 0,
            visible_tracks: 1,
            oscilloscopes: array::from_fn(|_| Oscilloscope::new(SCOPE_SAMPLES)),
            meters: array::from_fn(|_| VuMeter::default()),
        };
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
            let x = end - Font::Interface.width(&text);
            painter.text_in_font(x, y, &text, Pen::PINK, Font::Interface);
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
        let line = cursor_line as isize + (row - CENTER_ROW) as isize;
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
            digit => (NOTE_CHARS + digit as i32 - 1, 1),
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

    // OctaMED draws the cursor by setting the red bitplane, so its colour depends on what's
    // under it; the text under each part is redrawn in the matching colour.
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
            (parts.top, CENTER_ROW - 1, Pen::ROSE),
            (parts.bottom, CENTER_ROW + 1, Pen::ROSE),
        ];
        for (rect, row, text) in lines {
            painter.fill(rect, Pen::RED);
            painter.clipped(rect, |painter| {
                self.render_row(pattern, cursor_line, row, layout, area, text, painter);
            });
        }
        for side in parts.sides {
            painter.fill(side, Pen::PINK);
            painter.clipped(side, |painter| {
                let pen = Pen::BLUSH;
                self.render_row(pattern, cursor_line, CENTER_ROW, layout, area, pen, painter);
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
            if state.is_playing() {
                let x = layout.track_x(index);
                let scope = scope_area(layout, area, index);
                for run in self.oscilloscopes[track].runs(scope) {
                    painter.fill(run, Pen::PINK);
                    if let Some(cell) = cell_at(pattern, state.cursor.line, area, run.y, track) {
                        let text_line = ((run.y - area.y) % CHAR_HEIGHT) as usize;
                        blend_text(&track_text(cell), x, text_line, layout.font, run, painter);
                    }
                }
            }
            self.meters[track].render(state, meter_area(layout, area, index), painter);
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
        let block = state.active_pattern.0 as usize;
        let screen = self.visible_tracks as isize;
        let cursor = &mut state.cursor;
        match action {
            Up => cursor.move_lines(-1, lines),
            Down => cursor.move_lines(1, lines),
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
            ToggleEdit => cursor.edit_mode.flip(),
            FirstBlock => state.select_block(0),
            LastBlock => state.select_block(usize::MAX),
            PreviousBlock => state.select_block(block.saturating_sub(1)),
            NextBlock => state.select_block(block + 1),
            Octave(octave) => state.set_octave(octave),
        }
    }

    fn describe(action: EditorAction) -> &'static str {
        use EditorAction::*;
        return match action {
            Up => "Up one line",
            Down => "Down one line",
            Left => "Left one space",
            Right => "Right one space",
            FirstBlock => "Go to the first block",
            LastBlock => "Go to the last block",
            PreviousBlock => "Go to the previous block",
            NextBlock => "Go to the next block",
            FirstLine => "Go to the first line of the block",
            SecondQuarter => "Go to the second quarter of the block",
            Middle => "Go to the middle of the block",
            LastQuarter => "Go to the last quarter of the block",
            LastLine => "Go to the last line of the block",
            TrackLeft => "One track left",
            TrackRight => "One track right",
            ScreenLeft => "One screen left",
            ScreenRight => "One screen right",
            ToggleEdit => "Switch edit mode on or off",
            Octave(_) => "Play notes from this octave up",
        };
    }
}

impl OctamedicWidget for BlockEditor {
    fn update(&mut self, state: &mut AppState) {
        if !state.is_playing() {
            self.meters = array::from_fn(|_| VuMeter::default());
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
        if self.dispatch(state, event) {
            return true;
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
        return enter_key(state, character, repeat);
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::BLACK);
        let Some(pattern) = state.active_pattern() else {
            return;
        };
        let cursor = state.cursor;
        let layout = Layout::new(area, pattern.track_count());
        self.visible_tracks = layout.fits;
        self.scroll_to(cursor.track, pattern.track_count(), layout.tracks);
        let center_y = area.y + CENTER_ROW * CHAR_HEIGHT;
        let band = active_band(area, center_y);
        painter.fill(band, Pen::SHADOW);
        for row in 0..ROWS {
            let pen = if row == CENTER_ROW {
                Pen::FILL
            } else {
                Pen::LIGHT
            };
            self.render_row(pattern, cursor.line, row, &layout, area, pen, painter);
        }
        // one font pixel wide
        let width = layout.char_width / CHAR_WIDTH;
        for track in 1..layout.tracks {
            let x = layout.track_x(track) - 1;
            painter.fill(Rect::new(x, area.y, width, area.h), Pen::PINK);
            // OctaMED draws the band by flipping a bitplane, which turns the pink red
            painter.fill(Rect::new(x, band.y, width, band.h), Pen::RED);
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
            Font::Editor
        } else {
            Font::Topaz
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
}

// in edit mode a note key also writes the note and moves down by the line add
fn enter_key(state: &mut AppState, key: char, repeat: bool) -> bool {
    let cursor = state.cursor;
    if cursor.column != 0 {
        return false;
    }
    // holding a note key plays and writes its note once
    if repeat {
        return state.key_to_note(key).is_some();
    }
    if !cursor.edit_mode {
        return state.play_key(cursor.track, key).is_some();
    }
    if state.enter_key(cursor.track, cursor.line, key).is_none() {
        return false;
    }
    let lines = state
        .active_pattern()
        .map_or(0, |pattern| pattern.line_count());
    state.cursor.move_lines(state.line_add as isize, lines);
    return true;
}

fn note_text(note: u8) -> String {
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
    let x = layout.track_x(index);
    let w = SCOPE_SAMPLES as i32 * pixel;
    return Rect::new(
        x + SCOPE_OFFSET * pixel,
        area.y + area.h - SCOPE_HEIGHT,
        w,
        SCOPE_HEIGHT,
    );
}

fn meter_area(layout: &Layout, area: Rect, index: usize) -> Rect {
    let pixel = layout.char_width / CHAR_WIDTH;
    let x = layout.track_x(index);
    let top = area.h / 2;
    return Rect::new(
        x + METER_OFFSET * pixel,
        area.y + top,
        METER_WIDTH * pixel,
        area.h - top,
    );
}

// the cell of `track` on the screen line at `y`, if that line is in the block
fn cell_at(
    pattern: &OctamedicPattern,
    cursor_line: usize,
    area: Rect,
    y: i32,
    track: usize,
) -> Option<&OctamedicPatternTrack> {
    let row = (y - area.y) / CHAR_HEIGHT;
    let line = usize::try_from(cursor_line as isize + (row - CENTER_ROW) as isize).ok()?;
    return pattern.lines.get(line)?.tracks.get(track);
}

// OctaMED ORs the scope's bitplanes into the screen, so where a dot crosses text's ink it turns
// the paler pink, as the cursor does
fn blend_text(text: &str, x: i32, line: usize, font: Font, run: Rect, painter: &mut Painter) {
    for (start, width) in font.ink(text, line) {
        let left = (x + start).max(run.x);
        let right = (x + start + width).min(run.x + run.w);
        if left < right {
            painter.fill(Rect::new(left, run.y, right - left, run.h), Pen::BLUSH);
        }
    }
}

fn track_text(cell: &OctamedicPatternTrack) -> String {
    return format!(
        "{}{}{:02X}{:02X}",
        note_text(cell.note.value),
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
            command::CommandId, instrument::InstrumentId, note::OctamedicNote,
            project::OctamedicProject,
        },
        engine::handle::EngineHandle,
    };

    use super::*;
    use crate::framework::{
        screen::SCREEN,
        target::{Color, RenderTarget},
    };

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    #[derive(Default)]
    struct Fills(Vec<Rect>);

    impl RenderTarget for Fills {
        fn fill(&mut self, rect: Rect, _color: Color) {
            self.0.push(rect);
        }

        fn get_clip(&self) -> Rect {
            return SCREEN;
        }

        fn set_clip(&mut self, _rect: Rect) -> bool {
            return true;
        }
    }

    #[test]
    fn scopes_and_meters_sit_where_octamed_draws_them() {
        let area = Rect::new(0, 77, 640, 120);
        let layout = Layout::new(area, 4);
        // two pixels a sample, starting 4 pixels into the track, over the bottom 64 lines
        assert_eq!(scope_area(&layout, area, 0), Rect::new(68, 133, 84, 64));
        // 90 pixels in, from the editor's middle down
        assert_eq!(meter_area(&layout, area, 1), Rect::new(298, 137, 20, 60));
    }

    #[test]
    fn silent_meters_still_show_their_bottom_line() {
        let (mut editor, state) = (BlockEditor::new(), state());
        let area = Rect::new(0, 77, 640, 120);
        let layout = Layout::new(area, 4);
        let mut fills = Fills::default();
        let palette = state.palette;
        let mut painter = Painter::new(&mut fills, &palette);
        let pattern = state.active_pattern().unwrap().clone();
        editor.render_scopes(&state, &pattern, &layout, area, &mut painter);
        assert!(
            fills.0.contains(&Rect::new(156, 196, 16, 1)),
            "{:?}",
            fills.0
        );
    }

    #[test]
    fn scope_dots_turn_paler_where_they_cross_text() {
        let mut fills = Fills::default();
        let palette = OctamedicProject::new().palette();
        let mut painter = Painter::new(&mut fills, &palette);
        let ink: Vec<(i32, i32)> = Font::Editor.ink("0", 0).collect();
        blend_text(
            "0",
            100,
            0,
            Font::Editor,
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
    fn function_keys_jump_through_the_block() {
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
    fn function_keys_set_the_octave() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        let keys = [
            KeyCode::F1,
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
        ];
        for (octave, key) in (1..).zip(keys) {
            press(&mut editor, &mut state, key, KeyMods::NONE);
            assert_eq!(state.current_octave, octave, "{key:?}");
        }
    }

    #[test]
    fn note_keys_write_notes_only_in_edit_mode() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        assert!(editor.handle_event(&mut state, &Event::text('z')));
        assert!(state.is_saved());
        state.cursor.edit_mode = true;
        state.line_add = 2;
        assert!(editor.handle_event(&mut state, &Event::text('z')));
        let cell = *state.active_pattern().unwrap().track(0, 0).unwrap();
        assert_eq!(cell.note.value, 13);
        assert_eq!(state.cursor.line, 2);
        state.cursor.column = 1;
        assert!(!editor.handle_event(&mut state, &Event::text('z')));
    }

    #[test]
    fn held_or_modified_note_keys_write_nothing() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.cursor.edit_mode = true;
        let held = Event::Text {
            character: 'z',
            mods: KeyMods::NONE,
            repeat: true,
        };
        assert!(editor.handle_event(&mut state, &held));
        let with_super = Event::Text {
            character: 'z',
            mods: KeyMods::LOGO,
            repeat: false,
        };
        assert!(!editor.handle_event(&mut state, &with_super));
        assert!(state.is_saved());
        assert_eq!(state.cursor.line, 0);
        state.cursor.column = 1;
        assert!(!editor.handle_event(&mut state, &held));
    }

    #[test]
    fn arrows_move_the_cursor_and_wrap() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        press(&mut editor, &mut state, KeyCode::Up, KeyMods::NONE);
        assert_eq!(state.cursor.line, 63);
        press(&mut editor, &mut state, KeyCode::Left, KeyMods::NONE);
        assert_eq!((state.cursor.track, state.cursor.column), (3, 6));
        press(&mut editor, &mut state, KeyCode::Right, KeyMods::ALT);
        assert_eq!((state.cursor.track, state.cursor.column), (0, 6));
        editor.visible_tracks = 4;
        press(&mut editor, &mut state, KeyCode::Right, KeyMods::CTRL);
        assert_eq!(state.cursor.track, 3);
    }

    #[test]
    fn esc_switches_edit_mode() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        press(&mut editor, &mut state, KeyCode::Escape, KeyMods::NONE);
        assert!(state.cursor.edit_mode);
        press(&mut editor, &mut state, KeyCode::Escape, KeyMods::NONE);
        assert!(!state.cursor.edit_mode);
    }

    #[test]
    fn block_keys_stop_at_the_first_and_last_block() {
        let (mut editor, mut state) = (BlockEditor::new(), state());
        state.next_pattern();
        state.next_pattern();
        state.select_block(0);
        press(&mut editor, &mut state, KeyCode::Up, KeyMods::SHIFT);
        assert_eq!(state.active_pattern.0, 0);
        press(&mut editor, &mut state, KeyCode::Down, KeyMods::SHIFT);
        assert_eq!(state.active_pattern.0, 1);
        press(&mut editor, &mut state, KeyCode::Down, KeyMods::ALT);
        assert_eq!(state.active_pattern.0, 2);
        press(&mut editor, &mut state, KeyCode::Down, KeyMods::SHIFT);
        assert_eq!(state.active_pattern.0, 2);
        press(&mut editor, &mut state, KeyCode::Up, KeyMods::ALT);
        assert_eq!(state.active_pattern.0, 0);
    }

    #[test]
    fn the_view_scrolls_to_keep_the_cursors_track_on_screen() {
        let mut editor = BlockEditor::new();
        editor.scroll_to(10, 16, 8);
        assert_eq!(editor.first_track, 3);
        editor.scroll_to(1, 16, 8);
        assert_eq!(editor.first_track, 1);
        editor.scroll_to(15, 16, 8);
        assert_eq!(editor.first_track, 8);
    }

    #[test]
    fn notes_read_like_octameds() {
        assert_eq!(note_text(0), "---");
        assert_eq!(note_text(1), "C-1");
        assert_eq!(note_text(14), "C#2");
        assert_eq!(note_text(36), "B-3");
    }

    #[test]
    fn instruments_are_base_32() {
        assert_eq!(instrument_text(0), " 0");
        assert_eq!(instrument_text(9), " 9");
        assert_eq!(instrument_text(10), " A");
        assert_eq!(instrument_text(31), " V");
        assert_eq!(instrument_text(32), "10");
        assert_eq!(instrument_text(63), "1V");
    }

    #[test]
    fn a_cell_is_nine_characters() {
        let cell = OctamedicPatternTrack {
            note: OctamedicNote { value: 14 },
            instrument_id: InstrumentId(1),
            command_id: CommandId(0x0C),
            command_value: 0x20,
        };
        assert_eq!(track_text(&cell), "C#2 10C20");
        assert_eq!(track_text(&OctamedicPatternTrack::new()), "--- 00000");
    }
}
