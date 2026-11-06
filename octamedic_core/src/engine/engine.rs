use std::{
    ops::Range,
    sync::{Arc, mpsc::Receiver},
};

use mmd::mmd::module::SONG_TRACKS;

use super::{
    handle::{EngineCommand, EngineStatus},
    period::{SampleNote, sample_note},
};
use crate::{
    amiga::{
        clock::AmigaClock,
        paula::{PaulaRenderer, PaulaWrite, Renderer, SILENCE},
    },
    data::{
        instrument::{InstrumentId, MAX_INSTRUMENT_VOLUME, OctamedicInstrument, OctamedicSample},
        note::OctamedicNote,
        pattern::{MAX_TRACKS, OctamedicPattern, OctamedicPatternTrack, PatternId},
        tempo::OctamedicTempo,
    },
};

const MAX_NOTE: u8 = 0x7F;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum PlayMode {
    Paused,
    Song,
    SongLoop,
    PatternLoop,
    Pattern,
}

impl PlayMode {
    pub fn from_u8(value: u8) -> Self {
        return match value {
            1 => PlayMode::Song,
            2 => PlayMode::SongLoop,
            3 => PlayMode::PatternLoop,
            4 => PlayMode::Pattern,
            _ => PlayMode::Paused,
        };
    }
}

/// Lives on the audio thread, the callback calls `process`.
pub struct OctamedicEngine {
    commands: Receiver<EngineCommand>,
    status: Arc<EngineStatus>,
    sample_rate: u32,
    machine: AmigaClock,

    patterns: Vec<Arc<OctamedicPattern>>,
    sequence: Vec<PatternId>,
    instruments: Vec<Option<OctamedicInstrument>>,
    tempo: OctamedicTempo,
    track_volumes: [u8; MAX_TRACKS],
    master_volume: u8,
    global_transpose: i8,
    // listener's volume, not saved with the song
    output_volume: u8,
    muted_tracks: [bool; SONG_TRACKS],

    mode: PlayMode,
    position: Position,
    queued_sequence_index: Option<usize>,
    clock: Option<Clock>,
    renderer: Box<dyn Renderer>,
    // last instrument per track, for notes with no instrument number
    track_instruments: [Option<InstrumentId>; MAX_TRACKS],
    // the instrument each track's voice was last started with, None for a preview
    voices: [Option<InstrumentId>; MAX_TRACKS],
    // before track and master volume
    note_volumes: [u8; MAX_TRACKS],
    holds: [Hold; MAX_TRACKS],
    active_instrument: Option<InstrumentId>,
}

impl OctamedicEngine {
    pub fn new(
        commands: Receiver<EngineCommand>,
        status: Arc<EngineStatus>,
        sample_rate: u32,
        machine: AmigaClock,
    ) -> Self {
        let scopes = status.scopes().clone();
        let renderer = PaulaRenderer::new(sample_rate, machine, MAX_TRACKS, scopes);
        let mut engine = Self::with_renderer(commands, status, sample_rate, Box::new(renderer));
        engine.machine = machine;
        return engine;
    }

    pub fn with_renderer(
        commands: Receiver<EngineCommand>,
        status: Arc<EngineStatus>,
        sample_rate: u32,
        renderer: Box<dyn Renderer>,
    ) -> Self {
        return Self {
            commands,
            status,
            sample_rate,
            machine: AmigaClock::PAL,
            patterns: Vec::new(),
            sequence: Vec::new(),
            instruments: Vec::new(),
            tempo: OctamedicTempo::new(),
            track_volumes: [MAX_INSTRUMENT_VOLUME; MAX_TRACKS],
            master_volume: MAX_INSTRUMENT_VOLUME,
            global_transpose: 0,
            output_volume: MAX_INSTRUMENT_VOLUME,
            muted_tracks: [false; SONG_TRACKS],
            mode: PlayMode::Paused,
            position: Position {
                sequence_index: 0,
                pattern: PatternId(0),
                row: 0,
            },
            queued_sequence_index: None,
            clock: None,
            renderer,
            track_instruments: [None; MAX_TRACKS],
            voices: [None; MAX_TRACKS],
            note_volumes: [0; MAX_TRACKS],
            holds: [Hold::default(); MAX_TRACKS],
            active_instrument: None,
        };
    }

    /// Fills `out`, interleaved. Ticks land on the exact frame they're due.
    pub fn process(&mut self, out: &mut [f32], channels: usize) {
        out.fill(0.0);
        while let Ok(command) = self.commands.try_recv() {
            self.apply(command);
        }
        let frame_count = out.len() / channels;
        let mut frame = 0;
        while frame < frame_count {
            let frames_until_tick = self.clock.as_ref().map(|clock| clock.frames_until_tick);
            let frames = match frames_until_tick {
                Some(due) if due <= 0.0 => {
                    self.tick();
                    continue;
                }
                Some(due) => (due.ceil() as usize).min(frame_count - frame),
                None => frame_count - frame,
            };
            self.renderer.render(
                &mut out[frame * channels..(frame + frames) * channels],
                channels,
            );
            if let Some(clock) = &mut self.clock {
                clock.frames_until_tick -= frames as f64;
            }
            frame += frames;
        }
        let gain = self.output_volume as f32 / MAX_INSTRUMENT_VOLUME as f32;
        out.iter_mut()
            .for_each(|sample| *sample = (*sample * gain).clamp(-1.0, 1.0));
        self.publish_sample_position();
    }

    fn publish_sample_position(&self) {
        let position = self.active_instrument.and_then(|active| {
            return (0..MAX_TRACKS)
                .filter(|&track| self.voices[track] == Some(active))
                .find_map(|track| self.renderer.sample_position(track));
        });
        self.status.set_sample_position(position);
    }

