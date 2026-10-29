use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use octamedic_core::{
    data::{
        instrument::InstrumentId,
        note::OctamedicNote,
        pattern::{OctamedicPattern, OctamedicPatternTrack, PatternId},
        project::OctamedicProject,
        song::{OctamedicSong, SongId},
        tempo::OctamedicTempo,
    },
    engine::{engine::PlayMode, handle::EngineHandle},
};

const NOTIFICATION_DURATION: Duration = Duration::from_secs(2);

const MAX_INSTRUMENT: u8 = 63;

const MAX_OCTAVE: u8 = 9;

// note keys from C up to the C an octave above, like a piano keyboard
const BOTTOM_OCTAVE_KEYS: [char; 13] = [
    'z', 's', 'x', 'd', 'c', 'v', 'g', 'b', 'h', 'n', 'j', 'm', ',',
];

const TOP_OCTAVE_KEYS: [char; 13] = [
    'q', '2', 'w', '3', 'e', 'r', '5', 't', '6', 'y', '7', 'u', 'i',
];

pub struct AppState {
    project: OctamedicProject,
    pub engine: EngineHandle,

    pub active_song: SongId,
    pub active_pattern: PatternId,
    pub active_sequence_index: usize,
    pub active_instrument: InstrumentId,
    pub current_octave: u8,
    pub line_add: u8,
    pub recording: bool,
    pub exit_requested: bool,
    pub notification: Option<String>,
    notified_at: Instant,
    time: Duration,
    playback_start_time: Option<Instant>,
    has_changes: bool,
    changed_patterns: HashSet<PatternId>,
    sequence_changed: bool,
}

impl AppState {
    pub fn new(project: OctamedicProject, engine: EngineHandle) -> Self {
        let mut state = Self {
            project,
            engine,
            active_song: SongId(0),
            active_pattern: PatternId(0),
            active_sequence_index: 0,
            active_instrument: InstrumentId(1),
            current_octave: 2,
            line_add: 1,
            recording: false,
            exit_requested: false,
            notification: None,
            notified_at: Instant::now(),
            time: Duration::ZERO,
            playback_start_time: None,
            has_changes: false,
            changed_patterns: HashSet::new(),
            sequence_changed: false,
        };
        state.send_all_to_engine();
        return state;
    }

    pub fn update(&mut self) {
        if let Some(start) = self.playback_start_time
            && self.is_playing()
        {
            self.time = start.elapsed();
        }
        if self.notified_at.elapsed() >= NOTIFICATION_DURATION {
            self.notification = None;
        }
        self.send_changes_to_engine();
        if self.is_playing() {
            self.active_pattern = PatternId(self.engine.status().pattern() as u8);
        }
    }

    pub fn project(&self) -> &OctamedicProject {
        return &self.project;
    }

    pub fn project_mut(&mut self) -> &mut OctamedicProject {
        self.has_changes = true;
        return &mut self.project;
    }

    pub fn song(&self) -> &OctamedicSong {
        return self
            .project
            .get_song(self.active_song)
            .expect("the active song should exist");
    }

    fn song_mut(&mut self) -> &mut OctamedicSong {
        self.has_changes = true;
        let song = self.active_song;
        return self
            .project
            .get_song_mut(song)
            .expect("the active song should exist");
    }

    pub fn active_pattern(&self) -> Option<&OctamedicPattern> {
        return self.song().get_pattern(&self.active_pattern);
    }

    pub fn active_pattern_mut(&mut self) -> Option<&mut OctamedicPattern> {
        let id = self.active_pattern;
        self.changed_patterns.insert(id);
        return self.song_mut().get_pattern_mut(id);
    }

    pub fn is_playing(&self) -> bool {
        return self.engine.status().play_mode() != PlayMode::Paused;
    }

    pub fn time(&self) -> Duration {
        return self.time;
    }

    pub fn is_saved(&self) -> bool {
        return !self.has_changes;
    }

    pub fn notify(&mut self, notification: impl Into<String>) {
        self.notification = Some(notification.into());
        self.notified_at = Instant::now();
    }

    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    pub fn play_song(&mut self, sequence_index: usize) {
        self.send_all_to_engine();
        self.engine.play_song(sequence_index, !self.recording);
        self.playback_start_time = Some(Instant::now());
    }

    pub fn play_pattern(&mut self, id: PatternId) {
        self.send_all_to_engine();
        self.engine.play_pattern(id, !self.recording);
        self.playback_start_time = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        self.engine.stop();
    }

    pub fn next_instrument(&mut self) -> InstrumentId {
        self.active_instrument.0 = self
            .active_instrument
            .0
            .saturating_add(1)
            .min(MAX_INSTRUMENT);
        return self.active_instrument;
    }

    pub fn prev_instrument(&mut self) -> InstrumentId {
        self.active_instrument.0 = self.active_instrument.0.saturating_sub(1).max(1);
        return self.active_instrument;
    }

    pub fn increase_line_add(&mut self) {
        self.line_add = self.line_add.saturating_add(1);
    }

    pub fn decrease_line_add(&mut self) {
        self.line_add = self.line_add.saturating_sub(1);
    }

    pub fn next_pattern(&mut self) {
        self.active_pattern.0 = self.active_pattern.0.saturating_add(1);
        self.ensure_pattern(self.active_pattern);
    }

