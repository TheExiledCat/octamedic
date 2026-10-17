use std::sync::{Arc, mpsc::Receiver};

use octamed::utility::period::AmigaPalPeriod;

use super::{
    handle::{EngineCommand, EngineStatus},
    voice::Voices,
};
use crate::data::{
    instrument::{InstrumentId, MAX_INSTRUMENT_VOLUME, OctamedicInstrument},
    note::OctamedicNote,
    pattern::{MAX_TRACKS, OctamedicPattern, OctamedicPatternTrack, PatternId},
    tempo::OctamedicTempo,
};

/// OctaMED note number of C-2, which plays a sample at Amiga period 428 (about 8287 Hz).

const C2_NOTE: i32 = 13;

const C2_PERIOD: u16 = 428;

/// Highest note number a pattern can hold; anything above is not a note.

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
    /// Converts the stored atomic value back into a mode.

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

#[derive(Clone, Copy)]

struct Position {
    sequence_index: usize,
    pattern: PatternId,
    row: usize,
}

/// Playback timing, counted in output frames so ticks land sample accurately.

struct Clock {
    /// The tick within the current row; 0 is the tick the row's notes play on.
    tick: u32,
    /// Output frames left until the next tick, keeping the fraction so the tempo doesn't drift.
    frames_until_tick: f64,
}

/// Plays the song. Lives inside the audio callback, which drives it through `process`.

pub struct OctamedicEngine {
    commands: Receiver<EngineCommand>,
    status: Arc<EngineStatus>,
    sample_rate: u32,

    patterns: Vec<OctamedicPattern>,
    sequence: Vec<PatternId>,
    instruments: Vec<Option<OctamedicInstrument>>,
    tempo: OctamedicTempo,
    track_volumes: [u8; MAX_TRACKS],
    master_volume: u8,
    global_transpose: i8,
    // the listener's volume, applied to the whole mix; not part of the song
    output_volume: u8,

    mode: PlayMode,
    position: Position,
    clock: Option<Clock>,
    voices: Voices,
    /// The last instrument set on each track, used by notes without an instrument number.
    track_instruments: [Option<InstrumentId>; MAX_TRACKS],
}

impl OctamedicEngine {
    pub fn new(
        commands: Receiver<EngineCommand>,
        status: Arc<EngineStatus>,
        sample_rate: u32,
    ) -> Self {

        return Self {
            commands,
            status,
            sample_rate,
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
            voices: Voices::new(),
            track_instruments: [None; MAX_TRACKS],
        };
    }

    /// Fills `out`, an interleaved buffer with `channels` channels, with the next stretch of
    /// audio. Commands from the UI are applied first, then ticks are run as the frames they fall
    /// on are reached.

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

