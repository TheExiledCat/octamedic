use octamedic_core::data::note::OctamedicNote;

use crate::app::state::{AppState, SELECTABLE_TRACKS};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeldChord {
    pub first_track: usize,
    pub line: usize,
    pub tracks: Vec<usize>,
    pub entered: bool,
}

impl AppState {
    pub fn chord_tracks(&self, first: usize) -> Vec<usize> {
        let track_count = self
            .active_pattern()
            .map_or(0, |pattern| pattern.track_count());
        let selected = (first + 1..track_count.min(SELECTABLE_TRACKS))
            .filter(|&track| self.selected_tracks[track]);
        return std::iter::once(first)
            .filter(|&track| track < track_count)
            .chain(selected)
            .collect();
    }

    pub fn enter_chord_key(&mut self, key: char) -> bool {
        return self.add_to_chord(key, true);
    }

    pub fn play_chord_key(&mut self, key: char) -> bool {
        return self.add_to_chord(key, false);
    }

    pub fn end_chord(&mut self) {
        let Some(chord) = self.held_chord.take() else {
            return;
        };
        if chord.entered && self.chord_reset {
            self.cursor.track = chord.first_track;
            self.cursor.line = chord.line;
            self.move_down();
        }
        if !chord.tracks.is_empty() {
            self.last_chord_tracks = chord.tracks;
        }
    }

    pub fn hold_last_chord(&mut self) {
        if !self.cursor.edit_mode {
            return;
        }
        if self.last_chord_tracks.is_empty() {
            self.notify("No chord to hold");
            return;
        }
        let line = self.cursor.line;
        for track in self.last_chord_tracks.clone() {
            self.enter_note(track, line, OctamedicNote { value: 0 });
        }
        // a chord spans tracks, so it moves down whatever the note advance says, like ending one
        self.move_down();
    }

    fn add_to_chord(&mut self, key: char, enter: bool) -> bool {
        let Some(note) = self.key_to_note(key) else {
            return false;
        };
        let mut chord = self.held_chord.take().unwrap_or(HeldChord {
            first_track: self.cursor.track,
            line: self.cursor.line,
            tracks: Vec::new(),
            entered: false,
        });
        let tracks = self.chord_tracks(chord.first_track);
        let next = chord.tracks.len();
        if let Some(&track) = tracks.get(next) {
            self.play_note(track, note);
            if enter {
                self.enter_note(track, chord.line, note);
                chord.entered = true;
                self.cursor.track = tracks.get(next + 1).copied().unwrap_or(track);
            }
            chord.tracks.push(track);
        }
        self.held_chord = Some(chord);
        return true;
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;

    fn state() -> AppState {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.chord_mode = true;
        state.cursor.edit_mode = true;
        return state;
    }

    fn note(state: &AppState, key: char) -> u8 {
        return state.key_to_note(key).unwrap().value;
    }

    fn notes(state: &AppState, line: usize) -> Vec<u8> {
        let pattern = state.active_pattern().unwrap();
        return pattern.lines[line]
            .tracks
            .iter()
            .map(|cell| cell.note.value)
            .collect();
    }

    #[test]
    fn a_chord_goes_right_from_the_cursor() {
        let mut state = state();
        state.cursor.track = 1;
        // B is one note more than there are tracks left
        for key in ['q', 'e', 't', 'u'] {
            assert!(state.enter_chord_key(key));
        }
        let [c, e, g] = ['q', 'e', 't'].map(|key| note(&state, key));
        assert_eq!(notes(&state, 0), [0, c, e, g]);
        state.end_chord();
        assert_eq!((state.cursor.track, state.cursor.line), (3, 0));
        assert_eq!(state.last_chord_tracks, [1, 2, 3]);
    }

    #[test]
    fn only_the_selected_tracks_take_notes() {
        let mut state = state();
        state.selected_tracks = [false; SELECTABLE_TRACKS];
        state.selected_tracks[3] = true;
        state.enter_chord_key('q');
        state.enter_chord_key('t');
        let [c, g] = ['q', 't'].map(|key| note(&state, key));
        assert_eq!(notes(&state, 0), [c, 0, 0, g]);
    }

    #[test]
    fn rst_goes_back_and_down() {
        let mut state = state();
        state.chord_reset = true;
        state.enter_chord_key('q');
        state.enter_chord_key('e');
        state.end_chord();
        assert_eq!((state.cursor.track, state.cursor.line), (0, 1));
        state.enter_chord_key('t');
        state.end_chord();
        assert_eq!(notes(&state, 1), [note(&state, 't'), 0, 0, 0]);
    }

    #[test]
    fn playing_a_chord_enters_nothing() {
        let mut state = state();
        state.chord_reset = true;
        state.play_chord_key('q');
        state.play_chord_key('e');
        state.end_chord();
        assert_eq!(notes(&state, 0), [0; 4]);
        assert_eq!((state.cursor.track, state.cursor.line), (0, 0));
    }

    #[test]
    fn holding_the_last_chord() {
        let mut state = state();
        state.hold_last_chord();
        assert_eq!(state.notification.as_deref(), Some("No chord to hold"));
        state.cursor.track = 1;
        state.enter_chord_key('q');
        state.enter_chord_key('e');
        state.end_chord();
        state.cursor.line = 4;
        state.hold_last_chord();
        let held = &state.active_pattern().unwrap().lines[4].tracks;
        let instrument = state.active_instrument;
        assert!(
            held[1..3]
                .iter()
                .all(|cell| cell.instrument_id == instrument)
        );
        assert_eq!(held[0].instrument_id.0, 0);
        assert_eq!(state.cursor.line, 5);
    }

    #[test]
    fn keys_that_arent_notes_arent_taken() {
        let mut state = state();
        assert!(!state.enter_chord_key('p'));
        assert!(state.held_chord.is_none());
    }
}
