use std::{
    collections::VecDeque,
    fmt::Display,
    sync::{
        Arc, Mutex, MutexGuard,
        mpsc::{self, Sender},
    },
    thread::{self, JoinHandle},
};

use cpal::{
    Device, FromSample, SampleFormat, SizedSample, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use crate::{import::sample::SampleChannel, passthrough::Passthrough};

// what waits for `take` before the oldest goes, a few seconds at the usual rates
const MOST_WAITING: usize = 1 << 18;

type Waiting = Arc<Mutex<VecDeque<f32>>>;

/// How what comes in is turned into what the sample editor hears.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InputSettings {
    pub channel: SampleChannel,
    /// Times the level, clipped at full scale.
    pub gain: f32,
}

impl InputSettings {
    /// `gain_db` decibels up, or down if it's negative.
    pub fn new(channel: SampleChannel, gain_db: f32) -> Self {
        return Self {
            channel,
            gain: 10f32.powf(gain_db / 20.0),
        };
    }

    fn level<T>(&self, frame: &[T]) -> f32
    where
        T: SizedSample,
        f32: FromSample<T>,
    {
        return (mono(frame, self.channel) * self.gain).clamp(-1.0, 1.0);
    }
}

// where the stream puts what comes in
#[derive(Clone)]
struct Destination {
    settings: InputSettings,
    waiting: Waiting,
    // to be heard through the engine as well
    passthrough: Option<Passthrough>,
}

/// Sound coming in from an input device, as one channel, for the sample editor. Nothing else
/// listens, so it's only open while something records or monitors, and closes when dropped.
pub struct AudioInput {
    waiting: Waiting,
    sample_rate: u32,
    shutdown: Option<Sender<()>>,
    thread: Option<JoinHandle<()>>,
    passthrough: Option<Passthrough>,
}

#[derive(Debug)]
pub enum InputError {
    NoInputDevice,
    UnsupportedSampleFormat(SampleFormat),
    Audio(String),
    Thread(std::io::Error),
}

impl Display for InputError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            InputError::NoInputDevice => write!(f, "no audio input device available"),
            InputError::UnsupportedSampleFormat(format) => {
                write!(f, "unsupported input sample format {format}")
            }
            InputError::Audio(error) => write!(f, "audio error: {error}"),
            InputError::Thread(error) => write!(f, "could not start the input thread: {error}"),
        };
    }
}

impl std::error::Error for InputError {}

impl InputError {
    fn audio(error: impl Display) -> Self {
        return Self::Audio(error.to_string());
    }
}

