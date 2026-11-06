use std::{
    collections::{HashSet, VecDeque},
    time::{Duration, Instant},
};

use ggez::input::keyboard::KeyMods;
use mmd::{mmd::module::SONG_TRACKS, utility::note::SEMITONES_PER_OCTAVE};
use octamedic_core::{
    amiga::clock::VideoStandard,
    data::{
        instrument::{InstrumentId, InstrumentKind, MAX_INSTRUMENTS, OctamedicInstrument},
        note::OctamedicNote,
        palette::{OctamedColorPalette, PALETTE_SIZE},
        pattern::{MAX_PATTERNS, OctamedicPattern, OctamedicPatternTrack, PatternId},
        project::OctamedicProject,
        song::{OctamedicSong, SongId},
        tempo::OctamedicTempo,
        volume::MAX_VOLUME,
    },
    engine::{engine::PlayMode, handle::EngineHandle},
};
use rustuition::palette::{Color, IntuitionColorPalette};

use crate::{
    app::{
        DEFAULT_VIDEO,
        cursor::EditCursor,
        files::browser::Browser,
        mode::{HeaderMode, MainMode},
        prompt::Prompt,
        screen_filters::ScreenFilters,
    },
    framework::widgets::Dialog,
};

const NOTIFICATION_DURATION: Duration = Duration::from_secs(2);

const MAX_INSTRUMENT: u8 = MAX_INSTRUMENTS as u8;

// the keyboard's bottom row. F1 to F5 only go up to 5, the status bar's octaves box to 9
pub const MAX_OCTAVE: u8 = 9;

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

/// How the instrument name box gets selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameEdit {
    Select,
    Clear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayStart {
    Song,
    ContinueSong,
    Block,
    ContinueBlock,
}

/// The format the Files panel saves instruments in, picked in the sample editor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InstrumentSaveType {
    #[default]
    Raw,
    Iff,
    Wav,
}

impl InstrumentSaveType {
    pub const ALL: [Self; 3] = [Self::Raw, Self::Iff, Self::Wav];

    pub fn next(self) -> Self {
        return Self::ALL[(self as usize + 1) % Self::ALL.len()];
    }

    pub fn previous(self) -> Self {
        return Self::ALL[(self as usize + Self::ALL.len() - 1) % Self::ALL.len()];
    }
}

/// The MOD variant the Files panel saves the song as, cycled through with its one button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModFormat {
    #[default]
    Mod1WithInstruments,
    Mod1NoInstruments,
    Mod0,
}

impl ModFormat {
    pub const ALL: [Self; 3] = [
        Self::Mod1WithInstruments,
        Self::Mod1NoInstruments,
        Self::Mod0,
    ];

    pub fn next(self) -> Self {
        return Self::ALL[(self as usize + 1) % Self::ALL.len()];
    }

    pub fn previous(self) -> Self {
        return Self::ALL[(self as usize + Self::ALL.len() - 1) % Self::ALL.len()];
    }
}

/// Which of the Files panel's save-format buttons is picked; SAVE SONG acts on this one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SaveFormat {
    #[default]
    Mod,
    Wav,
    Mp3,
    StModule,
}

// the E1, E2 and E3 switches
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Meters {
    pub bars: bool,
    pub vu: bool,
    pub scopes: bool,
}

impl Default for Meters {
    fn default() -> Self {
        return Self {
            bars: true,
            vu: true,
            scopes: true,
        };
    }
}

