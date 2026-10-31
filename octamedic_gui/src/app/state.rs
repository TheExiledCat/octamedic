use std::{
    collections::{HashSet, VecDeque},
    time::{Duration, Instant},
};

use octamed::utility::note::SEMITONES_PER_OCTAVE;
use octamedic_core::{
    data::{
        instrument::{InstrumentId, MAX_INSTRUMENTS},
        note::OctamedicNote,
        palette::OctamedColorPalette,
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

const SEMITONES: u8 = SEMITONES_PER_OCTAVE;

// OctaMED's sequence limit
const MAX_SEQUENCE_ENTRIES: usize = 256;

const MAX_DEBUG_LINES: usize = 256;

// C up to the next C, laid out like a piano keyboard
const BOTTOM_OCTAVE_KEYS: [char; 13] = [
    'z', 's', 'x', 'd', 'c', 'v', 'g', 'b', 'h', 'n', 'j', 'm', ',',
];

const TOP_OCTAVE_KEYS: [char; 13] = [
    'q', '2', 'w', '3', 'e', 'r', '5', 't', '6', 'y', '7', 'u', 'i',
];

pub struct AppState {
    pub engine: EngineHandle,
    pub active_song: SongId,
    pub active_pattern: PatternId,
    pub cursor: EditCursor,
    pub active_sequence_index: usize,
    pub active_instrument: InstrumentId,
    pub current_octave: u8,
    pub line_add: u8,
    pub recording: bool,
    pub exit_requested: bool,
    pub notification: Option<String>,
    pub palette: OctamedColorPalette,
    project: OctamedicProject,
    header_mode: HeaderMode,
    main_mode: MainMode,
    // where playing goes back to: the block or the notation editor
    last_editor: MainMode,
    debug_lines: VecDeque<String>,
    notified_at: Instant,
    // time played before `timer_resumed`, which is set while playing
    timer: Duration,
    timer_resumed: Option<Instant>,
    time: Duration,
    playback_start_time: Option<Instant>,
    has_changes: bool,
    changed_patterns: HashSet<PatternId>,
    sequence_changed: bool,
}

impl AppState {
    pub fn new(project: OctamedicProject, engine: EngineHandle) -> Self {
        let palette = project.palette();
        let mut state = Self {
            engine,
            active_song: SongId(0),
            active_pattern: PatternId(0),
            cursor: EditCursor::default(),
            active_sequence_index: 0,
            active_instrument: InstrumentId(1),
            current_octave: 2,
            line_add: 1,
            recording: false,
            exit_requested: false,
            notification: None,
            palette,
            project,
            header_mode: HeaderMode::default(),
            main_mode: MainMode::default(),
            last_editor: MainMode::default(),
            debug_lines: VecDeque::new(),
            notified_at: Instant::now(),
            timer: Duration::ZERO,
            timer_resumed: None,
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
        self.run_timer(self.is_playing());
        if self.notified_at.elapsed() >= NOTIFICATION_DURATION {
            self.notification = None;
        }
        self.send_changes_to_engine();
        if self.is_playing() {
            let status = self.engine.status();
            let (mode, index, block) = (
                status.play_mode(),
                status.sequence_index(),
                status.pattern(),
            );
            // a block plays outside the sequence
            if matches!(mode, PlayMode::Song | PlayMode::SongLoop) {
                self.keep_sequence_index_inside(index);
            }
            self.select_block(block);
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

    pub fn song_mut(&mut self) -> &mut OctamedicSong {
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

    pub fn timer(&self) -> Duration {
        return self.timer + self.timer_resumed.map_or(Duration::ZERO, |at| at.elapsed());
    }

    pub fn restart_timer(&mut self) {
        self.timer = Duration::ZERO;
        self.timer_resumed = self.timer_resumed.map(|_| Instant::now());
    }

    pub fn is_saved(&self) -> bool {
        return !self.has_changes;
    }

    pub fn notify(&mut self, notification: impl Into<String>) {
        self.notification = Some(notification.into());
        self.notified_at = Instant::now();
    }

    pub fn debug(&mut self, line: impl Into<String>) {
        if self.debug_lines.len() == MAX_DEBUG_LINES {
            self.debug_lines.pop_front();
        }
        self.debug_lines.push_back(line.into());
    }

    pub fn debug_lines(&self) -> &VecDeque<String> {
        return &self.debug_lines;
    }

    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    pub fn play_song(&mut self, sequence_index: usize, row: usize) {
        self.send_all_to_engine();
        self.engine.play_song(sequence_index, row, !self.recording);
        self.playback_start_time = Some(Instant::now());
    }

    pub fn play_pattern(&mut self, id: PatternId, row: usize) {
        self.send_all_to_engine();
        self.engine.play_pattern(id, row, !self.recording);
        self.playback_start_time = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        self.engine.stop();
    }

    pub fn set_instrument(&mut self, instrument: u8) -> InstrumentId {
        self.active_instrument.0 = instrument.clamp(1, MAX_INSTRUMENT);
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

    pub fn next_sequence(&mut self) -> PatternId {
        return self.set_sequence(self.active_sequence_index.saturating_add(1));
    }

    pub fn prev_sequence(&mut self) -> PatternId {
        return self.set_sequence(self.active_sequence_index.saturating_sub(1));
    }

    // returns block 0 when the sequence is empty
    pub fn set_sequence(&mut self, index: usize) -> PatternId {
        self.keep_sequence_index_inside(index);
        let block = self
            .song()
            .sequence()
            .get(self.active_sequence_index)
            .copied()
            .unwrap_or(PatternId::ZERO);
        self.select_block(block.0 as usize);
        return block;
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

    pub fn select_block(&mut self, index: usize) {
        let last = self.song().patterns().len().saturating_sub(1);
        self.active_pattern = PatternId(index.min(last).min(u8::MAX as usize) as u8);
        if let Some(pattern) = self.active_pattern() {
            let (lines, tracks) = (pattern.line_count(), pattern.track_count());
            self.cursor.clamp(lines, tracks);
        }
    }

    pub fn set_octave(&mut self, octave: u8) {
        self.current_octave = octave.clamp(1, MAX_OCTAVE);
    }

    pub fn set_tempo(&mut self, tempo: OctamedicTempo) {
        *self.song_mut().tempo_mut() = tempo;
        self.engine.set_tempo(tempo);
    }

    pub fn key_to_note(&self, key: char) -> Option<OctamedicNote> {
        let key = key.to_ascii_lowercase();
        let semitone = BOTTOM_OCTAVE_KEYS
            .iter()
            .position(|&k| k == key)
            .or_else(|| {
                TOP_OCTAVE_KEYS
                    .iter()
                    .position(|&k| k == key)
                    .map(|n| n + SEMITONES as usize)
            })? as u8;
        // note 1 is C-1
        return Some(OctamedicNote {
            value: 1 + semitone + SEMITONES * (self.current_octave - 1),
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

    pub fn play_key(&self, track: usize, key: char) -> Option<OctamedicNote> {
        let note = self.key_to_note(key)?;
        self.play_note(track, note);
        return Some(note);
    }

    pub fn enter_key(&mut self, track: usize, line: usize, key: char) -> Option<OctamedicNote> {
        let note = self.play_key(track, key)?;
        self.enter_note(track, line, note);
        return Some(note);
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

    pub fn increment_sequence_pattern(&mut self) {
        let active = self.active_sequence_index;
        let last = self.song().patterns().len().saturating_sub(1);
        let last = last.min(u8::MAX as usize) as u8;
        self.edit_sequence(|sequence| {
            if let Some(entry) = sequence.get_mut(active) {
                entry.0 = entry.0.saturating_add(1).min(last);
            }
        });
    }

    pub fn decrement_sequence_pattern(&mut self) {
        let active = self.active_sequence_index;
        self.edit_sequence(|sequence| {
            if let Some(entry) = sequence.get_mut(active) {
                entry.0 = entry.0.saturating_sub(1);
            }
        });
    }

    // after the active entry, so there's a way to add to the end
    pub fn insert_sequence(&mut self, pattern: PatternId) {
        let index = (self.active_sequence_index + 1).min(self.song().sequence().len());
        if self.song().sequence().len() >= MAX_SEQUENCE_ENTRIES {
            return;
        }
        self.active_sequence_index = index;
        self.edit_sequence(|sequence| sequence.insert(index, pattern));
    }

    // the last entry stays: OctaMED's sequence always plays something
    pub fn delete_sequence(&mut self, index: usize) {
        self.edit_sequence(|sequence| {
            if sequence.len() > 1 && index < sequence.len() {
                sequence.remove(index);
            }
        });
    }

    fn run_timer(&mut self, playing: bool) {
        match (playing, self.timer_resumed) {
            (true, None) => self.timer_resumed = Some(Instant::now()),
            (false, Some(at)) => {
                self.timer += at.elapsed();
                self.timer_resumed = None;
            }
            _ => {}
        }
    }

    fn keep_sequence_index_inside(&mut self, index: usize) {
        let last = self.song().sequence().len().saturating_sub(1);
        self.active_sequence_index = index.min(last);
    }

    fn ensure_pattern(&mut self, id: PatternId) {
        if self.song().get_pattern(&id).is_some() {
            return;
        }
        self.song_mut().ensure_pattern(id);
        self.changed_patterns.insert(id);
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

    fn edit_sequence(&mut self, edit: impl FnOnce(&mut Vec<PatternId>)) {
        edit(self.song_mut().sequence_mut());
        self.set_sequence(self.active_sequence_index);
        self.sequence_changed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    #[test]
    fn sequence_clamps() {
        let mut state = state();
        let last = state.song().sequence().len().saturating_sub(1);
        state.prev_sequence();
        assert_eq!(state.active_sequence_index, 0);
        state.set_sequence(last + 10);
        assert_eq!(state.active_sequence_index, last);
        state.next_sequence();
        assert_eq!(state.active_sequence_index, last);
    }

    #[test]
    fn sequence_edits() {
        let mut state = state();
        state.insert_sequence(PatternId(0));
        assert_eq!(state.song().sequence(), [PatternId(0); 2]);
        assert_eq!(state.active_sequence_index, 1);
        assert!(state.sequence_changed);
        // only block 0 exists
        state.increment_sequence_pattern();
        assert_eq!(state.song().sequence()[1], PatternId(0));
        state.delete_sequence(1);
        assert_eq!(state.active_sequence_index, 0);
        state.delete_sequence(0);
        assert_eq!(state.song().sequence().len(), 1);
    }

    #[test]
    fn sequence_edit_selects_block() {
        let mut state = state();
        state.next_pattern();
        state.next_pattern();
        state.insert_sequence(PatternId(2));
        assert_eq!(state.active_pattern, PatternId(2));
        state.decrement_sequence_pattern();
        assert_eq!(state.active_pattern, PatternId(1));
        state.delete_sequence(1);
        assert_eq!(state.active_sequence_index, 0);
        assert_eq!(state.active_pattern, PatternId(0));
    }

    #[test]
    fn sequence_jump_selects_block() {
        let mut state = state();
        state.next_pattern();
        state.insert_sequence(PatternId(1));
        state.set_sequence(0);
        assert_eq!(state.active_pattern, PatternId(0));
        state.next_sequence();
        assert_eq!(state.active_pattern, PatternId(1));
        state.prev_sequence();
        assert_eq!(state.active_pattern, PatternId(0));
    }

    #[test]
    fn note_keys_octave() {
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
    fn debug_keeps_newest() {
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
    fn remembers_last_editor() {
        let mut state = state();
        assert_eq!(state.last_editor(), MainMode::BlockEditor);
        state.set_modes(HeaderMode::Play, MainMode::NotationEditor);
        state.set_modes(HeaderMode::Files, MainMode::Files);
        assert_eq!(state.last_editor(), MainMode::NotationEditor);
    }

    #[test]
    fn instrument_range() {
        let mut state = state();
        assert_eq!(state.prev_instrument(), InstrumentId(1));
        state.active_instrument = InstrumentId(MAX_INSTRUMENT);
        assert_eq!(state.next_instrument(), InstrumentId(MAX_INSTRUMENT));
        assert_eq!(state.set_instrument(0), InstrumentId(1));
    }

    #[test]
    fn enter_note() {
        let mut state = state();
        state.active_instrument = InstrumentId(5);
        state.enter_note(1, 3, OctamedicNote { value: 13 });
        let cell = *state.active_pattern().unwrap().track(3, 1).unwrap();
        assert_eq!(cell.note.value, 13);
        assert_eq!(cell.instrument_id, InstrumentId(5));
        assert!(!state.is_saved());
    }

    #[test]
    fn enter_key_octave() {
        let mut state = state();
        state.set_octave(3);
        assert_eq!(state.enter_key(2, 4, 'x').unwrap().value, 27);
        let cell = *state.active_pattern().unwrap().track(4, 2).unwrap();
        assert_eq!(cell.note.value, 27);
        assert!(state.enter_key(2, 5, 'p').is_none());
        let cell = *state.active_pattern().unwrap().track(5, 2).unwrap();
        assert_eq!(cell.note.value, 0);
    }

    #[test]
    fn timer_runs_when_playing() {
        let mut state = state();
        state.update();
        assert_eq!(state.timer(), Duration::ZERO);
        state.run_timer(true);
        let played = state.timer();
        state.run_timer(false);
        let held = state.timer();
        assert!(held >= played);
        state.run_timer(false);
        assert_eq!(state.timer(), held);
        state.restart_timer();
        assert_eq!(state.timer(), Duration::ZERO);
    }

    #[test]
    fn noop_edit_stays_saved() {
        let mut state = state();
        assert!(!state.edit_track(0, 0, |_| {}));
        assert!(state.is_saved());
    }

    #[test]
    fn new_pattern_created() {
        let mut state = state();
        state.next_pattern();
        assert_eq!(state.active_pattern, PatternId(1));
        assert!(state.active_pattern().is_some());
        state.prev_pattern();
        state.prev_pattern();
        assert_eq!(state.active_pattern, PatternId(0));
    }
}
