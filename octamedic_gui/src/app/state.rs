use std::{
    collections::{HashSet, VecDeque},
    time::{Duration, Instant},
};

use ggez::graphics::InstanceArray;
use octamedic_core::{
    data::{
        instrument::{InstrumentId, MAX_INSTRUMENTS},
        note::OctamedicNote,
        palette::OctamedColorPallete,
        pattern::{OctamedicPattern, OctamedicPatternTrack, PatternId},
        project::OctamedicProject,
        song::{OctamedicSong, SongId},
        tempo::OctamedicTempo,
    },
    engine::{engine::PlayMode, handle::EngineHandle},
};

use crate::app::{
    cursor::EditCursor,
    mode::{HeaderMode, MainMode},
};

const NOTIFICATION_DURATION: Duration = Duration::from_secs(2);

const MAX_INSTRUMENT: u8 = MAX_INSTRUMENTS as u8;

const MAX_OCTAVE: u8 = 9;

/// Older lines are dropped from the debug console past this many.
const MAX_DEBUG_LINES: usize = 256;

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
    pub cursor: EditCursor,
    header_mode: HeaderMode,
    main_mode: MainMode,
    /// The editor the main view goes back to when playing: the block or the notation editor.
    last_editor: MainMode,
    pub active_sequence_index: usize,
    pub active_instrument: InstrumentId,
    pub current_octave: u8,
    pub line_add: u8,
    pub recording: bool,
    pub exit_requested: bool,
    pub notification: Option<String>,
    pub pallete: OctamedColorPallete,
    debug_lines: VecDeque<String>,
    notified_at: Instant,
    timer_started: Instant,
    time: Duration,
    playback_start_time: Option<Instant>,
    has_changes: bool,
    changed_patterns: HashSet<PatternId>,
    sequence_changed: bool,
}

impl AppState {
    pub fn new(project: OctamedicProject, engine: EngineHandle) -> Self {
        let pallete = project.pallete();
        let mut state = Self {
            project,
            engine,
            active_song: SongId(0),
            active_pattern: PatternId(0),
            cursor: EditCursor::default(),
            header_mode: HeaderMode::default(),
            main_mode: MainMode::default(),
            last_editor: MainMode::default(),
            active_sequence_index: 0,
            active_instrument: InstrumentId(1),
            current_octave: 2,
            line_add: 1,
            recording: false,
            exit_requested: false,
            notification: None,
            pallete,
            debug_lines: VecDeque::new(),
            notified_at: Instant::now(),
            timer_started: Instant::now(),
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
        if let Some(start) = self.playback_start_time {
            if self.is_playing() {
                self.time = start.elapsed();
            }
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

    pub fn header_mode(&self) -> HeaderMode {
        return self.header_mode;
    }

    pub fn main_mode(&self) -> MainMode {
        return self.main_mode;
    }

    pub fn last_editor(&self) -> MainMode {
        return self.last_editor;
    }

    pub fn set_modes(&mut self, header: HeaderMode, main: MainMode) {
        self.header_mode = header;
        self.main_mode = main;
        if main.is_editor() {
            self.last_editor = main;
        }
    }

    pub fn is_playing(&self) -> bool {
        return self.engine.status().play_mode() != PlayMode::Paused;
    }

    pub fn time(&self) -> Duration {
        return self.time;
    }

    /// The title bar's timer.
    pub fn timer(&self) -> Duration {
        return self.timer_started.elapsed();
    }

    pub fn restart_timer(&mut self) {
        self.timer_started = Instant::now();
    }

    pub fn is_saved(&self) -> bool {
        return !self.has_changes;
    }

    pub fn notify(&mut self, notification: impl Into<String>) {
        self.notification = Some(notification.into());
        self.notified_at = Instant::now();
    }

    /// Adds a line to the debug console.
    pub fn debug(&mut self, line: impl Into<String>) {
        if self.debug_lines.len() == MAX_DEBUG_LINES {
            self.debug_lines.pop_front();
        }
        self.debug_lines.push_back(line.into());
    }

    /// Oldest first.
    pub fn debug_lines(&self) -> &VecDeque<String> {
        return &self.debug_lines;
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
    pub fn set_instrument(&mut self, instrument: u8) -> InstrumentId {
        self.active_instrument.0 = instrument.min(MAX_INSTRUMENT);
        return self.active_instrument;
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

    /// Shows block `index`, or the last block past the end, keeping the cursor inside it.
    pub fn select_block(&mut self, index: usize) {
        let last = self.song().patterns().len().saturating_sub(1);
        self.active_pattern = PatternId(index.min(last).min(u8::MAX as usize) as u8);
        if let Some(pattern) = self.active_pattern() {
            let (lines, tracks) = (pattern.line_count(), pattern.track_count());
            self.cursor.clamp(lines, tracks);
        }
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
            if let Some(pattern) = song.get_shared_pattern(&id) {
                self.engine.set_pattern(id, pattern);
            }
        }
        if self.sequence_changed {
            self.sequence_changed = false;
            self.engine.set_sequence(song.sequence().to_vec());
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
    fn the_debug_console_keeps_the_newest_lines() {
        let mut state = state();
        for line in 0..MAX_DEBUG_LINES + 2 {
            state.debug(line.to_string());
        }
        assert_eq!(state.debug_lines().len(), MAX_DEBUG_LINES);
        assert_eq!(state.debug_lines().front().unwrap(), "2");
        assert_eq!(
            state.debug_lines().back().unwrap(),
            &(MAX_DEBUG_LINES + 1).to_string()
        );
    }

    #[test]
    fn the_last_editor_is_remembered_through_other_views() {
        let mut state = state();
        assert_eq!(state.last_editor(), MainMode::BlockEditor);
        state.set_modes(HeaderMode::Play, MainMode::NotationEditor);
        state.set_modes(HeaderMode::Files, MainMode::Files);
        assert_eq!(state.last_editor(), MainMode::NotationEditor);
    }

    #[test]
    fn instruments_stay_between_1_and_63() {
        let mut state = state();
        assert_eq!(state.prev_instrument(), InstrumentId(1));
        state.active_instrument = InstrumentId(MAX_INSTRUMENT);
        assert_eq!(state.next_instrument(), InstrumentId(MAX_INSTRUMENT));
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