    fn apply(&mut self, command: EngineCommand) {
        match command {
            EngineCommand::PlayPattern {
                pattern,
                row,
                looping,
            } => self.play_pattern(pattern, row, looping),
            EngineCommand::PlaySong {
                sequence_start,
                row,
                looping,
            } => self.play_song(sequence_start, row, looping),
            EngineCommand::Stop => self.stop(),
            EngineCommand::SetPattern { id, pattern } => self.set_pattern(id, pattern),
            EngineCommand::SetSequence { sequence } => self.sequence = sequence,
            EngineCommand::QueueSequenceIndex(index) => self.queued_sequence_index = index,
            EngineCommand::SetInstrument { id, instrument } => self.set_instrument(id, instrument),
            EngineCommand::SetTempo(tempo) => self.tempo = tempo,
            EngineCommand::SetMachine(machine) => {
                self.machine = machine;
                self.renderer.set_clock(machine);
            }
            EngineCommand::SetFilter(on) => self.renderer.set_filter(on),
            EngineCommand::SetTrackVolume { track, volume } => {
                if let Some(track_volume) = self.track_volumes.get_mut(track) {
                    *track_volume = volume.min(MAX_INSTRUMENT_VOLUME);
                }
            }
            EngineCommand::SetMasterVolume(volume) => {
                self.master_volume = volume.min(MAX_INSTRUMENT_VOLUME)
            }
            EngineCommand::SetGlobalTranspose(transpose) => self.global_transpose = transpose,
            EngineCommand::SetTrackMuted { track, muted } => self.set_track_muted(track, muted),
            EngineCommand::SetOutputVolume(volume) => {
                self.output_volume = volume.min(MAX_INSTRUMENT_VOLUME)
            }
            EngineCommand::PlayNote {
                track,
                note,
                instrument,
            } => self.note_on(track, note, Some(instrument)),
            EngineCommand::ReleaseNote { track } => self.release(track),
            EngineCommand::Preview(Some((sample, note))) => {
                let played = sample_note(note.value, 0, 0, sample.octaves);
                self.trigger(0, sample, played, MAX_INSTRUMENT_VOLUME);
                self.voices[0] = None;
                self.holds[0] = Hold::default();
            }
            EngineCommand::Preview(None) => self.release(0),
            EngineCommand::SetActiveInstrument(instrument) => {
                self.active_instrument = Some(instrument)
            }
            EngineCommand::PlaySample {
                instrument,
                range,
                note,
            } => self.play_sample(instrument, range, note),
        }
    }

    fn set_track_muted(&mut self, track: usize, muted: bool) {
        let Some(track_muted) = self.muted_tracks.get_mut(track) else {
            return;
        };
        *track_muted = muted;
        // a looping sample would carry on forever otherwise
        if muted {
            self.renderer.write(track, PaulaWrite::DmaOff);
        }
    }

    fn set_pattern(&mut self, id: PatternId, pattern: Arc<OctamedicPattern>) {
        if id.0 as usize >= self.patterns.len() {
            self.patterns
                .resize_with(id.0 as usize + 1, || Arc::new(OctamedicPattern::new()));
        }
        self.patterns[id.0 as usize] = pattern;
    }

    fn set_instrument(&mut self, id: InstrumentId, instrument: Option<OctamedicInstrument>) {
        let Some(index) = id.index() else {
            return;
        };
        if index >= self.instruments.len() {
            self.instruments.resize_with(index + 1, || None);
        }
        self.instruments[index] = instrument;
    }

    fn play_pattern(&mut self, id: PatternId, row: usize, looping: bool) {
        self.position = Position {
            sequence_index: 0,
            pattern: id,
            row,
        };
        self.start(if looping {
            PlayMode::PatternLoop
        } else {
            PlayMode::Pattern
        });
    }

    fn play_song(&mut self, sequence_index: usize, row: usize, looping: bool) {
        let Some(&pattern) = self.sequence.get(sequence_index) else {
            return;
        };
        self.position = Position {
            sequence_index,
            pattern,
            row,
        };
        self.start(if looping {
            PlayMode::SongLoop
        } else {
            PlayMode::Song
        });
    }

    // first tick is due immediately. rows past the end clamp to the last line
    fn start(&mut self, mode: PlayMode) {
        let Some(lines) = self.current_pattern().map(OctamedicPattern::line_count) else {
            self.stop();
            return;
        };
        self.position.row = self.position.row.min(lines.saturating_sub(1));
        self.release_all();
        self.track_instruments = [None; MAX_TRACKS];
        self.note_volumes = [0; MAX_TRACKS];
        self.queued_sequence_index = None;
        self.clock = Some(Clock {
            tick: 0,
            frames_until_tick: 0.0,
        });
        self.mode = mode;
        self.status.set_play_mode(mode);
        self.publish_position();
    }

    fn stop(&mut self) {
        self.clock = None;
        self.mode = PlayMode::Paused;
        self.status.set_play_mode(PlayMode::Paused);
        self.release_all();
    }

    fn current_pattern(&self) -> Option<&OctamedicPattern> {
        return self
            .patterns
            .get(self.position.pattern.0 as usize)
            .map(Arc::as_ref);
    }

    fn ticks_per_line(&self) -> u32 {
        /* return self
        .effect_command_handler
        .speed()
        .unwrap_or(self.tempo.ticks_per_line)
        .max(1); */
        return self.tempo.ticks_per_line.max(1) as u32;
    }

    fn frames_per_tick(&self) -> f64 {
        // tempo commands will change the tempo used here
        return self.sample_rate as f64 * self.tempo.tick_seconds(self.machine);
    }

    fn tick(&mut self) {
        let frames_per_tick = self.frames_per_tick();
        let ticks_per_line = self.ticks_per_line();
        // advance at the start of the next tick, not the end of this one, or the last row of a
        // song loses its last tick
        if self
            .clock
            .as_ref()
            .is_some_and(|clock| clock.tick >= ticks_per_line)
        {
            self.advance_row();
        }
        let Some(clock) = &mut self.clock else {
            return;
        };
        if clock.tick >= ticks_per_line {
            clock.tick = 0;
        }
        clock.frames_until_tick += frames_per_tick;
        let tick = clock.tick;
        clock.tick += 1;
        if tick == 0 {
            self.play_row();
        }
        // todo effects
        for track in 0..MAX_TRACKS {
            self.hold_and_decay(track);
        }
    }

