use std::{
    fmt::Display,
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicUsize, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
};

use cpal::{
    Device, FromSample, SampleFormat, SizedSample, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use super::engine::{OctamedicEngine, PlayMode};
use crate::data::{
    instrument::{InstrumentId, OctamedicInstrument},
    note::OctamedicNote,
    pattern::{OctamedicPattern, PatternId},
    song::OctamedicSong,
    tempo::OctamedicTempo,
};

pub enum EngineCommand {
    PlayPattern {
        pattern: PatternId,
        looping: bool,
    },
    PlaySong {
        sequence_start: usize,
        looping: bool,
    },
    Stop,
    SetPattern {
        id: PatternId,
        pattern: OctamedicPattern,
    },
    SetSequence {
        sequence: Vec<PatternId>,
    },
    SetInstrument {
        id: InstrumentId,
        instrument: Option<OctamedicInstrument>,
    },
    SetTempo(OctamedicTempo),
    SetTrackVolume {
        track: usize,
        volume: u8,
    },
    SetMasterVolume(u8),
    SetGlobalTranspose(i8),
    PlayNote {
        track: usize,
        note: OctamedicNote,
        instrument: InstrumentId,
    },
    ReleaseNote {
        track: usize,
    },
}

impl EngineCommand {
    /// The commands that give the engine everything it needs to play `song`.

    pub fn load_song(song: &OctamedicSong) -> Vec<Self> {

        let mut commands = vec![
            EngineCommand::SetTempo(song.tempo),
            EngineCommand::SetMasterVolume(song.master_volume.get()),
            EngineCommand::SetGlobalTranspose(song.global_transpose.0),
        ];

        for (track, volume) in song.track_volumes.iter().enumerate() {

            commands.push(EngineCommand::SetTrackVolume {
                track,
                volume: volume.get(),
            });
        }

        for (index, instrument) in song.instruments.iter().enumerate() {

            commands.push(EngineCommand::SetInstrument {
                id: InstrumentId(index as u8 + 1),
                instrument: instrument.clone(),
            });
        }

        for (index, pattern) in song.patterns.iter().enumerate() {

            commands.push(EngineCommand::SetPattern {
                id: PatternId(index as u8),
                pattern: pattern.clone(),
            });
        }

        commands.push(EngineCommand::SetSequence {
            sequence: song
                .sequence
                .iter()
                .map(|&id| PatternId::from(id))
                .collect(),
        });

        return commands;
    }
}

#[derive(Default)]

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

#[derive(Debug)]

pub enum EngineError {
    NoOutputDevice,
    UnsupportedSampleFormat(SampleFormat),
    Audio(String),
    Thread(std::io::Error),
}

impl Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        return match self {
            EngineError::NoOutputDevice => write!(f, "no audio output device available"),
            EngineError::UnsupportedSampleFormat(format) => {

                write!(f, "unsupported output sample format {format}")
            }
            EngineError::Audio(error) => write!(f, "audio error: {error}"),
            EngineError::Thread(error) => write!(f, "could not start the engine thread: {error}"),
        };
    }
}

impl std::error::Error for EngineError {}

impl EngineError {
    fn audio(error: impl Display) -> Self {

        return Self::Audio(error.to_string());
    }
}

/// The UI's side of the engine. The engine itself runs inside the audio callback; the engine
/// thread owns the output stream and keeps it alive until the handle shuts down.

pub struct EngineHandle {
    commands: Sender<EngineCommand>,
    shutdown: Option<Sender<()>>,
    status: Arc<EngineStatus>,
    thread: Option<JoinHandle<()>>,
}

impl EngineHandle {
    /// Opens the default output device and starts the engine on it.

    pub fn spawn() -> Result<Self, EngineError> {

        let (commands, receiver) = mpsc::channel();

        let (shutdown, shutdown_receiver) = mpsc::channel::<()>();

        let (ready, ready_receiver) = mpsc::channel();

        let status = Arc::new(EngineStatus::default());

        let engine_status = status.clone();

        // The stream is built on the thread that owns it, since cpal streams are not `Send` on
        // every platform.
        let thread = thread::Builder::new()
            .name("octamedic-engine".into())
            .spawn(move || {

                let stream = match open_stream(receiver, engine_status) {
                    Ok(stream) => stream,
                    Err(error) => {

                        let _ = ready.send(Err(error));

                        return;
                    }
                };

                let _ = ready.send(Ok(()));

                // Returns once the handle drops its shutdown sender.
                let _ = shutdown_receiver.recv();

                drop(stream);
            })
            .map_err(EngineError::Thread)?;

        let started = ready_receiver
            .recv()
            .unwrap_or_else(|_| Err(EngineError::audio("engine thread exited")));

        if let Err(error) = started {

            let _ = thread.join();

            return Err(error);
        }

        return Ok(Self {
            commands,
            shutdown: Some(shutdown),
            status,
            thread: Some(thread),
        });
    }

