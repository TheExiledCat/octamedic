use std::{
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicUsize, Ordering},
        mpsc::{Receiver, RecvTimeoutError},
    },
    time::Instant,
};

use crate::{
    data::{
        note::OctamedicNote,
        pattern::{OctamedicPattern, PatternId},
        project::OctamedicProject,
        tempo::{MAX_BPM, OctamedicTempo},
    },
    engine::{engine::PlayMode::Paused, voice::Voices},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]

pub enum PlayMode {
    Paused,
    Song,
    Pattern,
}

impl PlayMode {
    /// Converts the stored atomic value back into a mode.

    pub fn from_u8(value: u8) -> Self {

        return match value {
            1 => PlayMode::Song,
            2 => PlayMode::Pattern,
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

struct Clock {
    next_pulse: Instant,
    row_pulse: u32,
    row_start: Instant,
    tick: u32,
}

pub enum EngineCommand {
    PlayPattern {
        pattern: PatternId,
    },
    PlaySong {
        sequence_start: u8,
    },
    SetPattern {
        id: PatternId,
        pattern: OctamedicPattern,
    },
    SetSequence {
        sequence: Vec<u8>,
    },
    SetTempo(OctamedicTempo),
    PlayNote {
        track: u8,
        pitch: OctamedicNote,
    },
    ReleaseNote {
        track: u8,
    },
}

pub struct EngineStatus {
    play_mode: AtomicU8,
    sequence_index: AtomicUsize,
    pattern: AtomicUsize,
    row: AtomicUsize,
}

impl EngineStatus {
    pub fn play_mode(&self) -> PlayMode {

        return PlayMode::from_u8(self.play_mode.load(Ordering::Relaxed));
    }

    pub fn sequence_index(&self) -> usize {

        return self.sequence_index.load(Ordering::Relaxed);
    }

    pub fn pattern(&self) -> usize {

        return self.pattern.load(Ordering::Relaxed);
    }

    pub fn row(&self) -> usize {

        return self.row.load(Ordering::Relaxed);
    }


    pub fn set_play_mode(&self, mode: PlayMode) {

        self.play_mode.store(mode as u8, Ordering::Relaxed);
    }

    pub fn set_position(&self, sequence_index: usize, pattern: usize, row: usize) {

        self.sequence_index.store(sequence_index, Ordering::Relaxed);

        self.pattern.store(pattern, Ordering::Relaxed);

        self.row.store(row, Ordering::Relaxed);
    }
}

pub struct OctamedicEngine {
    commands: Receiver<EngineCommand>,
    status: Arc<EngineStatus>,
    patterns: Vec<OctamedicPattern>,
    sequence: Vec<u8>,
    tempo: OctamedicTempo,
    mode: PlayMode,
    position: Position,
    clock: Option<Clock>,
    voices: Voices,
}

impl OctamedicEngine {
    pub fn new(commands: Receiver<EngineCommand>, status: Arc<EngineStatus>) -> Self {

        return Self {
            commands,
            status,
            patterns: Vec::new(),
            sequence: Vec::new(),
            tempo: OctamedicTempo::new(),
            mode: Paused,
            position: Position {
                sequence_index: 0,
                pattern: PatternId(0),
                row: 0,
            },
            clock: None,
            voices: Voices::new(),
        };
    }

    fn run(&mut self) {

        loop {

            let received = match self.next_deadline() {
                Some(deadline) => self
                    .commands
                    .recv_timeout(deadline.saturating_duration_since(Instant::now())),
                None => self
                    .commands
                    .recv()
                    .map_err(|_| RecvTimeoutError::Disconnected),
            };

            match received {
                Ok(command) => self.apply(command),
                Err(RecvTimeoutError::Timeout) => self.advance_clock(),
                Err(RecvTimeoutError::Disconnected) => break,
            }

            //  self.dispatch_pending();
        }
    }

    fn apply(&mut self, command: EngineCommand) {

        match command {
            EngineCommand::PlayPattern {
                pattern,
            } => todo!(),
            EngineCommand::PlaySong {
                sequence_start,
            } => todo!(),
            EngineCommand::SetPattern {
                id,
                pattern,
            } => self.set_pattern(id, pattern),
            EngineCommand::SetSequence {
                sequence,
            } => self.sequence = sequence,
            EngineCommand::SetTempo(octamedic_tempo) => self.tempo = octamedic_tempo,
            EngineCommand::PlayNote {
                track,
                pitch,
            } => todo!(),
            EngineCommand::ReleaseNote {
                track,
            } => todo!(),
        }
    }

    /// Stores the engine's copy of a pattern and cuts newly muted notes if it is playing.

    fn set_pattern(&mut self, id: PatternId, pattern: OctamedicPattern) {

        if id.0 as usize >= self.patterns.len() {

            self.patterns
                .resize_with(id.0 as usize + 1, OctamedicPattern::new);
        }

        self.patterns[id.0 as usize] = pattern;

        if self.clock.is_some() && id == self.position.pattern {

            self.release_muted();
        }
    }

    fn play_pattern(&mut self, id: PatternId) {

        self.position = Position {
            sequence_index: 0,
            pattern: id,
            row: 0,
        };

        self.start(PlayMode::Pattern);
    }

    /// Plays the sequence starting at the given index.

    fn play_song(&mut self, sequence_index: usize) {

        let Some(sequence_row) = self.sequence.get(sequence_index) else {

            return;
        };

        self.position = Position {
            sequence_index,
            pattern: PatternId(sequence_row.0),
            row: 0,
        };

        self.start(PlayMode::Song);
    }

    /// Resets voices and the clock, then plays the first row.

    fn start(&mut self, mode: PlayMode) {

        if self.current_pattern().is_none() {

            self.stop();

            return;
        }

        let mut out = vec![];

        self.voices.release_all(&mut out);

        let now = Instant::now();

        self.clock = Some(Clock {
            next_pulse: now,
            row_pulse: 0,
            row_start: now,
            tick: 0,
        });

        self.mode = mode;

        self.status.set_play_mode(mode);

        self.publish_position();


        self.play_row();

        self.time_next_pulse(now);
    }

    /// The pattern at the current play position.

    fn current_pattern(&self) -> Option<&OctamedicPattern> {

        return self.patterns.get(self.position.pattern.0 as usize);
    }

    /// The song's tempo, unless a tempo command has changed it.

    fn bpm(&self) -> u8 {

        // return self.effect_command_handler.bpm().unwrap_or(self.bpm); when commands get added
        return self.tempo.as_bpm().0.min(MAX_BPM as u16) as u8;
    }

    /// The ticks per line, unless a tempo command has changed the speed.

    fn ticks_per_line(&self) -> u8 {

        /* return self
        .effect_command_handler
        .speed()
        .unwrap_or(pattern.ticks_per_line)
        .max(1); */

        return self.tempo.ticks_per_line.0 as u8;
    }

    /// Fill `sample_buffer` with unsigned 8-bit PCM (128 = silence).

    /// Catches up on every pulse and effect tick that is due by now.

    fn advance_clock(&mut self) {

        let now = Instant::now();

        loop {

            let Some(clock) = &self.clock else {

                return;
            };

            let next_pulse = clock.next_pulse;

            //todo effects
            // match self.next_effect_tick() {
            //     Some(tick) if tick <= next_pulse && tick <= now => {

            //         if let Some(clock) = &mut self.clock {

            //             clock.tick += 1;
            //         }

            //         self.effect_tick();
            //     }
            //     _ if next_pulse <= now => self.pulse(),
            //     _ => return,
            // }
        }
    }

    pub fn process(&mut self, sample_buffer: &mut [u8], output_sample_rate: u32) -> bool {

        false
    }
}