            self.voices.render(
                &mut out[frame * channels..(frame + frames) * channels],
                channels,
                self.sample_rate,
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

    /// Handles one command sent from the UI.

    fn apply(&mut self, command: EngineCommand) {

        match command {
            EngineCommand::PlayPattern {
                pattern,
                looping,
            } => self.play_pattern(pattern, looping),
            EngineCommand::PlaySong {
                sequence_start,
                looping,
            } => self.play_song(sequence_start, looping),
            EngineCommand::Stop => self.stop(),
            EngineCommand::SetPattern {
                id,
                pattern,
            } => self.set_pattern(id, pattern),
            EngineCommand::SetSequence {
                sequence,
            } => self.sequence = sequence,
            EngineCommand::SetInstrument {
                id,
                instrument,
            } => self.set_instrument(id, instrument),
            EngineCommand::SetTempo(tempo) => self.tempo = tempo,
            EngineCommand::SetTrackVolume {
                track,
                volume,
            } => {
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
            EngineCommand::ReleaseNote {
                track,
            } => self.voices.release(track),
        }
    }

    /// Stores the engine's copy of a pattern.

    fn set_pattern(&mut self, id: PatternId, pattern: OctamedicPattern) {

        if id.0 as usize >= self.patterns.len() {

            self.patterns
                .resize_with(id.0 as usize + 1, OctamedicPattern::new);
        }

        self.patterns[id.0 as usize] = pattern;
    }

    /// Stores the engine's copy of an instrument; `None` clears the slot.

    fn set_instrument(&mut self, id: InstrumentId, instrument: Option<OctamedicInstrument>) {

        let Some(index) = id.index() else {

            return;
        };

        if index >= self.instruments.len() {

            self.instruments.resize_with(index + 1, || None);
        }

        self.instruments[index] = instrument;
    }

    /// Plays a single pattern from row 0, once or on repeat.

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

    /// Plays the sequence starting at the given index.

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

    /// Resets voices and the clock; the first row plays on the next tick, which is due straight
    /// away.

    fn start(&mut self, mode: PlayMode) {

        if self.current_pattern().is_none() {

            self.stop();

            return;
        }

        self.voices.release_all();

        self.track_instruments = [None; MAX_TRACKS];

        self.clock = Some(Clock {
            tick: 0,
            frames_until_tick: 0.0,
        });

        self.mode = mode;

        self.status.set_play_mode(mode);

        self.publish_position();
    }

    /// Stops playback and silences every track.

    fn stop(&mut self) {

        self.clock = None;

        self.mode = PlayMode::Paused;

        self.status.set_play_mode(PlayMode::Paused);

        self.voices.release_all();
    }

    /// The pattern at the current play position.

    fn current_pattern(&self) -> Option<&OctamedicPattern> {

        return self.patterns.get(self.position.pattern.0 as usize);
    }

    /// The song's ticks per line, unless a tempo command has changed the speed.

    fn ticks_per_line(&self) -> u32 {

        /* return self
        .effect_command_handler
        .speed()
        .unwrap_or(self.tempo.ticks_per_line)
        .max(1); */

        return self.tempo.ticks_per_line.0.max(1) as u32;
    }

    /// How many output frames one tick lasts at the current tempo.

    fn frames_per_tick(&self) -> f64 {

        // when tempo commands get added, they change the tempo used here.
        let tick_rate = self.tempo.get_tick_rate().as_hertz().max(1.0) as f64;

        return self.sample_rate as f64 / tick_rate;
    }

    /// One tick: plays the row on its first tick, and moves to the next row after the last.

    fn tick(&mut self) {

        let frames_per_tick = self.frames_per_tick();

        let ticks_per_line = self.ticks_per_line();

        let Some(clock) = &mut self.clock else {

            return;
        };

        clock.frames_until_tick += frames_per_tick;

        let tick = clock.tick;

        clock.tick += 1;

        let row_done = clock.tick >= ticks_per_line;

        if row_done {

            clock.tick = 0;
        }

        if tick == 0 {

            self.play_row();
        }

        // todo effects: run the tick's effects here.

        if row_done {

            self.advance_row();
        }
    }

    /// Moves the position forward a row, stopping when the song ends.

    fn advance_row(&mut self) {

        match self.next_position(&self.position) {
            Some(position) => self.position = position,
            None => self.stop(),
        }
    }

    /// The row played after `position`, following the sequence; `None` when the song ends.

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

    /// Shares the play position with the UI.

    fn publish_position(&self) {

        self.status.set_position(
            self.position.sequence_index,
            self.position.pattern.0 as usize,
            self.position.row,
        );
    }

    /// The pattern data of one track at the current row.

    fn current_track(&self, track: usize) -> Option<OctamedicPatternTrack> {

        return self
            .current_pattern()?
            .lines
            .get(self.position.row)?
            .tracks
            .get(track)
            .copied();
    }

    /// Starts the notes of the row at the current position.

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

    /// Plays `note` on the track with `instrument`, or the track's last instrument when `None`.

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

            self.voices.release(track);

            return;
        };

        let Some(sample) = &instrument.sample else {

            self.voices.release(track);

            return;
        };

        let note = note.value as i32 + instrument.transpose as i32 + self.global_transpose as i32;

        let c2_rate = AmigaPalPeriod::new(C2_PERIOD).get_frequency().as_hertz() as f64;

        let rate = c2_rate * 2f64.powf((note - C2_NOTE) as f64 / 12.0);

        let max = MAX_INSTRUMENT_VOLUME as u32;

        let volume = instrument.volume as u32 * self.track_volumes[track] as u32 / max
            * self.master_volume as u32
            / max;

        self.voices
            .note_on(track, sample.clone(), rate, volume as u8);
    }
}

#[cfg(test)]

mod tests {

    use std::sync::mpsc::{self, Sender};

    use octamed::utility::bytes::UByte;

    use super::*;
    use crate::data::instrument::OctamedicSample;

    const SAMPLE_RATE: u32 = 1000;

    fn engine() -> (OctamedicEngine, Sender<EngineCommand>, Arc<EngineStatus>) {

        let (sender, receiver) = mpsc::channel();

        let status = Arc::new(EngineStatus::default());

        let engine = OctamedicEngine::new(receiver, status.clone(), SAMPLE_RATE);

        return (engine, sender, status);
    }

    /// A tempo of 10 ticks a second, so a tick lasts 100 frames at `SAMPLE_RATE`.

    fn tempo(ticks_per_line: u8) -> OctamedicTempo {

        let mut tempo = OctamedicTempo::new();

        tempo.primary_tempo.0 = 25;

        tempo.lines_per_beat = UByte(4);

        tempo.ticks_per_line = UByte(ticks_per_line);

        return tempo;
    }

    fn pattern(lines: usize, notes: &[(usize, u8)]) -> OctamedicPattern {

        let mut pattern = OctamedicPattern::new();

        pattern.lines.truncate(lines);

        for &(line, note) in notes {

            let track = &mut pattern.lines[line].tracks[0];

            track.note = OctamedicNote {
                value: note,
            };

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
                pattern: pattern(4, &[]),
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
                pattern: pattern(2, &[(1, 13)]),
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
                    pattern: pattern(2, &[]),
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

    #[test]

    fn c2_plays_at_the_amiga_c2_rate() {

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
                note: OctamedicNote {
                    value: 25,
                },
                instrument: InstrumentId(1),
            })
            .unwrap();

        run(&mut engine, 1);

        let rate = engine.voices.rate(0).unwrap();

        assert!((rate - 2.0 * 8287.1369).abs() < 0.1, "{rate}");
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
                note: OctamedicNote {
                    value: 13,
                },
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