impl AudioInput {
    /// Listens to the device with the id `device`, or the default one with `None` or when that
    /// device is gone. With a `passthrough` it can be heard too, until it's closed.
    pub fn open(
        device: Option<&str>,
        settings: InputSettings,
        passthrough: Option<Passthrough>,
    ) -> Result<Self, InputError> {
        let device = device.map(str::to_owned);
        let waiting = Waiting::default();
        let destination = Destination {
            settings,
            waiting: waiting.clone(),
            passthrough: passthrough.clone(),
        };
        let (shutdown, shutdown_receiver) = mpsc::channel::<()>();
        let (ready, ready_receiver) = mpsc::channel();
        // cpal streams aren't Send on every platform, so it stays on a thread of its own like
        // the engine's
        let thread = thread::Builder::new()
            .name("octamedic-input".into())
            .spawn(move || {
                let (stream, sample_rate) = match open_stream(device.as_deref(), destination) {
                    Ok(opened) => opened,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                let _ = ready.send(Ok(sample_rate));
                let _ = shutdown_receiver.recv();
                drop(stream);
            })
            .map_err(InputError::Thread)?;
        let started = ready_receiver
            .recv()
            .unwrap_or_else(|_| Err(InputError::audio("input thread exited")));
        let sample_rate = match started {
            Ok(sample_rate) => sample_rate,
            Err(error) => {
                let _ = thread.join();
                return Err(error);
            }
        };
        return Ok(Self {
            waiting,
            sample_rate,
            shutdown: Some(shutdown),
            thread: Some(thread),
            passthrough,
        });
    }

    // no device, nothing comes in
    pub fn disconnected() -> Self {
        return Self {
            waiting: Waiting::default(),
            sample_rate: 0,
            shutdown: None,
            thread: None,
            passthrough: None,
        };
    }

    pub fn sample_rate(&self) -> u32 {
        return self.sample_rate;
    }

    /// Moves what came in since the last call onto the end of `into`, between -1 and 1 as the
    /// device gave it.
    pub fn take(&self, into: &mut Vec<f32>) {
        into.extend(lock(&self.waiting).drain(..));
    }
}

impl Drop for AudioInput {
    fn drop(&mut self) {
        if let Some(passthrough) = &self.passthrough {
            passthrough.stop();
        }
        drop(self.shutdown.take());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn open_stream(
    device: Option<&str>,
    destination: Destination,
) -> Result<(Stream, u32), InputError> {
    let host = cpal::default_host();
    let saved = device
        .and_then(|id| id.parse().ok())
        .and_then(|id| host.device_by_id(&id));
    let device = saved
        .or_else(|| host.default_input_device())
        .ok_or(InputError::NoInputDevice)?;
    let config = device.default_input_config().map_err(InputError::audio)?;
    let stream_config = config.config();
    if let Some(passthrough) = &destination.passthrough {
        passthrough.start(stream_config.sample_rate);
    }
    let stream = match config.sample_format() {
        SampleFormat::F32 => build_stream::<f32>(&device, &stream_config, destination),
        SampleFormat::I16 => build_stream::<i16>(&device, &stream_config, destination),
        SampleFormat::U16 => build_stream::<u16>(&device, &stream_config, destination),
        SampleFormat::I32 => build_stream::<i32>(&device, &stream_config, destination),
        format => Err(InputError::UnsupportedSampleFormat(format)),
    }?;
    stream.play().map_err(InputError::audio)?;
    return Ok((stream, stream_config.sample_rate));
}

fn build_stream<T>(
    device: &Device,
    config: &StreamConfig,
    destination: Destination,
) -> Result<Stream, InputError>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let frame_length = config.channels as usize;
    return device
        .build_input_stream::<T, _, _>(
            config,
            move |data: &[T], _| {
                let settings = destination.settings;
                let mut waiting = lock(&destination.waiting);
                let start = waiting.len();
                waiting.extend(data.chunks(frame_length).map(|frame| settings.level(frame)));
                if let Some(passthrough) = &destination.passthrough {
                    passthrough.push(waiting.range(start..).copied());
                }
                let over = waiting.len().saturating_sub(MOST_WAITING);
                waiting.drain(..over);
            },
            |error| eprintln!("audio input error: {error}"),
            None,
        )
        .map_err(InputError::audio);
}

// like importing a sample: right is the last channel, so a mono device gives the same for all three
fn mono<T>(frame: &[T], channel: SampleChannel) -> f32
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let level = |sample: &T| sample.to_sample::<f32>();
    return match channel {
        SampleChannel::Left => frame.first().map_or(0.0, level),
        SampleChannel::Right => frame.last().map_or(0.0, level),
        SampleChannel::Mixed => frame.iter().map(level).sum::<f32>() / frame.len() as f32,
    };
}

/// What RECORD keeps: the input brought to the sample's rate and made 8-bit, until there's as much
/// as the sample buffer holds.
pub struct Recorder {
    // input samples a recorded one covers
    step: f64,
    // how far into the input it is, and where the next recorded sample ends
    position: f64,
    next: f64,
    sum: f32,
    count: u32,
    last: f32,
    recorded: Vec<i8>,
    length: usize,
}

impl Recorder {
    pub fn new(input_rate: u32, sample_rate: u32, length: usize) -> Self {
        let step = match (input_rate, sample_rate) {
            (0, _) | (_, 0) => 1.0,
            (input, sample) => input as f64 / sample as f64,
        };
        return Self {
            step,
            position: 0.0,
            next: step,
            sum: 0.0,
            count: 0,
            last: 0.0,
            recorded: Vec::with_capacity(length),
            length,
        };
    }

    /// Going down in rate each recorded sample is the average of the ones it covers, which keeps
    /// out most of the aliasing; going up it's held. Gives back whether the buffer's full.
    pub fn record(&mut self, input: &[f32]) -> bool {
        for &sample in input {
            self.sum += sample;
            self.count += 1;
            self.last = sample;
            self.position += 1.0;
            while self.position >= self.next && !self.is_full() {
                let level = match self.count {
                    0 => self.last,
                    count => self.sum / count as f32,
                };
                self.recorded.push(eight_bit(level));
                self.sum = 0.0;
                self.count = 0;
                self.next += self.step;
            }
        }
        return self.is_full();
    }

