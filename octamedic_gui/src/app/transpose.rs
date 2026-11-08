use std::ops::Range;

use octamedic_core::{
    data::{
        instrument::InstrumentId,
        note::OctamedicNote,
        pattern::{OctamedicPatternTrack, PatternId},
    },
    engine::engine::MAX_NOTE,
};

use crate::app::state::{AppState, SELECTABLE_TRACKS};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Affect {
    #[default]
    Song,
    Block,
    Track,
    SelectedTracks,
    Range,
}

struct AffectedCells {
    blocks: Vec<PatternId>,
    tracks: Option<Vec<usize>>,
    lines: Option<Range<usize>>,
}

impl AffectedCells {
    fn has(&self, line: usize, track: usize) -> bool {
        let line_in = self
            .lines
            .as_ref()
            .is_none_or(|lines| lines.contains(&line));
        let track_in = self
            .tracks
            .as_ref()
            .is_none_or(|tracks| tracks.contains(&track));
        return line_in && track_in;
    }
}

impl AppState {
    pub fn transpose_notes(&mut self, affect: Affect, current_only: bool, semitones: i16) {
        self.edit_affected_cells(affect, current_only, |cell| {
            if cell.note.value == 0 {
                return;
            }
            let moved = cell.note.value as i16 + semitones;
            if (1..=MAX_NOTE as i16).contains(&moved) {
                cell.note.value = moved as u8;
            }
        });
    }

    pub fn change_notes(
        &mut self,
        affect: Affect,
        current_only: bool,
        [from, to]: [OctamedicNote; 2],
        exchange: bool,
    ) {
        if from.value == 0 || to.value == 0 {
            self.notify("Pick both notes first");
            return;
        }
        self.edit_affected_cells(affect, current_only, |cell| {
            if cell.note == from {
                cell.note = to;
            } else if exchange && cell.note == to {
                cell.note = from;
            }
        });
    }

    pub fn change_instruments(
        &mut self,
        affect: Affect,
        [from, to]: [InstrumentId; 2],
        exchange: bool,
    ) {
        if from.0 == 0 || to.0 == 0 {
            self.notify("Pick both instruments first");
            return;
        }
        self.edit_affected_cells(affect, false, |cell| {
            if cell.instrument_id == from {
                cell.instrument_id = to;
            } else if exchange && cell.instrument_id == to {
                cell.instrument_id = from;
            }
        });
    }

    pub fn delete_instrument_notes(&mut self, affect: Affect, instrument: InstrumentId) {
        if instrument.0 == 0 {
            self.notify("Pick an instrument first");
            return;
        }
        self.edit_affected_cells(affect, false, |cell| {
            if cell.instrument_id == instrument {
                cell.note = OctamedicNote::new();
                cell.instrument_id = InstrumentId(0);
            }
        });
    }

    pub fn set_play_transpose(&mut self, semitones: i8) {
        if self.song().global_transpose != semitones {
            self.song_mut().global_transpose = semitones;
            self.engine.set_global_transpose(semitones);
        }
    }

    pub fn set_song_text(&mut self, text: &str) {
        if self.project().annotation != text {
            self.project_mut().annotation = text.to_owned();
        }
    }

    fn affected_cells(&mut self, affect: Affect) -> Option<AffectedCells> {
        let active = self.active_pattern;
        let block_only = |tracks, lines| AffectedCells {
            blocks: vec![active],
            tracks,
            lines,
        };
        return match affect {
            Affect::Song => Some(AffectedCells {
                blocks: (0..self.song().patterns().len())
                    .map(|index| PatternId(index as u8))
                    .collect(),
                tracks: None,
                lines: None,
            }),
            Affect::Block => Some(block_only(None, None)),
            Affect::Track => Some(block_only(Some(vec![self.cursor.track]), None)),
            Affect::SelectedTracks => {
                let selected = (0..SELECTABLE_TRACKS).filter(|&track| self.selected_tracks[track]);
                Some(block_only(Some(selected.collect()), None))
            }
            Affect::Range => {
                let Some(range) = self.active_range() else {
                    self.notify("No range");
                    return None;
                };
                Some(block_only(
                    Some(range.tracks().collect()),
                    Some(range.lines()),
                ))
            }
        };
    }

