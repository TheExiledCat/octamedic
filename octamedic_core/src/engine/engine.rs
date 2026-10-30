use std::sync::{Arc, mpsc::Receiver};

use super::{
    handle::{EngineCommand, EngineStatus},
    period::note_period,
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

/// Anything higher is not a note.
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
        const ALL: [PlayMode; 5] = [
            PlayMode::Paused,
            PlayMode::Song,
            PlayMode::SongLoop,
            PlayMode::PatternLoop,
            PlayMode::Pattern,
        ];
        return ALL
            .into_iter()
            .find(|mode| *mode as u8 == value)
            .unwrap_or(PlayMode::Paused);
    }
}

#[derive(Clone, Copy)]
struct Position {
    sequence_index: usize,
    pattern: PatternId,
    row: usize,
}

/// Counted in output frames, so ticks land on the right sample.
struct Clock {
    /// The tick within the row; its notes play on tick 0.
    tick: u32,
    /// Keeps the fraction, so the tempo doesn't drift.
    frames_until_tick: f64,
}

/// Runs inside the audio callback, which drives it through `process`.
pub struct OctamedicEngine {
    commands: Receiver<EngineCommand>,
    status: Arc<EngineStatus>,
    sample_rate: u32,
    /// PAL or NTSC timing.
    machine: AmigaClock,

    patterns: Vec<Arc<OctamedicPattern>>,
    sequence: Vec<PatternId>,
    instruments: Vec<Option<OctamedicInstrument>>,
    tempo: OctamedicTempo,
    track_volumes: [u8; MAX_TRACKS],
    master_volume: u8,
    global_transpose: i8,
    // the listener's volume, not part of the song
    output_volume: u8,

    mode: PlayMode,
    position: Position,
    clock: Option<Clock>,
    renderer: Box<dyn Renderer>,
    /// For notes without an instrument number.
    track_instruments: [Option<InstrumentId>; MAX_TRACKS],
}

impl OctamedicEngine {
    pub fn new(
        commands: Receiver<EngineCommand>,
        status: Arc<EngineStatus>,
        sample_rate: u32,
    ) -> Self {
        let renderer = PaulaRenderer::new(sample_rate, AmigaClock::PAL, MAX_TRACKS);
        return Self::with_renderer(commands, status, sample_rate, Box::new(renderer));
    }

