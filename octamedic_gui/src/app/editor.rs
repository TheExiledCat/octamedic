use ggez::input::keyboard::KeyCode;
use octamedic_core::data::{
    pattern::{OctamedicPattern, OctamedicPatternTrack},
    track_count::TrackCount,
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
    },
};

// odd, so there is a middle row for the cursor's line
const ROWS: i32 = 15;

const CENTER_ROW: i32 = ROWS / 2;

/// The line number and the space after it.
const LINE_NUMBER_CHARS: i32 = 4;

/// Note, instrument, command and its value, as in `C#1 10C20`.
const TRACK_CHARS: i32 = 9;

const NOTE_NAMES: [&str; 12] = [
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
}

impl Action for EditorAction {}

pub struct BlockEditor {
    keys: Keymap<EditorAction>,
    /// The leftmost track on screen.
    first_track: usize,
    /// How many tracks fit on screen, as last rendered.
    visible_tracks: usize,
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
            .bind(Key::plain(KeyCode::Escape), ToggleEdit, "Edit");
        return Self {
            keys,
            first_track: 0,
            visible_tracks: 1,
        };
    }

    /// Scrolls so the cursor's track is on screen.
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

    /// Over the top two rows, whatever lines are there: each track's number on the first, ending
    /// where its instrument starts, and the block's track count on the second, ending where the
    /// tracks start.
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
            label(number.to_string(), layout.track_column(index) + 4, y);
        }
        let count = TrackCount::holding(pattern.track_count());
        label(
            count.tracks().to_string(),
            LINE_NUMBER_CHARS,
            y + CHAR_HEIGHT,
        );
    }

    /// Row `row` of the screen, the line `row - CENTER_ROW` from the cursor's, if there is one.
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

    /// The box around the cursor's field, clear of its text: lines a line above and below the
    /// active band and sides down the band, a pixel wider than the field so the right side is
    /// just past it.
    fn cursor_box(&self, cursor: &EditCursor, layout: &Layout, band: Rect) -> Option<CursorBox> {
        let index = cursor.track.checked_sub(self.first_track)?;
        // the note is 3 characters, then each digit is one
        let (offset, width) = match cursor.column {
            0 => (0, 3),
            digit => (digit as i32 + 2, 1),
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

    /// OctaMED draws the cursor by setting the red bitplane over what's there, so its colour
    /// depends on what it covers: red over black, pink over the band, rose over text and blush
    /// over the active line's text. The text under each part is drawn again in its colour.
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
}

/// Where the cursor's box goes.
#[derive(Clone, Copy)]
struct CursorBox {
    top: Rect,
    bottom: Rect,
    sides: [Rect; 2],
}

/// The active line's background: from a line above its text to the bottom of the row.
fn active_band(area: Rect, center_y: i32) -> Rect {
    return Rect::new(area.x, center_y - 1, area.w, CHAR_HEIGHT + 1);
}

/// Where the line numbers and tracks go.
struct Layout {
    x: i32,
    font: Font,
    char_width: i32,
    /// How many tracks are shown.
    tracks: usize,
    /// How many tracks fit.
    fits: usize,
}

impl Layout {
    /// Up to 4 tracks are shown in double-width characters, as in OctaMED.
    fn new(area: Rect, track_count: usize) -> Self {
        let font = if track_count <= 4 {
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
            SecondQuarter => cursor.set_line(lines / 4, lines),
            Middle => cursor.set_line(lines / 2, lines),
            LastQuarter => cursor.set_line(lines * 3 / 4, lines),
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
        };
    }
}

impl OctamedicWidget for BlockEditor {
    fn update(&mut self, state: &mut AppState) {
        if state.is_playing() {
            state.cursor.line = state.engine.status().row();
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.dispatch(state, event);
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
        // a line just before each track but the first, one font pixel wide
        let width = layout.char_width / CHAR_WIDTH;
        for track in 1..layout.tracks {
            let x = layout.x + layout.track_column(track) * layout.char_width - 1;
            painter.fill(Rect::new(x, area.y, width, area.h), Pen::PINK);
            // OctaMED draws the band by flipping a bitplane, which turns the pink red
            painter.fill(Rect::new(x, band.y, width, band.h), Pen::RED);
        }
        self.render_labels(pattern, &layout, area.y, painter);
        if let Some(parts) = self.cursor_box(&cursor, &layout, band) {
            self.render_cursor(pattern, cursor.line, parts, &layout, area, painter);
        }
    }
}

fn note_text(note: u8) -> String {
    if note == 0 {
        return "---".to_string();
    }
    let index = note as usize - 1;
    let octave = std::char::from_digit((index / 12 + 1) as u32, 36).unwrap_or('?');
    return format!("{}{}", NOTE_NAMES[index % 12], octave.to_ascii_uppercase());
}

/// Two base-32 digits, with a space for a leading zero.
fn instrument_text(instrument: u8) -> String {
    return base32::format(instrument as u32, 2, ' ');
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

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
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