    pub fn prev_pattern(&mut self) {
        self.active_pattern.0 = self.active_pattern.0.saturating_sub(1);
        self.ensure_pattern(self.active_pattern);
    }

    fn ensure_pattern(&mut self, id: PatternId) {
        if self.song().get_pattern(&id).is_some() {
            return;
        }
        self.song_mut().ensure_pattern(id);
        self.changed_patterns.insert(id);
    }

    pub fn set_octave(&mut self, octave: u8) {
        self.current_octave = octave.clamp(1, MAX_OCTAVE);
    }

    pub fn set_tempo(&mut self, tempo: OctamedicTempo) {
        *self.song_mut().tempo_mut() = tempo;
        self.engine.set_tempo(tempo);
    }

    // the bottom row of letter keys plays the current octave and the top row the one above
    pub fn key_to_note(&self, key: char) -> Option<OctamedicNote> {
        let key = key.to_ascii_lowercase();
        let semitone = BOTTOM_OCTAVE_KEYS
            .iter()
            .position(|&k| k == key)
            .or_else(|| {
                TOP_OCTAVE_KEYS
                    .iter()
                    .position(|&k| k == key)
                    .map(|n| n + 12)
            })? as u8;
        // note 1 is C-1
        return Some(OctamedicNote {
            value: 1 + semitone + 12 * (self.current_octave - 1),
        });
    }

    pub fn play_note(&self, track: usize, note: OctamedicNote) {
        self.engine.play_note(track, note, self.active_instrument);
    }

    pub fn release_note(&self, track: usize) {
        self.engine.release_note(track);
    }

    // returns whether the cell changed
    pub fn edit_track(
        &mut self,
        line: usize,
        track: usize,
        edit: impl FnOnce(&mut OctamedicPatternTrack),
    ) -> bool {
        let Some(&previous) = self
            .active_pattern()
            .and_then(|pattern| pattern.track(line, track))
        else {
            return false;
        };
        let mut cell = previous;
        edit(&mut cell);
        if cell == previous {
            return false;
        }
        if let Some(slot) = self
            .active_pattern_mut()
            .and_then(|pattern| pattern.track_mut(line, track))
        {
            *slot = cell;
        }
        return true;
    }

    pub fn enter_note(&mut self, track: usize, line: usize, note: OctamedicNote) {
        let instrument = self.active_instrument;
        self.edit_track(line, track, |cell| {
            cell.note = note;
            cell.instrument_id = instrument;
        });
    }

    pub fn enter_instrument(&mut self, track: usize, line: usize, instrument: InstrumentId) {
        if instrument.0 > MAX_INSTRUMENT {
            return;
        }
        self.edit_track(line, track, |cell| cell.instrument_id = instrument);
    }

    fn send_all_to_engine(&mut self) {
        self.changed_patterns.clear();
        self.sequence_changed = false;
        let song = self
            .project
            .get_song(self.active_song)
            .expect("the active song should exist");
        self.engine.load_song(song);
    }

    fn send_changes_to_engine(&mut self) {
        let song = self
            .project
            .get_song(self.active_song)
            .expect("the active song should exist");
        for id in self.changed_patterns.drain() {
            if let Some(pattern) = song.get_pattern(&id) {
                self.engine.set_pattern(id, pattern.clone());
            }
        }
        if self.sequence_changed {
            self.sequence_changed = false;
            self.engine.set_sequence(
                song.sequence()
                    .iter()
                    .map(|&id| PatternId::from(id))
                    .collect(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    #[test]
    fn note_keys_follow_the_current_octave() {
        let mut state = state();
        state.set_octave(1);
        assert_eq!(state.key_to_note('z').unwrap().value, 1);
        assert_eq!(state.key_to_note('q').unwrap().value, 13);
        state.set_octave(3);
        assert_eq!(state.key_to_note('Z').unwrap().value, 25);
        assert_eq!(state.key_to_note('i').unwrap().value, 49);
        assert!(state.key_to_note('p').is_none());
    }

    #[test]
    fn instruments_stay_between_1_and_63() {
        let mut state = state();
        assert_eq!(state.prev_instrument(), InstrumentId(1));
        state.active_instrument = InstrumentId(63);
        assert_eq!(state.next_instrument(), InstrumentId(63));
    }

    #[test]
    fn entering_a_note_writes_it_with_the_active_instrument() {
        let mut state = state();
        state.active_instrument = InstrumentId(5);
        state.enter_note(1, 3, OctamedicNote { value: 13 });
        let cell = *state.active_pattern().unwrap().track(3, 1).unwrap();
        assert_eq!(cell.note.value, 13);
        assert_eq!(cell.instrument_id, InstrumentId(5));
        assert!(!state.is_saved());
    }

    #[test]
    fn an_edit_that_changes_nothing_leaves_the_project_saved() {
        let mut state = state();
        assert!(!state.edit_track(0, 0, |_| {}));
        assert!(state.is_saved());
    }

    #[test]
    fn moving_to_a_new_pattern_creates_it() {
        let mut state = state();
        state.next_pattern();
        assert_eq!(state.active_pattern, PatternId(1));
        assert!(state.active_pattern().is_some());
        state.prev_pattern();
        state.prev_pattern();
        assert_eq!(state.active_pattern, PatternId(0));
    }
}
