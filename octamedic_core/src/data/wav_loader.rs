use std::{io::Cursor, sync::Arc};

use hound::{SampleFormat, WavReader, WavSpec};

use crate::data::{
    instrument::{OctamedicInstrument, OctamedicSample},
    instrument_loader::{InstrumentLoadError, InstrumentLoader},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WavChannel {
    Left,
    Right,
    /// All channels averaged together.
    Mixed,
}

/// What's in a WAV file, read from its header, to ask about before loading it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits: u16,
}

/// Loads any PCM or float WAV as an 8-bit mono sample, which is all Paula plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavLoader {
    /// Only matters for files with more than one channel.
    pub channel: WavChannel,
    /// Converts it to this rate, if that's lower than the file's. Never upsamples.
    pub downsample_to: Option<u32>,
    /// Downsamples by picking the nearest sample instead of averaging, which keeps the aliasing.
    pub allow_aliasing: bool,
}

impl WavLoader {
    pub fn info(bytes: &[u8]) -> Result<WavInfo, InstrumentLoadError> {
        let spec = reader(bytes)?.spec();
        return Ok(WavInfo {
            sample_rate: spec.sample_rate,
            channels: spec.channels,
            bits: spec.bits_per_sample,
        });
    }
}

impl InstrumentLoader for WavLoader {
    // "RIFF", the chunk's size, then "WAVE"
    fn recognizes(&self, bytes: &[u8]) -> bool {
        return bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE";
    }

    fn load(&self, name: &str, bytes: &[u8]) -> Result<OctamedicInstrument, InstrumentLoadError> {
        let mut reader = reader(bytes)?;
        let spec = reader.spec();
        let samples = read_normalized(&mut reader, spec)?;
        let mono = pick_channel(&samples, spec.channels as usize, self.channel);
        if mono.is_empty() {
            return Err(malformed("No sound in it"));
        }
        let mono = match self.downsample_to {
            Some(rate) if rate < spec.sample_rate => {
                downsample(&mono, spec.sample_rate, rate, self.allow_aliasing)
            }
            _ => mono,
        };
        let mut instrument = OctamedicInstrument::new(name);
        instrument.sample = Some(Arc::new(OctamedicSample {
            data: mono.iter().map(|&sample| to_8_bit(sample)).collect(),
            repeat: None,
        }));
        return Ok(instrument);
    }
}

fn reader(bytes: &[u8]) -> Result<WavReader<Cursor<&[u8]>>, InstrumentLoadError> {
    return WavReader::new(Cursor::new(bytes)).map_err(|_| malformed("WAV file is damaged"));
}

fn malformed(problem: &str) -> InstrumentLoadError {
    return InstrumentLoadError::Malformed(problem.to_owned());
}

// interleaved, -1.0..1.0 whatever the file's format
fn read_normalized(
    reader: &mut WavReader<Cursor<&[u8]>>,
    spec: WavSpec,
) -> Result<Vec<f32>, InstrumentLoadError> {
    let samples: Result<Vec<f32>, hound::Error> = match spec.sample_format {
        SampleFormat::Float => reader.samples::<f32>().collect(),
        SampleFormat::Int => {
            let full_scale = (1u64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|sample| sample.map(|sample| sample as f32 / full_scale))
                .collect()
        }
    };
    return samples.map_err(|_| malformed("WAV file is damaged"));
}

fn pick_channel(samples: &[f32], channels: usize, channel: WavChannel) -> Vec<f32> {
    let channels = channels.max(1);
    let frames = samples.chunks_exact(channels);
    return match channel {
        WavChannel::Left => frames.map(|frame| frame[0]).collect(),
        WavChannel::Right => frames.map(|frame| frame[1.min(channels - 1)]).collect(),
        WavChannel::Mixed => frames
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect(),
    };
}

fn downsample(samples: &[f32], from: u32, to: u32, allow_aliasing: bool) -> Vec<f32> {
    let step = from as f64 / to as f64;
    let length = (samples.len() as f64 / step).floor() as usize;
    return (0..length)
        .map(|index| {
            let start = index as f64 * step;
            if allow_aliasing {
                nearest(samples, start)
            } else {
                average_over(samples, start, start + step)
            }
        })
        .collect();
}

