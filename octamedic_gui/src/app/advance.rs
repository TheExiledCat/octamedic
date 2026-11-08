use octamedic_core::data::instrument::InstrumentId;

use crate::app::state::AppState;

// the command and value digits, after the note and instrument
const COMMAND_COLUMNS: std::ops::RangeInclusive<usize> = 2..=5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NoteAdvance {
    Up,
    #[default]
    Down,
    TrackRight,
    TrackLeft,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandAdvance {
    Right,
    Left,
}

impl AppState {
    pub fn advance_after_note(&mut self) {
        let (lines, tracks) = self.active_pattern().map_or((0, 0), |pattern| {
            (pattern.line_count(), pattern.track_count())
        });
        let step = self.line_step() as isize;
        match self.note_advance {
            NoteAdvance::Up => self.cursor.move_lines(-step, lines),
            NoteAdvance::Down => self.cursor.move_lines(step, lines),
            NoteAdvance::TrackRight => self.cursor.move_tracks(1, tracks),
            NoteAdvance::TrackLeft => self.cursor.move_tracks(-1, tracks),
        }
    }

    // without a command advance the digits move like notes do. With one the cursor stays among
    // the command digits, stopping at either end
    pub fn advance_after_digit(&mut self) {
        let step = match self.command_advance {
            Some(CommandAdvance::Right) => 1,
            Some(CommandAdvance::Left) => -1,
            None => return self.advance_after_note(),
        };
        let column = self.cursor.column.saturating_add_signed(step);
        self.cursor.column = column.clamp(*COMMAND_COLUMNS.start(), *COMMAND_COLUMNS.end());
    }

    pub fn play_line(&self, line: usize) {
        let Some(cells) = self
            .active_pattern()
            .and_then(|pattern| pattern.lines.get(line))
        else {
            return;
        };
        for (track, cell) in cells.tracks.iter().enumerate() {
            if cell.note.value == 0 {
                continue;
            }
            let instrument = match cell.instrument_id {
                InstrumentId(0) => self.active_instrument,
                instrument => instrument,
            };
            self.engine.play_note(track, cell.note, instrument);
        }
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    #[test]
    fn notes_go_the_way_advance_says() {
        let mut state = state();
        state.cursor.line = 5;
        state.advance_after_note();
        assert_eq!(state.cursor.line, 6);
        state.note_advance = NoteAdvance::Up;
        state.advance_after_note();
        assert_eq!(state.cursor.line, 5);
        state.note_advance = NoteAdvance::TrackRight;
        state.advance_after_note();
        assert_eq!((state.cursor.line, state.cursor.track), (5, 1));
        state.note_advance = NoteAdvance::TrackLeft;
        state.advance_after_note();
        state.advance_after_note();
        assert_eq!(state.cursor.track, 3);
    }

    #[test]
    fn digits_follow_the_command_advance() {
        let mut state = state();
        state.cursor.column = 2;
        state.advance_after_digit();
        assert_eq!((state.cursor.line, state.cursor.column), (1, 2));
        state.command_advance = Some(CommandAdvance::Right);
        state.advance_after_digit();
        assert_eq!((state.cursor.line, state.cursor.column), (1, 3));
        state.cursor.column = 5;
        state.advance_after_digit();
        assert_eq!(state.cursor.column, 5);
        state.command_advance = Some(CommandAdvance::Left);
        for _ in 0..4 {
            state.advance_after_digit();
        }
        assert_eq!((state.cursor.track, state.cursor.column), (0, 2));
    }
}