    fn edit_affected_cells(
        &mut self,
        affect: Affect,
        current_only: bool,
        mut edit: impl FnMut(&mut OctamedicPatternTrack),
    ) {
        let Some(affected) = self.affected_cells(affect) else {
            return;
        };
        let current = self.active_instrument;
        for &id in &affected.blocks {
            let Some(pattern) = self.song().get_pattern(&id) else {
                continue;
            };
            let mut edited = Vec::new();
            for (line_index, line) in pattern.lines.iter().enumerate() {
                for (track, &cell) in line.tracks.iter().enumerate() {
                    if !affected.has(line_index, track) {
                        continue;
                    }
                    if current_only && cell.instrument_id != current {
                        continue;
                    }
                    let mut changed = cell;
                    edit(&mut changed);
                    if changed != cell {
                        edited.push((line_index, track, changed));
                    }
                }
            }
            if edited.is_empty() {
                continue;
            }
            let mut pattern = pattern.clone();
            for (line, track, cell) in edited {
                pattern.lines[line].tracks[track] = cell;
            }
            self.replace_pattern(id, pattern);
        }
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;
    use crate::app::block_range::Cell;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    fn put(state: &mut AppState, line: usize, track: usize, note: u8, instrument: u8) {
        let cell = &mut state.active_pattern_mut().unwrap().lines[line].tracks[track];
        cell.note.value = note;
        cell.instrument_id = InstrumentId(instrument);
    }

    fn note(state: &AppState, line: usize, track: usize) -> u8 {
        return state.active_pattern().unwrap().lines[line].tracks[track]
            .note
            .value;
    }

    fn instrument(state: &AppState, line: usize, track: usize) -> u8 {
        return state.active_pattern().unwrap().lines[line].tracks[track]
            .instrument_id
            .0;
    }

    #[test]
    fn transposing_skips_empty_cells_and_notes_that_would_fall_off() {
        let mut state = state();
        put(&mut state, 0, 0, 13, 1);
        put(&mut state, 1, 0, 1, 1);
        state.transpose_notes(Affect::Block, false, -1);
        assert_eq!((note(&state, 0, 0), note(&state, 1, 0)), (12, 1));
        assert_eq!(note(&state, 2, 0), 0);
        state.transpose_notes(Affect::Block, false, 12);
        assert_eq!((note(&state, 0, 0), note(&state, 1, 0)), (24, 13));
    }

    #[test]
    fn affect_picks_the_cells() {
        let mut state = state();
        for track in 0..4 {
            put(&mut state, 0, track, 13, 1);
            put(&mut state, 5, track, 13, 1);
        }
        state.cursor.track = 1;
        state.transpose_notes(Affect::Track, false, 1);
        assert_eq!(note(&state, 0, 1), 14);
        assert_eq!(note(&state, 0, 0), 13);

        state.selected_tracks = [false; SELECTABLE_TRACKS];
        state.selected_tracks[2] = true;
        state.transpose_notes(Affect::SelectedTracks, false, 1);
        assert_eq!((note(&state, 0, 2), note(&state, 0, 3)), (14, 13));

        state.transpose_notes(Affect::Range, false, 1);
        assert_eq!(state.notification.as_deref(), Some("No range"));
        state.start_range(Cell { track: 3, line: 0 });
        state.extend_range(Cell { track: 3, line: 2 });
        state.transpose_notes(Affect::Range, false, 1);
        assert_eq!((note(&state, 0, 3), note(&state, 5, 3)), (14, 13));
    }

    #[test]
    fn song_reaches_every_block() {
        let mut state = state();
        put(&mut state, 0, 0, 13, 1);
        state.new_block();
        put(&mut state, 0, 0, 13, 1);
        state.transpose_notes(Affect::Song, false, 12);
        assert_eq!(note(&state, 0, 0), 25);
        state.active_pattern = PatternId(0);
        assert_eq!(note(&state, 0, 0), 25);
    }

    #[test]
    fn curr_only_touches_the_current_instrument() {
        let mut state = state();
        put(&mut state, 0, 0, 13, 1);
        put(&mut state, 0, 1, 13, 2);
        state.active_instrument = InstrumentId(2);
        state.transpose_notes(Affect::Block, true, 1);
        assert_eq!((note(&state, 0, 0), note(&state, 0, 1)), (13, 14));
    }

    #[test]
    fn changing_and_exchanging_notes() {
        let mut state = state();
        put(&mut state, 0, 0, 13, 1);
        put(&mut state, 1, 0, 20, 1);
        let [c, g] = [13, 20].map(|value| OctamedicNote { value });
        state.change_notes(Affect::Block, false, [c, g], true);
        assert_eq!((note(&state, 0, 0), note(&state, 1, 0)), (20, 13));
        state.change_notes(Affect::Block, false, [c, g], false);
        assert_eq!((note(&state, 0, 0), note(&state, 1, 0)), (20, 20));
        state.change_notes(Affect::Block, false, [OctamedicNote::new(), g], false);
        assert_eq!(state.notification.as_deref(), Some("Pick both notes first"));
    }

    #[test]
    fn changing_exchanging_and_deleting_instruments() {
        let mut state = state();
        put(&mut state, 0, 0, 13, 1);
        put(&mut state, 1, 0, 14, 2);
        let [one, two] = [1, 2].map(InstrumentId);
        state.change_instruments(Affect::Block, [one, two], true);
        assert_eq!((instrument(&state, 0, 0), instrument(&state, 1, 0)), (2, 1));
        state.delete_instrument_notes(Affect::Block, two);
        assert_eq!((note(&state, 0, 0), instrument(&state, 0, 0)), (0, 0));
        assert_eq!(note(&state, 1, 0), 14);
    }

    #[test]
    fn one_transpose_is_one_undo() {
        let mut state = state();
        put(&mut state, 0, 0, 13, 1);
        state.new_block();
        put(&mut state, 0, 0, 13, 1);
        state.update();
        state.transpose_notes(Affect::Song, false, 1);
        state.update();
        state.undo();
        state.active_pattern = PatternId(0);
        assert_eq!(note(&state, 0, 0), 13);
        state.active_pattern = PatternId(1);
        assert_eq!(note(&state, 0, 0), 13);
    }
}