// the old samples the new one covers, weighted by how much of each it covers, so the high end
// doesn't fold back down
fn average_over(samples: &[f32], start: f64, end: f64) -> f32 {
    let mut sum = 0.0;
    let mut position = start;
    while position < end {
        let index = position as usize;
        let next = ((index + 1) as f64).min(end);
        let sample = samples.get(index).copied().unwrap_or(0.0);
        sum += sample as f64 * (next - position);
        position = next;
    }
    return (sum / (end - start)) as f32;
}

// the old sample at the new one's start, the rest are dropped
fn nearest(samples: &[f32], position: f64) -> f32 {
    return samples.get(position as usize).copied().unwrap_or(0.0);
}

// by 128, so an 8-bit file's bytes come out as they went in
fn to_8_bit(sample: f32) -> i8 {
    return (sample * 128.0).round().clamp(-128.0, 127.0) as i8;
}

#[cfg(test)]
mod tests {
    use hound::WavWriter;

    use super::*;

    fn wav_16_bit(channels: u16, sample_rate: u32, frames: &[&[i16]]) -> Vec<u8> {
        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };
        let mut bytes = Cursor::new(Vec::new());
        let mut writer = WavWriter::new(&mut bytes, spec).unwrap();
        for frame in frames {
            for &sample in *frame {
                writer.write_sample(sample).unwrap();
            }
        }
        writer.finalize().unwrap();
        return bytes.into_inner();
    }

    fn data(loader: WavLoader, bytes: &[u8]) -> Vec<i8> {
        let instrument = loader.load("test", bytes).unwrap();
        return instrument.sample.unwrap().data.clone();
    }

    const KEEP_MIXED: WavLoader = WavLoader {
        channel: WavChannel::Mixed,
        downsample_to: None,
        allow_aliasing: false,
    };

    #[test]
    fn info_from_the_header() {
        let bytes = wav_16_bit(2, 44_100, &[&[0, 0]]);
        let info = WavLoader::info(&bytes).unwrap();
        assert_eq!(
            info,
            WavInfo {
                sample_rate: 44_100,
                channels: 2,
                bits: 16
            }
        );
    }

    #[test]
    fn quantized_to_8_bit() {
        let bytes = wav_16_bit(1, 8000, &[&[0], &[i16::MAX], &[i16::MIN], &[16384]]);
        assert_eq!(data(KEEP_MIXED, &bytes), [0, 127, -128, 64]);
    }

    #[test]
    fn picks_a_channel_or_mixes() {
        let bytes = wav_16_bit(2, 8000, &[&[i16::MAX, 0], &[0, i16::MIN]]);
        let with = |channel| WavLoader {
            channel,
            downsample_to: None,
            allow_aliasing: false,
        };
        assert_eq!(data(with(WavChannel::Left), &bytes), [127, 0]);
        assert_eq!(data(with(WavChannel::Right), &bytes), [0, -128]);
        assert_eq!(data(with(WavChannel::Mixed), &bytes), [64, -64]);
    }

    #[test]
    fn mono_ignores_the_channel() {
        let bytes = wav_16_bit(1, 8000, &[&[i16::MAX]]);
        let right = WavLoader {
            channel: WavChannel::Right,
            downsample_to: None,
            allow_aliasing: false,
        };
        assert_eq!(data(right, &bytes), [127]);
    }

    #[test]
    fn averaging_smooths_the_high_end_out() {
        let samples = [0.5, -0.5, 0.5, 0.5, 0.0, 0.0];
        let averaged: Vec<f32> = (0..3)
            .map(|index| average_over(&samples, index as f64 * 2.0, index as f64 * 2.0 + 2.0))
            .collect();
        assert_eq!(averaged, [0.0, 0.5, 0.0]);
    }

    #[test]
    fn nearest_keeps_a_sample_and_drops_the_rest() {
        let samples = [0.5, -0.5, 0.25, 0.75, 0.0, 1.0];
        let picked: Vec<f32> = (0..3)
            .map(|index| nearest(&samples, index as f64 * 2.0))
            .collect();
        assert_eq!(picked, [0.5, 0.25, 0.0]);
    }

    #[test]
    fn averaging_weighs_partly_covered_samples() {
        let samples = [1.0, 0.0, 0.0];
        // a step of 1.5: all of the first sample and half of the second
        assert!((average_over(&samples, 0.0, 1.5) - 1.0 / 1.5).abs() < 1e-6);
    }

    #[test]
    fn downsampling_halves_the_length() {
        let frames: Vec<[i16; 1]> = vec![[0]; 6];
        let frames: Vec<&[i16]> = frames.iter().map(|frame| &frame[..]).collect();
        let bytes = wav_16_bit(1, 16_000, &frames);
        let halved = WavLoader {
            channel: WavChannel::Mixed,
            downsample_to: Some(8000),
            allow_aliasing: false,
        };
        assert_eq!(data(halved, &bytes).len(), 3);
    }

    #[test]
    fn allow_aliasing_picks_samples() {
        let frames: Vec<[i16; 1]> = [16384, -16384, 16384, 16384, 0, 0]
            .iter()
            .map(|&sample| [sample])
            .collect();
        let frames: Vec<&[i16]> = frames.iter().map(|frame| &frame[..]).collect();
        let bytes = wav_16_bit(1, 16_000, &frames);
        let halved = |allow_aliasing| WavLoader {
            channel: WavChannel::Mixed,
            downsample_to: Some(8000),
            allow_aliasing,
        };
        assert_eq!(data(halved(false), &bytes), [0, 64, 0]);
        assert_eq!(data(halved(true), &bytes), [64, 64, 0]);
    }

    #[test]
    fn never_upsamples() {
        let bytes = wav_16_bit(1, 8000, &[&[0], &[0], &[0], &[0]]);
        let higher = WavLoader {
            channel: WavChannel::Mixed,
            downsample_to: Some(16_574),
            allow_aliasing: false,
        };
        assert_eq!(data(higher, &bytes).len(), 4);
    }

    #[test]
    fn fractional_steps_keep_the_length() {
        let frames: Vec<[i16; 1]> = vec![[0]; 44_100];
        let frames: Vec<&[i16]> = frames.iter().map(|frame| &frame[..]).collect();
        let bytes = wav_16_bit(1, 44_100, &frames);
        let to_c3 = WavLoader {
            channel: WavChannel::Mixed,
            downsample_to: Some(16_574),
            allow_aliasing: false,
        };
        assert_eq!(data(to_c3, &bytes).len(), 16_574);
    }

    #[test]
    fn recognized_by_its_header() {
        assert!(KEEP_MIXED.recognizes(&wav_16_bit(1, 8000, &[&[0]])));
        assert!(!KEEP_MIXED.recognizes(b"RIFF\0\0\0\0AVI LIST"));
        assert!(!KEEP_MIXED.recognizes(b"FORM\0\0\0\08SVX"));
        assert!(!KEEP_MIXED.recognizes(b"RIFF"));
    }

    #[test]
    fn not_a_wav() {
        let result = KEEP_MIXED.load("test", b"definitely not a wav file");
        assert!(matches!(result, Err(InstrumentLoadError::Malformed(_))));
    }

    #[test]
    fn empty_wav() {
        let bytes = wav_16_bit(1, 8000, &[]);
        let result = KEEP_MIXED.load("test", &bytes);
        assert!(matches!(result, Err(InstrumentLoadError::Malformed(_))));
    }

    #[test]
    fn float_and_8_bit_files() {
        let float = WavSpec {
            channels: 1,
            sample_rate: 8000,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };
        let mut bytes = Cursor::new(Vec::new());
        let mut writer = WavWriter::new(&mut bytes, float).unwrap();
        writer.write_sample(0.5f32).unwrap();
        writer.finalize().unwrap();
        assert_eq!(data(KEEP_MIXED, &bytes.into_inner()), [64]);

        let eight_bit = WavSpec {
            bits_per_sample: 8,
            sample_format: SampleFormat::Int,
            ..float
        };
        let mut bytes = Cursor::new(Vec::new());
        let mut writer = WavWriter::new(&mut bytes, eight_bit).unwrap();
        for sample in [-64i8, 127, -128] {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        assert_eq!(data(KEEP_MIXED, &bytes.into_inner()), [-64, 127, -128]);
    }
}