    fn hold_and_decay(&mut self, track: usize) {
        let hold = &mut self.holds[track];
        let Some(left) = hold.ticks_left else {
            return;
        };
        if left == 0 {
            let volume = &mut self.note_volumes[track];
            let step = if hold.decay == 0 { *volume } else { hold.decay };
            *volume = volume.saturating_sub(step);
            if *volume == 0 {
                hold.ticks_left = None;
            }
            let volume = self.scaled_volume(track, self.note_volumes[track]);
            self.write(track, PaulaWrite::Volume(volume));
        } else if !hold.sustained {
            hold.ticks_left = Some(left - 1);
        }
    }

    fn advance_row(&mut self) {
        match self.next_position(&self.position) {
            Some(position) => {
                if position.row == 0 {
                    self.queued_sequence_index = None;
                }
                self.position = position;
            }
            None => self.stop(),
        }
    }

    fn next_position(&self, position: &Position) -> Option<Position> {
        let mut next = *position;
        next.row += 1;
        let row_count = self
            .patterns
            .get(next.pattern.0 as usize)
            .map_or(0, |p| p.lines.len());
        if next.row < row_count {
            return Some(next);
        }
        next.row = 0;
        match self.mode {
            PlayMode::PatternLoop => return Some(next),
            PlayMode::Pattern => return None,
            _ => {}
        }
        next.sequence_index = match self.queued_sequence_index {
            Some(index) if index < self.sequence.len() => index,
            _ => next.sequence_index + 1,
        };
        if next.sequence_index >= self.sequence.len() {
            if self.mode == PlayMode::Song {
                return None;
            }
            next.sequence_index = 0;
        }
        next.pattern = *self.sequence.get(next.sequence_index)?;
        return Some(next);
    }

    fn publish_position(&self) {
        self.status.set_position(
            self.position.sequence_index,
            self.position.pattern.0 as usize,
            self.position.row,
        );
    }

    fn current_track(&self, track: usize) -> Option<OctamedicPatternTrack> {
        return self
            .current_pattern()?
            .lines
            .get(self.position.row)?
            .tracks
            .get(track)
            .copied();
    }

    fn play_row(&mut self) {
        self.publish_position();
        let track_count = self
            .current_pattern()
            .and_then(|pattern| pattern.lines.get(self.position.row))
            .map_or(0, |line| line.tracks.len())
            .min(MAX_TRACKS);
        for track in 0..track_count {
            let Some(line) = self.current_track(track) else {
                continue;
            };
            let instrument = (line.instrument_id.0 != 0).then_some(line.instrument_id);
            if line.note.value != 0 {
                self.note_on(track, line.note, instrument);
            } else if let Some(id) = instrument {
                self.hold_symbol(track, id);
            }
            self.holds[track].sustained = self.next_line_holds(track);
        }
    }

    // OctaMED 3.00 on resets the volume here
    fn hold_symbol(&mut self, track: usize, id: InstrumentId) {
        self.track_instruments[track] = Some(id);
        let Some(default_volume) = self.instrument(id).map(|instrument| instrument.volume) else {
            return;
        };
        self.note_volumes[track] = default_volume;
        let volume = self.scaled_volume(track, default_volume);
        self.write(track, PaulaWrite::Volume(volume));
    }

    // -|- on the next line pauses the hold on this one. Like OctaMED, not across blocks
    fn next_line_holds(&self, track: usize) -> bool {
        if self.holds[track].ticks_left.is_none_or(|left| left == 0) {
            return false;
        }
        let next = self
            .current_pattern()
            .and_then(|pattern| pattern.lines.get(self.position.row + 1))
            .and_then(|line| line.tracks.get(track));
        return next.is_some_and(|cell| cell.note.value == 0 && cell.instrument_id.0 != 0);
    }

    fn instrument(&self, id: InstrumentId) -> Option<&OctamedicInstrument> {
        return id
            .index()
            .and_then(|index| self.instruments.get(index))
            .and_then(Option::as_ref);
    }

    fn scaled_volume(&self, track: usize, volume: u8) -> u8 {
        let max = MAX_INSTRUMENT_VOLUME as u32;
        let volume = volume as u32 * self.track_volumes[track] as u32 / max
            * self.master_volume as u32
            / max;
        return volume as u8;
    }

    fn note_on(
        &mut self,
        track: usize,
        note: OctamedicNote,
        instrument_number: Option<InstrumentId>,
    ) {
        if track >= MAX_TRACKS || note.value == 0 || note.value > MAX_NOTE {
            return;
        }
        let instrument_id = instrument_number.or(self.track_instruments[track]);
        self.track_instruments[track] = instrument_id;
        let Some(instrument) = instrument_id.and_then(|id| self.instrument(id)) else {
            self.release(track);
            return;
        };
        let Some(sample) = instrument.sample().cloned() else {
            self.release(track);
            return;
        };
        let transpose = instrument.transpose.wrapping_add(self.global_transpose);
        let played = sample_note(note.value, transpose, instrument.finetune, sample.octaves);
        let hold = Hold::new(instrument.hold, instrument.decay);
        let default_volume = instrument.volume;
        // like MED, a note without an instrument number keeps the track's hold and volume
        if instrument_number.is_some() {
            self.holds[track] = hold;
            self.note_volumes[track] = default_volume;
        }
        let volume = self.scaled_volume(track, self.note_volumes[track]);
        if !self.is_muted(track) {
            let instrument = instrument_id.map_or(0, |id| id.0);
            self.status
                .note_triggered(track, note.value, instrument, volume);
        }
        self.trigger(track, sample, played, volume);
        self.voices[track] = instrument_id;
    }