    pub fn recorded(&self) -> &[i8] {
        return &self.recorded;
    }

    pub fn is_full(&self) -> bool {
        return self.recorded.len() >= self.length;
    }

    pub fn into_recorded(self) -> Vec<i8> {
        return self.recorded;
    }
}

/// As a sample keeps it, with no gain: -1 to 1 is the whole of 8 bits.
pub fn eight_bit(level: f32) -> i8 {
    return (level * 128.0).round().clamp(-128.0, 127.0) as i8;
}

fn lock(waiting: &Waiting) -> MutexGuard<'_, VecDeque<f32>> {
    return waiting
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_scale_is_all_of_8_bits() {
        assert_eq!(eight_bit(0.0), 0);
        assert_eq!(eight_bit(0.5), 64);
        assert_eq!(eight_bit(-0.5), -64);
        assert_eq!(eight_bit(-1.0), -128);
        // full scale, and a little over, as far as 8 bits go
        assert_eq!(eight_bit(1.0), 127);
        assert_eq!(eight_bit(1.5), 127);
    }

    #[test]
    fn half_the_rate_averages_pairs() {
        let mut recorder = Recorder::new(48000, 24000, 3);
        assert!(!recorder.record(&[0.5, 0.0, -0.5, -0.5]));
        assert!(recorder.record(&[1.0, 1.0, 0.25, 0.25]));
        assert_eq!(recorder.into_recorded(), [32, -64, 127]);
    }

    #[test]
    fn stops_when_the_buffer_is_full() {
        let mut recorder = Recorder::new(8000, 8000, 2);
        assert!(recorder.record(&[0.5, 0.5, 0.5, 0.5]));
        assert_eq!(recorder.into_recorded().len(), 2);
    }

    #[test]
    fn a_higher_rate_holds_each_sample() {
        let mut recorder = Recorder::new(8000, 16000, 4);
        assert!(recorder.record(&[0.5, -0.5]));
        assert_eq!(recorder.into_recorded(), [64, 64, -64, -64]);
    }

    #[test]
    fn a_pitch_rate_comes_out_at_its_length() {
        // a second at 48 kHz, recorded at PAL's period 428
        let rate = 3_546_895 / 428;
        let mut recorder = Recorder::new(48000, rate, 131072);
        recorder.record(&vec![0.0; 48000]);
        let recorded = recorder.into_recorded().len() as i64;
        assert!(
            (recorded - rate as i64).abs() <= 1,
            "{recorded} for {rate} Hz"
        );
    }

    #[test]
    fn gain_raises_the_level_and_clips_at_full_scale() {
        let none = InputSettings::new(SampleChannel::Left, 0.0);
        assert_eq!(none.level(&[0.25f32]), 0.25);
        let raised = InputSettings::new(SampleChannel::Left, 20.0);
        assert!((raised.gain - 10.0).abs() < 1e-4);
        assert!((raised.level(&[0.05f32]) - 0.5).abs() < 1e-4);
        assert_eq!(raised.level(&[0.5f32]), 1.0);
        assert_eq!(raised.level(&[-0.5f32]), -1.0);
        // and down, for a hot input
        let lowered = InputSettings::new(SampleChannel::Left, -6.0);
        assert!((lowered.level(&[1.0f32]) - 0.501).abs() < 1e-3);
    }

    #[test]
    fn mixed_averages_the_channels() {
        assert_eq!(mono(&[0.5f32, 0.5], SampleChannel::Mixed), 0.5);
        assert_eq!(mono(&[1.0f32, -1.0], SampleChannel::Mixed), 0.0);
        assert_eq!(mono(&[i16::MIN], SampleChannel::Mixed), -1.0);
    }

    #[test]
    fn one_channel_keeps_its_full_level() {
        // a mic clipping on input 1 of an interface's pair, with nothing on input 2
        assert_eq!(mono(&[1.0f32, 0.0], SampleChannel::Mixed), 0.5);
        assert_eq!(mono(&[1.0f32, 0.0], SampleChannel::Left), 1.0);
        assert_eq!(mono(&[0, i16::MIN], SampleChannel::Right), -1.0);
        // and a mono device is the same whichever's picked
        assert_eq!(mono(&[0.25f32], SampleChannel::Right), 0.25);
    }
}