pub struct AppState {
    pub engine: EngineHandle,
    pub active_song: SongId,
    pub active_pattern: PatternId,
    pub cursor: EditCursor,
    pub active_sequence_index: usize,
    pub active_instrument: InstrumentId,
    pub current_octave: u8,
    // how far note entry and the arrows move while SP is on, never 0
    pub line_add: u8,
    // SP: move by the line add instead of a line at a time
    pub spacing: bool,
    // the keypad edits the play sequence without Ctrl while it's on
    pub caps_lock: bool,
    // Amiga+I asked for the instrument name box
    pub name_edit_request: Option<NameEdit>,
    pub recording: bool,
    pub exit_requested: bool,
    pub notification: Option<String>,
    // TRK: and BLK:'s buffers. A copied track is kept as a block one track wide. Both survive
    // loading another song, so you can copy between songs.
    // TODO: check whether OctaMED's range copy buffer is one of these
    pub track_buffer: Option<OctamedicPattern>,
    pub block_buffer: Option<OctamedicPattern>,
    // shown instead of the notification, and holding everything up until it's answered
    pub prompt: Option<Prompt>,
    // the sample editor's clipboard, CBUFF
    pub copy_buffer: Vec<i8>,
    pub instrument_save_type: InstrumentSaveType,
    pub mod_format: ModFormat,
    pub save_format: SaveFormat,
    pub palette: IntuitionColorPalette,
    // the D switch
    pub play_on_note: bool,
    pub meters: Meters,
    pub browser: Browser,
    pub screen_filters: ScreenFilters,
    // opened since the dialog stack last looked
    pending_dialogs: Vec<Dialog>,
    // right click: the title bar shows and the meters and scrolling stop
    pub displays_off: bool,
    // the modifier keys down at the last mouse event, Amiga included
    pub mouse_mods: KeyMods,
    video: VideoStandard,
    queued_sequence_index: Option<usize>,
    started: Option<PlayStart>,
    muted_tracks: [bool; SONG_TRACKS],
    // the active instrument the engine was last told about
    engine_active_instrument: Option<InstrumentId>,
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

// the song's colours, as the screen shows them
pub fn screen_palette(palette: &OctamedColorPalette) -> IntuitionColorPalette {
    let colors: Vec<Color> = (0..PALETTE_SIZE)
        .map(|pen| Color(palette.get(pen)))
        .collect();
    return IntuitionColorPalette::new(colors);
}

impl AppState {
    pub fn new(project: OctamedicProject, engine: EngineHandle) -> Self {
        let palette = screen_palette(&project.palette());
        let mut state = Self {
            engine,
            active_song: SongId(0),
            active_pattern: PatternId(0),
            cursor: EditCursor::default(),
            active_sequence_index: 0,
            active_instrument: InstrumentId(1),
            current_octave: 1,
            line_add: 1,
            spacing: false,
            caps_lock: false,
            name_edit_request: None,
            recording: false,
            exit_requested: false,
            notification: None,
            track_buffer: None,
            block_buffer: None,
            prompt: None,
            copy_buffer: Vec::new(),
            instrument_save_type: InstrumentSaveType::default(),
            mod_format: ModFormat::default(),
            save_format: SaveFormat::default(),
            palette,
            play_on_note: false,
            meters: Meters::default(),
            browser: Browser::new(),
            screen_filters: ScreenFilters::default(),
            pending_dialogs: Vec::new(),
            displays_off: false,
            mouse_mods: KeyMods::NONE,
            video: DEFAULT_VIDEO,
            queued_sequence_index: None,
            started: None,
            muted_tracks: [false; SONG_TRACKS],
            engine_active_instrument: None,
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
                self.follow_sequence(index);
            }
            self.select_block(block);
        }
    }

    pub fn open_dialog(&mut self, dialog: Dialog) {
        self.pending_dialogs.push(dialog);
    }

    pub fn take_pending_dialogs(&mut self) -> Vec<Dialog> {
        return std::mem::take(&mut self.pending_dialogs);
    }

    pub fn project(&self) -> &OctamedicProject {
        return &self.project;
    }

    pub fn project_mut(&mut self) -> &mut OctamedicProject {
        self.has_changes = true;
        return &mut self.project;
    }