    // same order the 68k player does it: dma off, sample, period, volume, dma on, then the
    // repeat range, which is the loop or silence
    fn trigger(
        &mut self,
        channel: usize,
        sample: Arc<OctamedicSample>,
        played: SampleNote,
        volume: u8,
    ) {
        let copy = sample.octave_copy(played.copy);
        let (end, repeat) = match copy.repeat {
            Some(repeat) => (
                repeat.end,
                PaulaWrite::Sample {
                    sample: sample.clone(),
                    start: repeat.start,
                    len: repeat.len(),
                },
            ),
            None => (
                copy.data.end,
                PaulaWrite::Sample {
                    sample: SILENCE.clone(),
                    start: 0,
                    len: SILENCE.data.len(),
                },
            ),
        };
        let writes = [
            PaulaWrite::DmaOff,
            PaulaWrite::Sample {
                sample,
                start: copy.data.start,
                len: end - copy.data.start,
            },
            PaulaWrite::Period(played.period),
            PaulaWrite::Volume(volume),
            PaulaWrite::DmaOn,
            repeat,
        ];
        for write in writes {
            self.write(channel, write);
        }
    }

    // the range is in bytes of the whole sample, so the note only picks the period, never an
    // octave copy
    fn play_sample(&mut self, instrument: InstrumentId, range: Range<usize>, note: OctamedicNote) {
        let Some(sample) = instrument
            .index()
            .and_then(|index| self.instruments.get(index))
            .and_then(Option::as_ref)
            .and_then(|instrument| instrument.sample().cloned())
        else {
            return;
        };
        let end = range.end.min(sample.data.len());
        if range.start >= end {
            return;
        }
        let period = sample_note(note.value, 0, 0, sample.octaves).period;
        let writes = [
            PaulaWrite::DmaOff,
            PaulaWrite::Sample {
                sample,
                start: range.start,
                len: end - range.start,
            },
            PaulaWrite::Period(period),
            PaulaWrite::Volume(MAX_INSTRUMENT_VOLUME),
            PaulaWrite::DmaOn,
            PaulaWrite::Sample {
                sample: SILENCE.clone(),
                start: 0,
                len: SILENCE.data.len(),
            },
        ];
        for write in writes {
            self.write(0, write);
        }
        self.voices[0] = Some(instrument);
        self.holds[0] = Hold::default();
    }

    fn release(&mut self, track: usize) {
        self.write(track, PaulaWrite::DmaOff);
        if let Some(hold) = self.holds.get_mut(track) {
            *hold = Hold::default();
        }
    }

    fn write(&mut self, channel: usize, write: PaulaWrite) {
        if !self.is_muted(channel) {
            self.renderer.write(channel, write);
        }
    }

    fn is_muted(&self, track: usize) -> bool {
        return self.muted_tracks.get(track).copied().unwrap_or(false);
    }

    fn release_all(&mut self) {
        for track in 0..MAX_TRACKS {
            self.release(track);
        }
    }
}

#[derive(Clone, Copy)]
struct Position {
    sequence_index: usize,
    pattern: PatternId,
    row: usize,
}

struct Clock {
    tick: u32,
    // keep the fraction or the tempo drifts
    frames_until_tick: f64,
}

#[derive(Clone, Copy, Default)]
struct Hold {
    // None without a hold, or once decayed
    ticks_left: Option<u32>,
    decay: u8,
    // the next line has a -|-
    sustained: bool,
}

impl Hold {
    fn new(hold: u8, decay: u8) -> Self {
        return Self {
            ticks_left: (hold > 0).then_some(hold as u32),
            decay,
            sustained: false,
        };
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Mutex,
        mpsc::{self, Sender},
    };

    use super::*;
    use crate::{
        data::instrument::{InstrumentKind, SampleOctaves},
        engine::period::note_period,
    };

    const SAMPLE_RATE: u32 = 1000;

    fn engine() -> (OctamedicEngine, Sender<EngineCommand>, Arc<EngineStatus>) {
        let (sender, receiver) = mpsc::channel();
        let status = Arc::new(EngineStatus::default());
        let engine = OctamedicEngine::new(receiver, status.clone(), SAMPLE_RATE, AmigaClock::PAL);
        return (engine, sender, status);
    }

    // ~10 ticks/s, 99.997 frames per tick
    fn tempo(ticks_per_line: u8) -> OctamedicTempo {
        let mut tempo = OctamedicTempo::new();
        tempo.primary_tempo = 25;
        tempo.lines_per_beat = 4;
        tempo.ticks_per_line = ticks_per_line;
        return tempo;
    }

    fn pattern(lines: usize, notes: &[(usize, u8)]) -> OctamedicPattern {
        let mut pattern = OctamedicPattern::new();
        pattern.lines.truncate(lines);
        for &(line, note) in notes {
            let track = &mut pattern.lines[line].tracks[0];
            track.note = OctamedicNote { value: note };
            track.instrument_id = InstrumentId(1);
        }
        return pattern;
    }