    /// Plays through `renderer` instead of Paula.
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
            mode: PlayMode::Paused,
            position: Position {
                sequence_index: 0,
                pattern: PatternId(0),
                row: 0,
            },
            clock: None,
            renderer,
            track_instruments: [None; MAX_TRACKS],
        };
    }

    /// Fills `out` (interleaved, `channels` per frame). UI commands run first, then each tick
    /// on the frame it falls on.
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
    }

    fn apply(&mut self, command: EngineCommand) {
        match command {
            EngineCommand::PlayPattern { pattern, looping } => self.play_pattern(pattern, looping),
            EngineCommand::PlaySong {
                sequence_start,
                looping,
            } => self.play_song(sequence_start, looping),
            EngineCommand::Stop => self.stop(),
            EngineCommand::SetPattern { id, pattern } => self.set_pattern(id, pattern),
            EngineCommand::SetSequence { sequence } => self.sequence = sequence,
            EngineCommand::SetInstrument { id, instrument } => self.set_instrument(id, instrument),
            EngineCommand::SetTempo(tempo) => self.tempo = tempo,
            EngineCommand::SetTrackVolume { track, volume } => {
                if let Some(track_volume) = self.track_volumes.get_mut(track) {
                    *track_volume = volume.min(MAX_INSTRUMENT_VOLUME);
                }
            }
            EngineCommand::SetMasterVolume(volume) => {
                self.master_volume = volume.min(MAX_INSTRUMENT_VOLUME)
            }
            EngineCommand::SetGlobalTranspose(transpose) => self.global_transpose = transpose,
            EngineCommand::SetOutputVolume(volume) => {
                self.output_volume = volume.min(MAX_INSTRUMENT_VOLUME)
            }
            EngineCommand::PlayNote {
                track,
                note,
                instrument,
            } => self.note_on(track, note, Some(instrument)),
            EngineCommand::ReleaseNote { track } => self.release(track),
        }
    }

    fn set_pattern(&mut self, id: PatternId, pattern: Arc<OctamedicPattern>) {
        if id.0 as usize >= self.patterns.len() {
            self.patterns
                .resize_with(id.0 as usize + 1, || Arc::new(OctamedicPattern::new()));
        }
        self.patterns[id.0 as usize] = pattern;
    }

    /// `None` clears the slot.
    fn set_instrument(&mut self, id: InstrumentId, instrument: Option<OctamedicInstrument>) {
        let Some(index) = id.index() else {
            return;
        };
        if index >= self.instruments.len() {
            self.instruments.resize_with(index + 1, || None);
        }
        self.instruments[index] = instrument;
    }

    fn play_pattern(&mut self, id: PatternId, looping: bool) {
        self.position = Position {
            sequence_index: 0,
            pattern: id,
            row: 0,
        };
        self.start(if looping {
            PlayMode::PatternLoop
        } else {
            PlayMode::Pattern
        });
    }

    fn play_song(&mut self, sequence_index: usize, looping: bool) {
        let Some(&pattern) = self.sequence.get(sequence_index) else {
            return;
        };
        self.position = Position {
            sequence_index,
            pattern,
            row: 0,
        };
        self.start(if looping {
            PlayMode::SongLoop
        } else {
            PlayMode::Song
        });
    }

    /// The first row plays on the next tick, which is due straight away.
    fn start(&mut self, mode: PlayMode) {
        if self.current_pattern().is_none() {
            self.stop();
            return;
        }
        self.release_all();
        self.track_instruments = [None; MAX_TRACKS];
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

    /// Plays the row on its first tick and moves on after its last.
    fn tick(&mut self) {
        let frames_per_tick = self.frames_per_tick();
        let ticks_per_line = self.ticks_per_line();
        // the previous row ends when this tick starts, so the last row of a song gets its last tick
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
        // todo effects: run the tick's effects here.
    }

    fn advance_row(&mut self) {
        match self.next_position(&self.position) {
            Some(position) => self.position = position,
            None => self.stop(),
        }
    }

    /// `None` when the song ends.
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
        next.sequence_index += 1;
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
            } else if instrument.is_some() {
                self.track_instruments[track] = instrument;
            }
        }
    }

    /// Uses the track's last instrument when `instrument` is `None`.
    fn note_on(&mut self, track: usize, note: OctamedicNote, instrument: Option<InstrumentId>) {
        if track >= MAX_TRACKS || note.value == 0 || note.value > MAX_NOTE {
            return;
        }
        let instrument_id = instrument.or(self.track_instruments[track]);
        self.track_instruments[track] = instrument_id;
        let Some(instrument) = instrument_id
            .and_then(|id| id.index())
            .and_then(|index| self.instruments.get(index))
            .and_then(Option::as_ref)
        else {
            self.release(track);
            return;
        };
        let Some(sample) = instrument.sample.clone() else {
            self.release(track);
            return;
        };
        let transpose = instrument.transpose.wrapping_add(self.global_transpose);
        let period = note_period(note.value, transpose, instrument.finetune);
        let max = MAX_INSTRUMENT_VOLUME as u32;
        let volume = instrument.volume as u32 * self.track_volumes[track] as u32 / max
            * self.master_volume as u32
            / max;
        self.trigger(track, sample, period, volume as u8);
    }

    /// Like an Amiga player: DMA off, first range, period, volume, DMA on, then the range to
    /// repeat after the first, which is the loop or silence.
    fn trigger(&mut self, channel: usize, sample: Arc<OctamedicSample>, period: u16, volume: u8) {
        let (first, repeat) = match sample.repeat {
            Some((start, end)) => (
                end,
                PaulaWrite::Sample {
                    sample: sample.clone(),
                    start,
                    len: end - start,
                },
            ),
            None => (
                sample.data.len(),
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
                start: 0,
                len: first,
            },
            PaulaWrite::Period(period),
            PaulaWrite::Volume(volume),
            PaulaWrite::DmaOn,
            repeat,
        ];
        for write in writes {
            self.renderer.write(channel, write);
        }
    }

    fn release(&mut self, track: usize) {
        self.renderer.write(track, PaulaWrite::DmaOff);
    }

    fn release_all(&mut self) {
        for track in 0..MAX_TRACKS {
            self.release(track);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Mutex,
        mpsc::{self, Sender},
    };

    use super::*;

    const SAMPLE_RATE: u32 = 1000;

    fn engine() -> (OctamedicEngine, Sender<EngineCommand>, Arc<EngineStatus>) {
        let (sender, receiver) = mpsc::channel();
        let status = Arc::new(EngineStatus::default());
        let engine = OctamedicEngine::new(receiver, status.clone(), SAMPLE_RATE);
        return (engine, sender, status);
    }

    /// About 10 ticks a second: 99.997 frames per tick at `SAMPLE_RATE`.
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
        instrument.sample = Some(Arc::new(OctamedicSample {
            data: vec![127; 4],
            repeat: Some((0, 4)),
        }));
        return instrument;
    }

    fn run(engine: &mut OctamedicEngine, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames];
        engine.process(&mut out, 1);
        return out;
    }

    #[test]
    fn rows_advance_after_ticks_per_line_ticks() {
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
    fn notes_start_on_the_frame_their_row_starts() {
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
                looping: true,
            })
            .unwrap();
        let out = run(&mut engine, 250);
        assert!(out[..200].iter().all(|&sample| sample == 0.0));
        assert!(out[200..].iter().all(|&sample| sample != 0.0));
    }

    #[test]
    fn song_stops_after_the_last_sequence_entry() {
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

    /// Records every Paula write.
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

    /// Plays a note on track 0 and returns that channel's writes.
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
    fn a_note_starts_its_sample_like_an_amiga_player() {
        let instrument = instrument();
        let sample = instrument.sample.clone().unwrap();
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
    fn notes_play_at_their_period() {
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
    fn a_one_shot_sample_repeats_silence_once_it_ends() {
        let mut instrument = instrument();
        instrument.sample = Some(Arc::new(OctamedicSample {
            data: vec![127; 6],
            repeat: None,
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
    fn a_looped_sample_plays_up_to_its_loop_end_then_repeats_the_loop() {
        let mut instrument = instrument();
        let sample = Arc::new(OctamedicSample {
            data: vec![127; 10],
            repeat: Some((2, 6)),
        });
        instrument.sample = Some(sample.clone());
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
    fn the_instruments_finetune_picks_the_period() {
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
    fn output_volume_scales_the_whole_mix_right_away() {
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
        // a note that is already playing gets quieter too
        commands.send(EngineCommand::SetOutputVolume(16)).unwrap();
        let quarter = run(&mut engine, 1)[0];
        assert!(full > 0.0);
        assert!((quarter - full / 4.0).abs() < 1e-6, "{full} {quarter}");
    }

    #[test]
    fn loading_a_song_keeps_the_output_volume() {
        let (mut engine, commands, _) = engine();
        commands.send(EngineCommand::SetOutputVolume(10)).unwrap();
        commands.send(EngineCommand::SetMasterVolume(64)).unwrap();
        run(&mut engine, 1);
        assert_eq!(engine.output_volume, 10);
    }
}