    // keeps what isn't the song's, like the file browser and which screen is showing
    pub fn load_project(&mut self, project: OctamedicProject) {
        self.stop();
        for track in 0..SONG_TRACKS {
            if self.muted_tracks[track] {
                self.engine.set_track_muted(track, false);
            }
        }
        self.palette = screen_palette(&project.palette());
        self.project = project;
        self.active_song = SongId(0);
        self.active_pattern = PatternId(0);
        self.cursor = EditCursor::default();
        self.active_sequence_index = 0;
        self.active_instrument = InstrumentId(1);
        self.muted_tracks = [false; SONG_TRACKS];
        self.has_changes = false;
        self.restart_timer();
        self.send_all_to_engine();
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

    pub fn toggle_highlight(&mut self, line: usize) {
        if let Some(line) = self
            .active_pattern_mut()
            .and_then(|pattern| pattern.lines.get_mut(line))
        {
            line.highlighted = !line.highlighted;
        }
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

    pub fn is_track_muted(&self, track: usize) -> bool {
        return self.muted_tracks.get(track).copied().unwrap_or(false);
    }

    pub fn toggle_track_mute(&mut self, track: usize) {
        let Some(muted) = self.muted_tracks.get_mut(track) else {
            return;
        };
        *muted = !*muted;
        self.engine.set_track_muted(track, *muted);
    }

    pub fn restart_timer(&mut self) {
        self.timer = Duration::ZERO;
        self.timer_resumed = self.timer_resumed.map(|_| Instant::now());
    }

    pub fn is_saved(&self) -> bool {
        return !self.has_changes;
    }

    pub fn open_prompt(&mut self, prompt: Prompt) {
        self.prompt = Some(prompt);
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

    pub fn video(&self) -> VideoStandard {
        return self.video;
    }

    pub fn set_video(&mut self, video: VideoStandard) {
        self.video = video;
        self.engine.set_machine(video.clock());
    }

    pub fn active_meters(&self) -> Meters {
        if self.displays_off {
            return Meters {
                bars: false,
                vu: false,
                scopes: false,
            };
        }
        return self.meters;
    }

    pub fn start(&mut self, start: PlayStart) {
        self.queued_sequence_index = None;
        let line = self.cursor.line;
        match start {
            PlayStart::Song => self.play_song(0, 0),
            PlayStart::ContinueSong => self.play_song(self.active_sequence_index, line),
            PlayStart::Block => self.play_pattern(self.active_pattern, 0),
            PlayStart::ContinueBlock => self.play_pattern(self.active_pattern, line),
        }
        self.started = Some(start);
    }

    pub fn started(&self) -> Option<PlayStart> {
        return self.started;
    }

    pub fn stop(&mut self) {
        self.engine.stop();
        self.started = None;
        self.queued_sequence_index = None;
    }

    // plays at `note`, without it going in the song
    pub fn preview(&self, instrument: &OctamedicInstrument, note: u8) {
        if let Some(sample) = instrument.sample() {
            self.engine
                .preview(sample.clone(), OctamedicNote { value: note });
        }
    }

    pub fn stop_preview(&self) {
        self.engine.stop_preview();
    }

    pub fn replace_instrument(&mut self, id: InstrumentId, instrument: OctamedicInstrument) {
        let Some(index) = id.index() else {
            return;
        };
        let Some(slot) = self.song_mut().instruments.get_mut(index) else {
            return;
        };
        *slot = Some(instrument.clone());
        self.engine.set_instrument(id, Some(instrument));
    }

    /// Changes the active instrument, putting an empty one in its slot first if there isn't one.
    pub fn edit_active_instrument(&mut self, edit: impl FnOnce(&mut OctamedicInstrument)) {
        let id = self.active_instrument;
        let mut instrument = self
            .song()
            .instrument(id)
            .cloned()
            .unwrap_or_else(|| OctamedicInstrument::new(""));
        edit(&mut instrument);
        self.replace_instrument(id, instrument);
    }

    pub fn set_instrument(&mut self, instrument: u8) -> InstrumentId {
        self.active_instrument.0 = instrument.clamp(1, MAX_INSTRUMENT);
        return self.active_instrument;
    }

    /// Makes the last instrument with a sound active, or the first when none has one.
    pub fn last_filled_instrument(&mut self) -> InstrumentId {
        let last = self.song().instruments.iter().rposition(|slot| {
            slot.as_ref()
                .is_some_and(|instrument| !matches!(instrument.kind, InstrumentKind::Empty))
        });
        return self.set_instrument(last.map_or(1, |index| index as u8 + 1));
    }

    pub fn next_instrument(&mut self) -> InstrumentId {
        self.active_instrument.0 = self
            .active_instrument
            .0
            .saturating_add(1)
            .min(MAX_INSTRUMENT);
        return self.active_instrument;
    }

    /// Moves the active instrument by `delta`, stopping at the first and last.
    pub fn move_instrument(&mut self, delta: i32) -> InstrumentId {
        let instrument = (self.active_instrument.0 as i32 + delta).clamp(1, MAX_INSTRUMENT as i32);
        return self.set_instrument(instrument as u8);
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

    // returns block 0 when the sequence is empty. while a song plays, the move waits for the
    // block playing to end
    pub fn set_sequence(&mut self, index: usize) -> PatternId {
        self.keep_sequence_index_inside(index);
        let block = self
            .song()
            .sequence()
            .get(self.active_sequence_index)
            .copied()
            .unwrap_or(PatternId::ZERO);
        if self.is_playing_song() {
            let playing = self.engine.status().sequence_index();
            let queued =
                (self.active_sequence_index != playing).then_some(self.active_sequence_index);
            self.queued_sequence_index = queued;
            self.engine.queue_sequence_index(queued);
        } else {
            self.select_block(block.0 as usize);
        }
        return block;
    }

    fn is_playing_song(&self) -> bool {
        return matches!(
            self.engine.status().play_mode(),
            PlayMode::Song | PlayMode::SongLoop
        );
    }

    fn follow_sequence(&mut self, playing: usize) {
        match self.queued_sequence_index {
            Some(queued) if queued != playing => self.keep_sequence_index_inside(queued),
            _ => {
                self.queued_sequence_index = None;
                self.keep_sequence_index_inside(playing);
            }
        }
    }

    /// Lines the cursor moves after entering something, and with the arrows.
    pub fn line_step(&self) -> usize {
        if self.spacing {
            return self.line_add.max(1) as usize;
        }
        return 1;
    }

    pub fn increase_line_add(&mut self) {
        self.line_add = self.line_add.saturating_add(1);
    }

    pub fn decrease_line_add(&mut self) {
        self.line_add = self.line_add.saturating_sub(1).max(1);
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
        self.clamp_cursor_to_active_pattern();
    }

    pub fn clamp_cursor_to_active_pattern(&mut self) {
        let (lines, tracks) = self.active_pattern().map_or((1, 1), |pattern| {
            (pattern.line_count(), pattern.track_count())
        });
        self.cursor.clamp(lines, tracks);
    }

    pub fn set_octave(&mut self, octave: u8) {
        self.current_octave = octave.clamp(1, MAX_OCTAVE);
    }

    pub fn set_tempo(&mut self, tempo: OctamedicTempo) {
        *self.song_mut().tempo_mut() = tempo;
        self.engine.set_tempo(tempo);
    }

    pub fn set_block_name(&mut self, name: &str) {
        if let Some(pattern) = self.active_pattern_mut() {
            pattern.name = name.to_owned();
        }
    }

    pub fn set_song_name(&mut self, name: &str) {
        self.song_mut().set_name(name);
    }

    pub fn set_track_volume(&mut self, track: usize, volume: u8) {
        let volume = volume.min(MAX_VOLUME);
        let current = self
            .song()
            .track_volumes
            .get(track)
            .map(|volume| volume.get());
        if current.is_none_or(|current| current == volume) {
            return;
        }
        self.song_mut().track_volumes[track].set(volume);
        self.engine.set_track_volume(track, volume);
    }

    pub fn set_master_volume(&mut self, volume: u8) {
        let volume = volume.min(MAX_VOLUME);
        if self.song().master_volume.get() == volume {
            return;
        }
        self.song_mut().master_volume.set(volume);
        self.engine.set_master_volume(volume);
    }

    pub fn set_volumes_hex(&mut self, hex: bool) {
        if self.song().flags.volumes_hex != hex {
            self.song_mut().flags.volumes_hex = hex;
            self.engine.set_volumes_hex(hex);
        }
    }

    /// Like `set_volumes_hex`, rewriting every volume command in the song so it still plays the
    /// same. What Shift-click on HEX or DEC does.
    pub fn convert_volumes_hex(&mut self, hex: bool) {
        if self.song().flags.volumes_hex == hex {
            return;
        }
        let changed = self.song_mut().convert_volume_commands(hex);
        self.changed_patterns.extend(changed);
        self.set_volumes_hex(hex);
    }

    pub fn set_filter(&mut self, on: bool) {
        if self.song().flags.filter_on != on {
            self.song_mut().flags.filter_on = on;
        }
        self.engine.set_filter(on);
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

    /// Takes the cell at `line` out of `track`, the rest of the track closing the gap from below.
    pub fn pull_up(&mut self, line: usize, track: usize) {
        if let Some(pattern) = self.active_pattern_mut() {
            pattern.pull_track_up(line, track);
        }
    }

    /// Opens an empty cell at `line` in `track`, pushing the rest of the track down and off the end.
    pub fn push_down(&mut self, line: usize, track: usize) {
        if let Some(pattern) = self.active_pattern_mut() {
            pattern.push_track_down(line, track);
        }
    }

    /// INS.TRK: an empty track at the cursor's, the rightmost one dropping off.
    pub fn insert_track(&mut self) {
        let track = self.cursor.track;
        if let Some(pattern) = self.active_pattern_mut() {
            pattern.insert_track(track);
        }
    }

    /// DEL.TRK: the cursor's track goes, the ones right of it moving left.
    pub fn delete_track(&mut self) {
        let track = self.cursor.track;
        if let Some(pattern) = self.active_pattern_mut() {
            pattern.delete_track(track);
        }
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

    pub fn new_block(&mut self) {
        let pattern = self.blank_pattern_like_active();
        self.append_block(pattern);
    }

    pub fn new_default_block(&mut self) {
        self.append_block(OctamedicPattern::new());
    }

    pub fn new_block_here(&mut self) {
        let pattern = self.blank_pattern_like_active();
        self.insert_block_here(pattern);
    }

    pub fn new_default_block_here(&mut self) {
        self.insert_block_here(OctamedicPattern::new());
    }

    fn blank_pattern_like_active(&self) -> OctamedicPattern {
        let mut pattern = OctamedicPattern::new();
        if let Some(active) = self.active_pattern() {
            pattern.set_track_count(active.track_count());
            pattern.set_line_count(active.line_count());
        }
        return pattern;
    }

    // appended to the sequence too, so it actually plays
    fn append_block(&mut self, pattern: OctamedicPattern) {
        if self.song().patterns.len() >= MAX_PATTERNS {
            self.notify("Too many blocks");
            return;
        }
        let index = self.song().patterns.len();
        let id = self.song_mut().insert_pattern(index, pattern);
        self.song_mut().sequence_mut().push(id);
        self.active_pattern = id;
        self.cursor = EditCursor::default();
        self.send_all_to_engine();
    }

    // not added to the sequence: the song still plays exactly what it did before
    fn insert_block_here(&mut self, pattern: OctamedicPattern) {
        if self.song().patterns.len() >= MAX_PATTERNS {
            self.notify("Too many blocks");
            return;
        }
        let index = self.active_pattern.0 as usize;
        let id = self.song_mut().insert_pattern(index, pattern);
        self.active_pattern = id;
        self.cursor = EditCursor::default();
        self.send_all_to_engine();
    }

    // no confirmation, matching the real OctaMED
    pub fn delete_last_block(&mut self) {
        let last = self.song().patterns.len().saturating_sub(1);
        self.remove_block(last);
    }

    pub fn delete_this_block(&mut self) {
        self.remove_block(self.active_pattern.0 as usize);
    }

    fn remove_block(&mut self, index: usize) {
        if self.song().patterns.len() <= 1 {
            self.notify("Can't delete the only block");
            return;
        }
        self.song_mut().remove_pattern(index);
        let last = self.song().patterns.len().saturating_sub(1) as u8;
        self.active_pattern.0 = self.active_pattern.0.min(last);
        self.clamp_cursor_to_active_pattern();
        self.keep_sequence_index_inside(self.active_sequence_index);
        self.send_all_to_engine();
    }

    pub fn split_block(&mut self) {
        let line = self.cursor.line;
        let can_split = line > 0 && self.active_pattern().is_some_and(|p| p.line_count() > 1);
        if !can_split {
            self.notify("Nothing to split there");
            return;
        }
        if self.song().patterns.len() >= MAX_PATTERNS {
            self.notify("Too many blocks");
            return;
        }
        let index = self.active_pattern.0 as usize;
        let tail = self
            .active_pattern_mut()
            .expect("just checked it's there")
            .split_at(line);
        self.song_mut().insert_pattern(index + 1, tail);
        self.cursor.line = 0;
        self.send_all_to_engine();
    }

    pub fn join_block(&mut self) {
        let index = self.active_pattern.0 as usize;
        let Some(next) = self.song().patterns.get(index + 1).cloned() else {
            self.notify("No block after this one");
            return;
        };
        let joined = self
            .active_pattern_mut()
            .is_some_and(|active| active.join(&next));
        if !joined {
            self.notify("Too many lines");
            return;
        }
        // what played the block below now plays the two of them, like what played this one
        for id in self.song_mut().sequence_mut() {
            if id.0 as usize == index + 1 {
                id.0 = index as u8;
            }
        }
        self.song_mut().remove_pattern(index + 1);
        self.send_all_to_engine();
    }

    /// SLIDE: VOL, volume commands going evenly between the ones around the cursor in its track.
    pub fn slide_volume(&mut self) {
        let EditCursor { line, track, .. } = self.cursor;
        let hex = self.song().flags.volumes_hex;
        let id = self.active_pattern;
        let Some(pattern) = self.song_mut().get_pattern_mut(id) else {
            return;
        };
        if pattern.slide_volume(line, track, hex) {
            self.changed_patterns.insert(id);
        }
    }

    pub fn cut_track(&mut self) {
        self.copy_track();
        let track = self.cursor.track;
        if let Some(pattern) = self.active_pattern_mut() {
            pattern.clear_track(track);
        }
    }

    pub fn copy_track(&mut self) {
        let track = self.cursor.track;
        if let Some(pattern) = self.active_pattern() {
            self.track_buffer = Some(pattern.copy_track(track));
        }
    }

    pub fn paste_track(&mut self) {
        let Some(buffer) = self.track_buffer.take() else {
            self.notify("Track buffer is empty");
            return;
        };
        let track = self.cursor.track;
        if let Some(pattern) = self.active_pattern_mut() {
            pattern.paste_track(track, &buffer);
        }
        self.track_buffer = Some(buffer);
    }

    pub fn swap_track(&mut self) {
        if self.track_buffer.is_none() {
            self.notify("Track buffer is empty");
            return;
        }
        let track = self.cursor.track;
        let Some(current) = self
            .active_pattern()
            .map(|pattern| pattern.copy_track(track))
        else {
            return;
        };
        let buffer = self.track_buffer.replace(current).expect("checked above");
        if let Some(pattern) = self.active_pattern_mut() {
            pattern.paste_track(track, &buffer);
        }
    }

    pub fn cut_block(&mut self) {
        self.copy_block();
        if let Some(pattern) = self.active_pattern_mut() {
            pattern.clear();
        }
    }

    pub fn copy_block(&mut self) {
        if let Some(pattern) = self.active_pattern() {
            let mut copy = pattern.clone();
            copy.name.clear();
            self.block_buffer = Some(copy);
        }
    }

    // the block takes the copied one's size too, so pasting into a new block makes an exact copy
    pub fn paste_block(&mut self) {
        let Some(buffer) = self.block_buffer.as_ref() else {
            self.notify("Block buffer is empty");
            return;
        };
        let lines = buffer.lines.clone();
        if let Some(pattern) = self.active_pattern_mut() {
            pattern.lines = lines;
        }
        self.clamp_cursor_to_active_pattern();
    }

    pub fn swap_block(&mut self) {
        let Some(mut buffer) = self.block_buffer.take() else {
            self.notify("Block buffer is empty");
            return;
        };
        if let Some(pattern) = self.active_pattern_mut() {
            std::mem::swap(&mut pattern.lines, &mut buffer.lines);
        }
        self.block_buffer = Some(buffer);
        self.clamp_cursor_to_active_pattern();
    }

    pub fn delete_buffer(&mut self) {
        self.track_buffer = None;
        self.block_buffer = None;
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
        // it's set straight on the field in places, so it's caught here rather than at each
        if self.engine_active_instrument != Some(self.active_instrument) {
            self.engine_active_instrument = Some(self.active_instrument);
            self.engine.set_active_instrument(self.active_instrument);
        }
    }

    fn edit_sequence(&mut self, edit: impl FnOnce(&mut Vec<PatternId>)) {
        edit(self.song_mut().sequence_mut());
        self.set_sequence(self.active_sequence_index);
        self.sequence_changed = true;
    }

    pub fn clear_all(&mut self) {
        self.load_project(OctamedicProject::new());
    }

    /// The song cleared but its instruments kept, with the editor back at its start.
    pub fn clear_song(&mut self) {
        self.stop();
        self.song_mut().clear();
        self.active_pattern = PatternId(0);
        self.cursor = EditCursor::default();
        self.active_sequence_index = 0;
        self.send_all_to_engine();
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::data::pattern::{DEFAULT_LINES, MAX_LINES};

    use super::*;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    #[test]
    fn loading_resets_the_song_state() {
        let mut state = state();
        state.set_song_name("old");
        state.toggle_track_mute(2);
        state.set_instrument(5);
        assert!(!state.is_saved());
        let mut project = OctamedicProject::new();
        project.get_song_mut(SongId(0)).unwrap().set_name("new");
        state.load_project(project);
        assert_eq!(state.song().name(), "new");
        assert!(state.is_saved());
        assert!(!state.is_track_muted(2));
        assert_eq!(state.active_instrument, InstrumentId(1));
    }

    #[test]
    fn clearing_the_song_keeps_its_instruments() {
        let mut state = state();
        state.set_song_name("old");
        let instrument = Some(OctamedicInstrument::new("kept"));
        state.song_mut().set_instrument(InstrumentId(1), instrument);
        state.song_mut().sequence_mut().push(PatternId(0));
        state.active_sequence_index = 1;
        state.clear_song();
        assert_eq!(state.song().name(), "");
        assert_eq!(state.song().sequence(), [PatternId(0)]);
        assert_eq!(state.active_sequence_index, 0);
        assert!(state.song().instrument(InstrumentId(1)).is_some());
    }

    #[test]
    fn clearing_all_drops_the_instruments_too() {
        let mut state = state();
        let instrument = Some(OctamedicInstrument::new("gone"));
        state.song_mut().set_instrument(InstrumentId(1), instrument);
        state.clear_all();
        assert!(state.song().instrument(InstrumentId(1)).is_none());
        assert!(state.is_saved());
    }

    #[test]
    fn loading_keeps_the_browser() {
        let mut state = state();
        state.browser.open(std::env::temp_dir()).unwrap();
        state.load_project(OctamedicProject::new());
        assert_eq!(state.browser.path(), std::env::temp_dir());
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

    #[test]
    fn new_block_appends_and_selects_it() {
        let mut state = state();
        state.active_pattern_mut().unwrap().set_line_count(10);
        state.new_block();
        assert_eq!(state.active_pattern, PatternId(1));
        assert_eq!(state.song().patterns.len(), 2);
        // matches the previously active block's dimensions, not the defaults
        assert_eq!(state.active_pattern().unwrap().line_count(), 10);
        assert_eq!(state.song().sequence(), [PatternId(0), PatternId(1)]);
    }

    #[test]
    fn new_default_block_uses_octameds_defaults() {
        let mut state = state();
        state.active_pattern_mut().unwrap().set_line_count(10);
        state.new_default_block();
        assert_eq!(state.active_pattern().unwrap().line_count(), DEFAULT_LINES);
    }

    #[test]
    fn new_block_here_inserts_and_shifts_the_rest() {
        let mut state = state();
        state.new_block(); // block 1, so there's something after block 0
        state.active_pattern = PatternId(0);
        state.new_block_here();
        assert_eq!(state.active_pattern, PatternId(0));
        assert_eq!(state.song().patterns.len(), 3);
        // the old block 0 and 1 moved to 1 and 2; the new one isn't added to the sequence
        assert_eq!(state.song().sequence(), [PatternId(1), PatternId(2)]);
    }

    #[test]
    fn delete_last_and_delete_this_keep_one_block() {
        let mut state = state();
        state.delete_last_block();
        assert_eq!(state.song().patterns.len(), 1, "the only block stays");
        state.new_block();
        state.active_pattern = PatternId(0);
        state.delete_this_block();
        assert_eq!(state.song().patterns.len(), 1);
        assert_eq!(state.active_pattern, PatternId(0));
    }

    #[test]
    fn split_then_join_round_trips() {
        let mut state = state();
        state.cursor.line = 32;
        state.split_block();
        assert_eq!(state.song().patterns.len(), 2);
        assert_eq!(state.active_pattern().unwrap().line_count(), 32);
        // the split-off tail isn't added to the sequence either
        assert_eq!(state.song().sequence(), [PatternId(0)]);
        state.join_block();
        assert_eq!(state.song().patterns.len(), 1);
        assert_eq!(state.active_pattern().unwrap().line_count(), 64);
    }

    #[test]
    fn joining_keeps_the_sequence_on_the_joined_block() {
        let mut state = state();
        state.new_block();
        state.new_block();
        assert_eq!(state.song().sequence(), [0, 1, 2].map(PatternId));
        state.active_pattern = PatternId(0);
        state.join_block();
        assert_eq!(state.song().sequence(), [0, 0, 1].map(PatternId));
    }

    #[test]
    fn blocks_are_capped_at_max_patterns() {
        let mut state = state();
        for _ in 0..MAX_PATTERNS + 5 {
            state.new_block();
        }
        assert_eq!(state.song().patterns.len(), MAX_PATTERNS);
        assert_eq!(state.notification.as_deref(), Some("Too many blocks"));
    }

    fn notes(state: &AppState, track: usize) -> Vec<u8> {
        return state
            .active_pattern()
            .unwrap()
            .lines
            .iter()
            .map(|line| line.tracks[track].note.value)
            .collect();
    }

    fn number_track(state: &mut AppState, track: usize, lines: usize) {
        let pattern = state.active_pattern_mut().unwrap();
        pattern.set_line_count(lines);
        for (index, line) in pattern.lines.iter_mut().enumerate() {
            line.tracks[track].note.value = index as u8 + 1;
        }
    }

    #[test]
    fn cutting_and_pasting_a_track() {
        let mut state = state();
        number_track(&mut state, 0, 3);
        state.cut_track();
        assert_eq!(notes(&state, 0), [0, 0, 0]);
        state.cursor.track = 2;
        state.paste_track();
        assert_eq!(notes(&state, 2), [1, 2, 3]);
        // the buffer stays for pasting again
        state.cursor.track = 3;
        state.paste_track();
        assert_eq!(notes(&state, 3), [1, 2, 3]);
    }

    #[test]
    fn swapping_a_track_with_the_buffer() {
        let mut state = state();
        number_track(&mut state, 0, 3);
        state.copy_track();
        state.cursor.track = 1;
        state.swap_track();
        assert_eq!(notes(&state, 1), [1, 2, 3]);
        // the buffer now holds the empty track 1
        state.cursor.track = 0;
        state.paste_track();
        assert_eq!(notes(&state, 0), [0, 0, 0]);
    }

    #[test]
    fn pasting_a_block_into_a_new_one_makes_a_twin() {
        let mut state = state();
        number_track(&mut state, 0, 3);
        state.copy_block();
        state.new_default_block();
        state.cursor.line = 40;
        state.paste_block();
        assert_eq!(notes(&state, 0), [1, 2, 3]);
        assert_eq!(state.cursor.line, 2);
    }

    #[test]
    fn cutting_and_swapping_blocks() {
        let mut state = state();
        number_track(&mut state, 0, 3);
        state.cut_block();
        assert_eq!(notes(&state, 0), [0, 0, 0]);
        number_track(&mut state, 1, 5);
        state.swap_block();
        assert_eq!(state.active_pattern().unwrap().line_count(), 3);
        assert_eq!(notes(&state, 0), [1, 2, 3]);
        state.swap_block();
        assert_eq!(notes(&state, 1), [1, 2, 3, 4, 5]);
    }

    #[test]
    fn pasting_with_an_empty_buffer_says_so() {
        let mut state = state();
        state.copy_track();
        state.copy_block();
        state.delete_buffer();
        state.paste_track();
        assert_eq!(state.notification.as_deref(), Some("Track buffer is empty"));
        state.paste_block();
        assert_eq!(state.notification.as_deref(), Some("Block buffer is empty"));
    }

    #[test]
    fn track_and_block_buffers_are_separate() {
        let mut state = state();
        number_track(&mut state, 0, 3);
        state.copy_block();
        state.cursor.track = 1;
        state.copy_track();
        // a copied track doesn't end up as the whole block
        state.swap_block();
        let pattern = state.active_pattern().unwrap();
        assert_eq!((pattern.line_count(), pattern.track_count()), (3, 4));
        assert_eq!(notes(&state, 0), [1, 2, 3]);
        // and the block copy didn't replace the track buffer
        state.cursor.track = 2;
        state.paste_track();
        assert_eq!(notes(&state, 2), [0, 0, 0]);
    }

    #[test]
    fn joining_past_max_lines_is_declined() {
        let mut state = state();
        state
            .active_pattern_mut()
            .unwrap()
            .set_line_count(MAX_LINES);
        state.new_block();
        state.active_pattern = PatternId(0);
        state.join_block();
        assert_eq!(state.song().patterns.len(), 2);
        assert_eq!(state.active_pattern().unwrap().line_count(), MAX_LINES);
        assert_eq!(state.notification.as_deref(), Some("Too many lines"));
    }

    #[test]
    fn deleting_a_block_drops_it_from_the_sequence() {
        let mut state = state();
        state.new_block();
        state.new_block();
        state.active_pattern = PatternId(1);
        state.delete_this_block();
        assert_eq!(state.song().sequence(), [0, 1].map(PatternId));
    }
}