    fn instrument() -> OctamedicInstrument {
        let mut instrument = OctamedicInstrument::new("test");
        instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data: vec![127; 4],
            repeat: Some((0, 4)),
            octaves: SampleOctaves::One,
        }));
        return instrument;
    }

    fn run(engine: &mut OctamedicEngine, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames];
        engine.process(&mut out, 1);
        return out;
    }

    #[test]
    fn start_row() {
        let (mut engine, commands, status) = engine();
        commands.send(EngineCommand::SetTempo(tempo(3))).unwrap();
        commands
            .send(EngineCommand::SetPattern {
                id: PatternId(0),
                pattern: Arc::new(pattern(4, &[])),
            })
            .unwrap();
        for (row, starts_at) in [(2, 2), (10, 3)] {
            commands
                .send(EngineCommand::PlayPattern {
                    pattern: PatternId(0),
                    row,
                    looping: false,
                })
                .unwrap();
            run(&mut engine, 1);
            assert_eq!(status.row(), starts_at, "from line {row}");
        }
    }

    #[test]
    fn row_advance() {
        let (mut engine, commands, status) = engine();
        commands.send(EngineCommand::SetTempo(tempo(3))).unwrap();
        commands
            .send(EngineCommand::SetPattern {
                id: PatternId(0),
                pattern: Arc::new(pattern(4, &[])),
            })
            .unwrap();
        commands
            .send(EngineCommand::PlayPattern {
                pattern: PatternId(0),
                row: 0,
                looping: false,
            })
            .unwrap();
        run(&mut engine, 299);
        assert_eq!(status.row(), 0);
        run(&mut engine, 2);
        assert_eq!(status.row(), 1);
        run(&mut engine, 300);
        assert_eq!(status.row(), 2);
    }

    #[test]
    fn note_timing() {
        let (mut engine, commands, _) = engine();
        commands.send(EngineCommand::SetTempo(tempo(2))).unwrap();
        commands
            .send(EngineCommand::SetInstrument {
                id: InstrumentId(1),
                instrument: Some(instrument()),
            })
            .unwrap();
        commands
            .send(EngineCommand::SetPattern {
                id: PatternId(0),
                pattern: Arc::new(pattern(2, &[(1, 13)])),
            })
            .unwrap();
        commands
            .send(EngineCommand::PlayPattern {
                pattern: PatternId(0),
                row: 0,
                looping: true,
            })
            .unwrap();
        let out = run(&mut engine, 250);
        assert!(out[..200].iter().all(|&sample| sample == 0.0));
        assert!(out[200..].iter().all(|&sample| sample != 0.0));
    }

    #[test]
    fn song_end() {
        let (mut engine, commands, status) = engine();
        commands.send(EngineCommand::SetTempo(tempo(1))).unwrap();
        for id in 0..2 {
            commands
                .send(EngineCommand::SetPattern {
                    id: PatternId(id),
                    pattern: Arc::new(pattern(2, &[])),
                })
                .unwrap();
        }
        commands
            .send(EngineCommand::SetSequence {
                sequence: vec![PatternId(1), PatternId(0)],
            })
            .unwrap();
        commands
            .send(EngineCommand::PlaySong {
                sequence_start: 0,
                row: 0,
                looping: false,
            })
            .unwrap();
        run(&mut engine, 250);
        assert_eq!(
            (status.sequence_index(), status.pattern(), status.row()),
            (1, 0, 0)
        );
        assert_eq!(status.play_mode(), PlayMode::Song);
        run(&mut engine, 200);
        assert_eq!(status.play_mode(), PlayMode::Paused);
    }

    #[derive(Clone, Default)]
    struct Recorder {
        writes: Arc<Mutex<Vec<(usize, PaulaWrite)>>>,
    }

    impl Renderer for Recorder {
        fn write(&mut self, channel: usize, write: PaulaWrite) {
            self.writes.lock().unwrap().push((channel, write));
        }

        fn render(&mut self, _out: &mut [f32], _channels: usize) {}
    }

    fn recording_engine() -> (OctamedicEngine, Sender<EngineCommand>, Recorder) {
        let (sender, receiver) = mpsc::channel();
        let status = Arc::new(EngineStatus::default());
        let recorder = Recorder::default();
        let renderer = Box::new(recorder.clone());
        let engine = OctamedicEngine::with_renderer(receiver, status, SAMPLE_RATE, renderer);
        return (engine, sender, recorder);
    }

    // note on track 0, returns channel 0's writes
    fn note_writes(instrument: OctamedicInstrument, note: u8) -> Vec<PaulaWrite> {
        let (mut engine, commands, recorder) = recording_engine();
        commands
            .send(EngineCommand::SetInstrument {
                id: InstrumentId(1),
                instrument: Some(instrument),
            })
            .unwrap();
        commands
            .send(EngineCommand::PlayNote {
                track: 0,
                note: OctamedicNote { value: note },
                instrument: InstrumentId(1),
            })
            .unwrap();
        run(&mut engine, 1);
        return recorder
            .writes
            .lock()
            .unwrap()
            .iter()
            .filter(|(channel, _)| *channel == 0)
            .map(|(_, write)| write.clone())
            .collect();
    }

    #[test]
    fn play_sample_plays_just_the_range_once() {
        let (mut engine, commands, recorder) = recording_engine();
        let id = InstrumentId(1);
        // 4 bytes long
        let instrument = instrument();
        let sample = instrument.sample().cloned().unwrap();
        let set = EngineCommand::SetInstrument {
            id,
            instrument: Some(instrument),
        };
        commands.send(set).unwrap();
        let note = OctamedicNote { value: 13 };
        // past the end is cut short, and all of it past the end plays nothing. so does an
        // instrument that isn't there
        let plays = [
            (id, 1..3),
            (id, 2..9),
            (id, 100..200),
            (InstrumentId(2), 0..4),
        ];
        for (instrument, range) in plays {
            let command = EngineCommand::PlaySample {
                instrument,
                range,
                note,
            };
            commands.send(command).unwrap();
        }
        run(&mut engine, 1);
        let writes = recorder.writes.lock().unwrap();
        let channel_0: Vec<&PaulaWrite> = writes
            .iter()
            .filter(|(channel, _)| *channel == 0)
            .map(|(_, write)| write)
            .collect();
        let played: Vec<(usize, usize)> = channel_0
            .iter()
            .filter_map(|write| match write {
                PaulaWrite::Sample {
                    sample: played,
                    start,
                    len,
                } if Arc::ptr_eq(played, &sample) => Some((*start, *len)),
                _ => None,
            })
            .collect();
        assert_eq!(played, [(1, 2), (2, 2)]);
        assert!(channel_0.contains(&&PaulaWrite::Period(note_period(13, 0, 0))));
        // no loop: silence takes over at the end
        let silence = PaulaWrite::Sample {
            sample: SILENCE.clone(),
            start: 0,
            len: SILENCE.data.len(),
        };
        assert_eq!(channel_0.last(), Some(&&silence));
    }

    #[test]
    fn status_follows_the_active_instruments_voice() {
        let (mut engine, commands, status) = engine();
        let mut long = instrument();
        long.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data: vec![1; 50_000],
            repeat: None,
            octaves: SampleOctaves::One,
        }));
        for id in [InstrumentId(1), InstrumentId(2)] {
            let instrument = Some(long.clone());
            commands
                .send(EngineCommand::SetInstrument { id, instrument })
                .unwrap();
        }
        let note = OctamedicNote { value: 13 };
        let play_sample = EngineCommand::PlaySample {
            instrument: InstrumentId(1),
            range: 20_000..50_000,
            note,
        };
        commands.send(play_sample).unwrap();
        let play_note = EngineCommand::PlayNote {
            track: 3,
            note,
            instrument: InstrumentId(2),
        };
        commands.send(play_note).unwrap();
        run(&mut engine, 64);
        assert_eq!(status.sample_position(), None);
        commands
            .send(EngineCommand::SetActiveInstrument(InstrumentId(1)))
            .unwrap();
        run(&mut engine, 64);
        let first = status.sample_position().unwrap();
        assert!(first > 20_000, "{first}");
        run(&mut engine, 64);
        let later = status.sample_position().unwrap();
        assert!(later > first, "{later} {first}");
        commands
            .send(EngineCommand::SetActiveInstrument(InstrumentId(2)))
            .unwrap();
        run(&mut engine, 64);
        let other = status.sample_position().unwrap();
        assert!(other < 20_000, "{other}");
        commands
            .send(EngineCommand::ReleaseNote { track: 3 })
            .unwrap();
        run(&mut engine, 64);
        assert_eq!(status.sample_position(), None);
    }

    #[test]
    fn preview_plays_without_an_instrument() {
        let (mut engine, commands, recorder) = recording_engine();
        let sample = instrument().sample().cloned().unwrap();
        let note = OctamedicNote { value: 13 };
        commands
            .send(EngineCommand::Preview(Some((sample, note))))
            .unwrap();
        commands.send(EngineCommand::Preview(None)).unwrap();
        run(&mut engine, 1);
        let writes = recorder.writes.lock().unwrap();
        let channel_0: Vec<&PaulaWrite> = writes
            .iter()
            .filter(|(channel, _)| *channel == 0)
            .map(|(_, write)| write)
            .collect();
        assert!(channel_0.contains(&&PaulaWrite::Period(note_period(13, 0, 0))));
        assert_eq!(channel_0.last(), Some(&&PaulaWrite::DmaOff));
    }

    #[test]
    fn multi_octave_notes_play_their_copy() {
        let mut instrument = OctamedicInstrument::new("bass");
        instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data: (0..14).collect(),
            repeat: None,
            octaves: SampleOctaves::Three,
        }));
        // C-1 on a 3 octave sample is the longest copy, 8 bytes from byte 6, at C-1's period
        let writes = note_writes(instrument, 1);
        assert!(writes.iter().any(|write| matches!(
            write,
            PaulaWrite::Sample {
                start: 6,
                len: 8,
                ..
            }
        )));
        assert!(writes.contains(&PaulaWrite::Period(856)));
    }

    #[test]
    fn note_writes_order() {
        let instrument = instrument();
        let sample = instrument.sample().cloned().unwrap();
        let writes = note_writes(instrument, 13);
        assert_eq!(
            writes,
            vec![
                PaulaWrite::DmaOff,
                PaulaWrite::Sample {
                    sample: sample.clone(),
                    start: 0,
                    len: 4,
                },
                PaulaWrite::Period(428),
                PaulaWrite::Volume(64),
                PaulaWrite::DmaOn,
                PaulaWrite::Sample {
                    sample,
                    start: 0,
                    len: 4,
                },
            ]
        );
    }

    #[test]
    fn note_periods() {
        let period = |note| {
            note_writes(instrument(), note)
                .into_iter()
                .find_map(|write| match write {
                    PaulaWrite::Period(period) => Some(period),
                    _ => None,
                })
        };
        assert_eq!(period(1), Some(856));
        assert_eq!(period(13), Some(428));
        assert_eq!(period(25), Some(214));
    }

    #[test]
    fn one_shot() {
        let mut instrument = instrument();
        instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data: vec![127; 6],
            repeat: None,
            octaves: SampleOctaves::One,
        }));
        let writes = note_writes(instrument, 13);
        assert!(matches!(
            &writes[1],
            PaulaWrite::Sample {
                start: 0,
                len: 6,
                ..
            }
        ));
        assert_eq!(
            writes.last(),
            Some(&PaulaWrite::Sample {
                sample: SILENCE.clone(),
                start: 0,
                len: 2,
            })
        );
    }

    #[test]
    fn looped() {
        let mut instrument = instrument();
        let sample = Arc::new(OctamedicSample {
            data: vec![127; 10],
            repeat: Some((2, 6)),
            octaves: SampleOctaves::One,
        });
        instrument.kind = InstrumentKind::Sample(sample.clone());
        let writes = note_writes(instrument, 13);
        assert_eq!(
            writes[1],
            PaulaWrite::Sample {
                sample: sample.clone(),
                start: 0,
                len: 6,
            }
        );
        assert_eq!(
            writes.last(),
            Some(&PaulaWrite::Sample {
                sample,
                start: 2,
                len: 4,
            })
        );
    }

    #[test]
    fn finetune() {
        let mut tuned = instrument();
        tuned.finetune = -8;
        let periods: Vec<u16> = note_writes(tuned, 13)
            .into_iter()
            .filter_map(|write| match write {
                PaulaWrite::Period(period) => Some(period),
                _ => None,
            })
            .collect();
        assert_eq!(periods, vec![453]);
    }

    #[test]
    fn output_volume() {
        let (mut engine, commands, _) = engine();
        commands
            .send(EngineCommand::SetInstrument {
                id: InstrumentId(1),
                instrument: Some(instrument()),
            })
            .unwrap();
        commands
            .send(EngineCommand::PlayNote {
                track: 0,
                note: OctamedicNote { value: 13 },
                instrument: InstrumentId(1),
            })
            .unwrap();
        let full = run(&mut engine, 1)[0];
        // already playing note should drop too
        commands.send(EngineCommand::SetOutputVolume(16)).unwrap();
        let quarter = run(&mut engine, 1)[0];
        assert!(full > 0.0);
        assert!((quarter - full / 4.0).abs() < 1e-6, "{full} {quarter}");
    }

    #[test]
    fn output_volume_kept() {
        let (mut engine, commands, _) = engine();
        commands.send(EngineCommand::SetOutputVolume(10)).unwrap();
        commands.send(EngineCommand::SetMasterVolume(64)).unwrap();
        run(&mut engine, 1);
        assert_eq!(engine.output_volume, 10);
    }

    fn play_note(commands: &Sender<EngineCommand>, track: usize) {
        commands
            .send(EngineCommand::PlayNote {
                track,
                note: OctamedicNote { value: 13 },
                instrument: InstrumentId(1),
            })
            .unwrap();
    }

    fn channel_writes(recorder: &Recorder, channel: usize) -> Vec<PaulaWrite> {
        return recorder
            .writes
            .lock()
            .unwrap()
            .iter()
            .filter(|(written, _)| *written == channel)
            .map(|(_, write)| write.clone())
            .collect();
    }

    #[test]
    fn muted_track_writes_nothing() {
        let (mut engine, commands, recorder) = recording_engine();
        commands
            .send(EngineCommand::SetInstrument {
                id: InstrumentId(1),
                instrument: Some(instrument()),
            })
            .unwrap();
        commands
            .send(EngineCommand::SetTrackMuted {
                track: 1,
                muted: true,
            })
            .unwrap();
        run(&mut engine, 1);
        recorder.writes.lock().unwrap().clear();
        play_note(&commands, 0);
        play_note(&commands, 1);
        commands
            .send(EngineCommand::ReleaseNote { track: 1 })
            .unwrap();
        run(&mut engine, 1);
        assert!(channel_writes(&recorder, 1).is_empty());
        assert!(channel_writes(&recorder, 0).contains(&PaulaWrite::DmaOn));
    }

    #[test]
    fn muting_cuts_and_unmuting_plays() {
        let (mut engine, commands, recorder) = recording_engine();
        commands
            .send(EngineCommand::SetInstrument {
                id: InstrumentId(1),
                instrument: Some(instrument()),
            })
            .unwrap();
        play_note(&commands, 2);
        run(&mut engine, 1);
        recorder.writes.lock().unwrap().clear();
        commands
            .send(EngineCommand::SetTrackMuted {
                track: 2,
                muted: true,
            })
            .unwrap();
        run(&mut engine, 1);
        assert_eq!(channel_writes(&recorder, 2), vec![PaulaWrite::DmaOff]);
        commands
            .send(EngineCommand::SetTrackMuted {
                track: 2,
                muted: false,
            })
            .unwrap();
        play_note(&commands, 2);
        run(&mut engine, 1);
        assert!(channel_writes(&recorder, 2).contains(&PaulaWrite::DmaOn));
    }

    #[test]
    fn muted_track_is_silent() {
        let (mut engine, commands, _) = engine();
        commands
            .send(EngineCommand::SetInstrument {
                id: InstrumentId(1),
                instrument: Some(instrument()),
            })
            .unwrap();
        play_note(&commands, 0);
        assert!(run(&mut engine, 1)[0] > 0.0);
        commands
            .send(EngineCommand::SetTrackMuted {
                track: 0,
                muted: true,
            })
            .unwrap();
        assert_eq!(run(&mut engine, 1)[0], 0.0);
    }

    #[test]
    fn mutes_past_song_tracks_ignored() {
        let (mut engine, commands, _) = engine();
        commands
            .send(EngineCommand::SetTrackMuted {
                track: SONG_TRACKS,
                muted: true,
            })
            .unwrap();
        run(&mut engine, 1);
        assert_eq!(engine.muted_tracks, [false; SONG_TRACKS]);
    }

    fn play_sequence(
        sequence: Vec<u8>,
    ) -> (OctamedicEngine, Sender<EngineCommand>, Arc<EngineStatus>) {
        let (engine, commands, status) = engine();
        commands.send(EngineCommand::SetTempo(tempo(1))).unwrap();
        for id in 0..3 {
            commands
                .send(EngineCommand::SetPattern {
                    id: PatternId(id),
                    pattern: Arc::new(pattern(2, &[])),
                })
                .unwrap();
        }
        commands
            .send(EngineCommand::SetSequence {
                sequence: sequence.into_iter().map(PatternId).collect(),
            })
            .unwrap();
        commands
            .send(EngineCommand::PlaySong {
                sequence_start: 0,
                row: 0,
                looping: true,
            })
            .unwrap();
        return (engine, commands, status);
    }

    #[test]
    fn queued_index_waits_for_block_end() {
        let (mut engine, commands, status) = play_sequence(vec![0, 1, 2, 0]);
        run(&mut engine, 50);
        commands
            .send(EngineCommand::QueueSequenceIndex(Some(3)))
            .unwrap();
        run(&mut engine, 100);
        assert_eq!(status.sequence_index(), 0);
        run(&mut engine, 100);
        assert_eq!(status.sequence_index(), 3);
        run(&mut engine, 200);
        assert_eq!(status.sequence_index(), 0);
    }

    #[test]
    fn queue_cancelled() {
        let (mut engine, commands, status) = play_sequence(vec![0, 1, 2]);
        run(&mut engine, 50);
        commands
            .send(EngineCommand::QueueSequenceIndex(Some(2)))
            .unwrap();
        commands
            .send(EngineCommand::QueueSequenceIndex(None))
            .unwrap();
        run(&mut engine, 200);
        assert_eq!(status.sequence_index(), 1);
    }

    #[test]
    fn sequence_edit_waits_for_block_end() {
        let (mut engine, commands, status) = play_sequence(vec![0, 1]);
        run(&mut engine, 50);
        commands
            .send(EngineCommand::SetSequence {
                sequence: vec![PatternId(0), PatternId(2)],
            })
            .unwrap();
        run(&mut engine, 100);
        assert_eq!(status.pattern(), 0);
        run(&mut engine, 100);
        assert_eq!((status.sequence_index(), status.pattern()), (1, 2));
    }

    #[test]
    fn notes_leave_a_trigger() {
        let (mut engine, commands, status) = engine();
        commands
            .send(EngineCommand::SetInstrument {
                id: InstrumentId(1),
                instrument: Some(instrument()),
            })
            .unwrap();
        commands
            .send(EngineCommand::SetTrackMuted {
                track: 1,
                muted: true,
            })
            .unwrap();
        play_note(&commands, 0);
        play_note(&commands, 1);
        run(&mut engine, 1);
        let trigger = status.note_trigger(0);
        assert_eq!(
            (trigger.count, trigger.note, trigger.instrument),
            (1, 13, 1)
        );
        assert_eq!(trigger.volume, 64);
        assert_eq!(status.note_trigger(1).count, 0);
        play_note(&commands, 0);
        run(&mut engine, 1);
        assert_eq!(status.note_trigger(0).count, 2);
    }

    #[test]
    fn machine_sets_tick_rate() {
        let (mut engine, commands, _) = engine();
        let pal = engine.frames_per_tick();
        commands
            .send(EngineCommand::SetMachine(AmigaClock::NTSC))
            .unwrap();
        run(&mut engine, 1);
        assert!(engine.frames_per_tick() < pal);
    }

    #[derive(Clone, Copy)]
    enum Cell {
        Note,
        NoInstrument,
        HoldSymbol,
    }

    // channel 0's volume after each tick, 0 while DMA's off
    fn volume_per_tick(
        hold: u8,
        decay: u8,
        ticks_per_line: u8,
        cells: &[(usize, Cell)],
        ticks: usize,
    ) -> Vec<u8> {
        let (mut engine, commands, recorder) = recording_engine();
        let mut instrument = instrument();
        instrument.hold = hold;
        instrument.decay = decay;
        commands
            .send(EngineCommand::SetInstrument {
                id: InstrumentId(1),
                instrument: Some(instrument),
            })
            .unwrap();
        let mut block = pattern(16, &[]);
        for &(line, cell) in cells {
            let track = &mut block.lines[line].tracks[0];
            match cell {
                Cell::Note => {
                    track.note.value = 13;
                    track.instrument_id = InstrumentId(1);
                }
                Cell::NoInstrument => track.note.value = 13,
                Cell::HoldSymbol => track.instrument_id = InstrumentId(1),
            }
        }
        commands
            .send(EngineCommand::SetPattern {
                id: PatternId(0),
                pattern: Arc::new(block),
            })
            .unwrap();
        commands
            .send(EngineCommand::SetTempo(tempo(ticks_per_line)))
            .unwrap();
        commands
            .send(EngineCommand::PlayPattern {
                pattern: PatternId(0),
                row: 0,
                looping: false,
            })
            .unwrap();
        let (mut volume, mut on) = (0, false);
        let mut heard = Vec::new();
        for _ in 0..ticks {
            recorder.writes.lock().unwrap().clear();
            // a tick is just under 100 frames, so this runs exactly the one due at its start
            run(&mut engine, 100);
            for write in channel_writes(&recorder, 0) {
                match write {
                    PaulaWrite::Volume(written) => volume = written,
                    PaulaWrite::DmaOn => on = true,
                    PaulaWrite::DmaOff => on = false,
                    _ => {}
                }
            }
            heard.push(if on { volume } else { 0 });
        }
        return heard;
    }

    const FULL: u8 = MAX_INSTRUMENT_VOLUME;

    #[test]
    fn hold_cuts_the_note_after_its_ticks() {
        // the manual's example
        let heard = volume_per_tick(2, 0, 6, &[(0, Cell::Note), (1, Cell::Note)], 9);
        assert_eq!(heard, [FULL, FULL, 0, 0, 0, 0, FULL, FULL, 0]);
    }

    #[test]
    fn no_hold_plays_on() {
        let heard = volume_per_tick(0, 3, 6, &[(0, Cell::Note)], 12);
        assert!(heard.iter().all(|&volume| volume == FULL));
    }

    #[test]
    fn decay_fades_by_its_value_each_tick() {
        let heard = volume_per_tick(2, 16, 6, &[(0, Cell::Note)], 8);
        assert_eq!(heard, [FULL, FULL, 48, 32, 16, 0, 0, 0]);
    }

    #[test]
    fn hold_symbols_keep_the_note_going() {
        // the manual's 08 example, 6 + 6 + 4 ticks
        let cells = [
            (0, Cell::Note),
            (1, Cell::HoldSymbol),
            (2, Cell::HoldSymbol),
        ];
        let heard = volume_per_tick(4, 0, 6, &cells, 20);
        assert!(heard[..16].iter().all(|&volume| volume == FULL));
        assert!(heard[16..].iter().all(|&volume| volume == 0));
    }

    #[test]
    fn a_note_without_an_instrument_number_keeps_the_hold_going() {
        // still cut 8 ticks after line 0's note
        let heard = volume_per_tick(8, 0, 6, &[(0, Cell::Note), (1, Cell::NoInstrument)], 10);
        assert_eq!(&heard[6..], [FULL, FULL, 0, 0]);
        // silent once decayed, like MED
        let heard = volume_per_tick(1, 0, 6, &[(0, Cell::Note), (1, Cell::NoInstrument)], 8);
        assert_eq!(&heard[6..], [0, 0]);
    }

    #[test]
    fn a_hold_symbol_puts_the_volume_back() {
        // -|- after the decay brings the volume back for good
        let cells = [(0, Cell::Note), (2, Cell::HoldSymbol)];
        let heard = volume_per_tick(1, 32, 6, &cells, 18);
        assert_eq!(&heard[..4], [FULL, 32, 0, 0]);
        assert!(heard[12..].iter().all(|&volume| volume == FULL));
    }
}