    pub fn status(&self) -> &EngineStatus {

        return &self.status;
    }

    /// Sends everything the engine needs to play `song`.

    pub fn load_song(&self, song: &OctamedicSong) {

        EngineCommand::load_song(song)
            .into_iter()
            .for_each(|command| self.send(command));
    }

    pub fn play_pattern(&self, pattern: PatternId, looping: bool) {

        self.send(EngineCommand::PlayPattern {
            pattern,
            looping,
        });
    }

    pub fn play_song(&self, sequence_start: usize, looping: bool) {

        self.send(EngineCommand::PlaySong {
            sequence_start,
            looping,
        });
    }

    pub fn stop(&self) {

        self.send(EngineCommand::Stop);
    }

    pub fn set_pattern(&self, id: PatternId, pattern: OctamedicPattern) {

        self.send(EngineCommand::SetPattern {
            id,
            pattern,
        });
    }

    pub fn set_sequence(&self, sequence: Vec<PatternId>) {

        self.send(EngineCommand::SetSequence {
            sequence,
        });
    }

    pub fn set_instrument(&self, id: InstrumentId, instrument: Option<OctamedicInstrument>) {

        self.send(EngineCommand::SetInstrument {
            id,
            instrument,
        });
    }

    pub fn set_tempo(&self, tempo: OctamedicTempo) {

        self.send(EngineCommand::SetTempo(tempo));
    }

    pub fn set_track_volume(&self, track: usize, volume: u8) {

        self.send(EngineCommand::SetTrackVolume {
            track,
            volume,
        });
    }

    pub fn play_note(&self, track: usize, note: OctamedicNote, instrument: InstrumentId) {

        self.send(EngineCommand::PlayNote {
            track,
            note,
            instrument,
        });
    }

    pub fn release_note(&self, track: usize) {

        self.send(EngineCommand::ReleaseNote {
            track,
        });
    }

    fn send(&self, command: EngineCommand) {

        let _ = self.commands.send(command);
    }

    pub fn shut_down(&mut self) {

        self.shutdown.take();

        if let Some(thread) = self.thread.take() {

            let _ = thread.join();
        }
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {

        self.shut_down();
    }
}

/// Builds and starts the output stream on the default device, with the engine inside its callback.

fn open_stream(
    commands: Receiver<EngineCommand>,
    status: Arc<EngineStatus>,
) -> Result<Stream, EngineError> {

    let device = cpal::default_host()
        .default_output_device()
        .ok_or(EngineError::NoOutputDevice)?;

    let config = device.default_output_config().map_err(EngineError::audio)?;

    let engine = OctamedicEngine::new(commands, status, config.sample_rate());

    let stream_config = config.config();

    let stream = match config.sample_format() {
        SampleFormat::F32 => build_stream::<f32>(&device, &stream_config, engine),
        SampleFormat::I16 => build_stream::<i16>(&device, &stream_config, engine),
        SampleFormat::U16 => build_stream::<u16>(&device, &stream_config, engine),
        SampleFormat::I32 => build_stream::<i32>(&device, &stream_config, engine),
        format => Err(EngineError::UnsupportedSampleFormat(format)),
    }?;

    stream.play().map_err(EngineError::audio)?;

    return Ok(stream);
}

/// Builds an output stream of `T` samples; the engine mixes into `f32`, converted on the way out.

fn build_stream<T>(
    device: &Device,
    config: &StreamConfig,
    mut engine: OctamedicEngine,
) -> Result<Stream, EngineError>
where
    T: SizedSample + FromSample<f32>,
{

    let channels = config.channels as usize;

    let mut mix: Vec<f32> = Vec::new();

    return device
        .build_output_stream::<T, _, _>(
            config,
            move |data: &mut [T], _| {

                mix.resize(data.len(), 0.0);

                engine.process(&mut mix, channels);

                for (out, &sample) in data.iter_mut().zip(&mix) {

                    *out = T::from_sample(sample);
                }
            },
            |error| eprintln!("audio error: {error}"),
            None,
        )
        .map_err(EngineError::audio);
}
