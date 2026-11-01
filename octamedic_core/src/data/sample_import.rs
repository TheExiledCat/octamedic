use std::sync::Arc;

use crate::data::{
    instrument::{OctamedicInstrument, OctamedicSample},
    instrument_loader::InstrumentLoadError,
};

/// A sample as a format loader read it, before it's made into what Paula plays.
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedSample {
    pub rate: u32,
    /// The file's own bit depth, to show before importing.
    pub bits: u16,
    /// One list per channel, -1.0..1.0.
    pub channels: Vec<Vec<f32>>,
    /// Loop start..end in frames, for formats that store one.
    pub repeat: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SampleChannel {
    Left,
    Right,
    /// All channels averaged together.
    Mixed,
}

/// How a decoded sample gets made into an instrument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportSettings {
    /// Only matters for samples with more than one channel.
    pub channel: SampleChannel,
    /// Converts it to this rate, if that's lower than its own. Never upsamples.
    pub downsample_to: Option<u32>,
    /// Downsamples by picking the nearest sample instead of averaging, which keeps the aliasing.
    pub allow_aliasing: bool,
}

impl Default for ImportSettings {
    fn default() -> Self {
        return Self {
            channel: SampleChannel::Mixed,
            downsample_to: None,
            allow_aliasing: false,
        };
    }
}

impl DecodedSample {
    /// 8-bit mono at the rate the settings ask for, which is all Paula plays.
    pub fn to_instrument(
        &self,
        name: &str,
        settings: ImportSettings,
    ) -> Result<OctamedicInstrument, InstrumentLoadError> {
        let mono = self.pick_channel(settings.channel);
        if mono.is_empty() {
            return Err(InstrumentLoadError::Malformed("No sound in it".to_owned()));
        }
        let (mono, repeat) = match settings.downsample_to {
            Some(rate) if rate < self.rate => (
                downsample(&mono, self.rate, rate, settings.allow_aliasing),
                self.repeat
                    .map(|repeat| scale_repeat(repeat, self.rate, rate)),
            ),
            _ => (mono, self.repeat),
        };
        let mut instrument = OctamedicInstrument::new(name);
        instrument.sample = Some(Arc::new(OctamedicSample {
            data: mono.iter().map(|&sample| to_8_bit(sample)).collect(),
            repeat,
        }));
        return Ok(instrument);
    }

    fn pick_channel(&self, channel: SampleChannel) -> Vec<f32> {
        let Some(left) = self.channels.first() else {
            return Vec::new();
        };
        return match channel {
            SampleChannel::Left => left.clone(),
            SampleChannel::Right => self.channels.last().cloned().unwrap_or_default(),
            SampleChannel::Mixed => (0..left.len())
                .map(|frame| {
                    let sum: f32 = self.channels.iter().map(|channel| channel[frame]).sum();
                    sum / self.channels.len() as f32
                })
                .collect(),
        };
    }
}

fn scale_repeat((start, end): (usize, usize), from: u32, to: u32) -> (usize, usize) {
    let scale = |frame: usize| (frame as u64 * to as u64 / from as u64) as usize;
    return (scale(start), scale(end));
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

// by 128, so 8-bit data comes out as it went in
fn to_8_bit(sample: f32) -> i8 {
    return (sample * 128.0).round().clamp(-128.0, 127.0) as i8;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoded(rate: u32, channels: Vec<Vec<f32>>) -> DecodedSample {
        return DecodedSample {
            rate,
            bits: 16,
            channels,
            repeat: None,
        };
    }

    fn data(sample: &DecodedSample, settings: ImportSettings) -> Vec<i8> {
        let instrument = sample.to_instrument("test", settings).unwrap();
        return instrument.sample.unwrap().data.clone();
    }

    fn with_channel(channel: SampleChannel) -> ImportSettings {
        return ImportSettings {
            channel,
            ..ImportSettings::default()
        };
    }

    fn halved(allow_aliasing: bool) -> ImportSettings {
        return ImportSettings {
            downsample_to: Some(8000),
            allow_aliasing,
            ..ImportSettings::default()
        };
    }

    #[test]
    fn quantized_to_8_bit() {
        let sample = decoded(8000, vec![vec![0.0, 1.0, -1.0, 0.5]]);
        assert_eq!(data(&sample, ImportSettings::default()), [0, 127, -128, 64]);
    }

    #[test]
    fn eight_bit_data_comes_out_unchanged() {
        let bytes: [i8; 3] = [-64, 127, -128];
        let sample = decoded(
            8000,
            vec![bytes.iter().map(|&b| b as f32 / 128.0).collect()],
        );
        assert_eq!(data(&sample, ImportSettings::default()), bytes);
    }

    #[test]
    fn picks_a_channel_or_mixes() {
        let sample = decoded(8000, vec![vec![1.0, 0.0], vec![0.0, -1.0]]);
        assert_eq!(data(&sample, with_channel(SampleChannel::Left)), [127, 0]);
        assert_eq!(data(&sample, with_channel(SampleChannel::Right)), [0, -128]);
        assert_eq!(data(&sample, with_channel(SampleChannel::Mixed)), [64, -64]);
    }

    #[test]
    fn mono_ignores_the_channel() {
        let sample = decoded(8000, vec![vec![1.0]]);
        assert_eq!(data(&sample, with_channel(SampleChannel::Right)), [127]);
    }

    #[test]
    fn averaging_or_aliasing() {
        let sample = decoded(16_000, vec![vec![0.5, -0.5, 0.5, 0.5, 0.0, 0.0]]);
        assert_eq!(data(&sample, halved(false)), [0, 64, 0]);
        assert_eq!(data(&sample, halved(true)), [64, 64, 0]);
    }

    #[test]
    fn averaging_weighs_partly_covered_samples() {
        let samples = [1.0, 0.0, 0.0];
        assert!((average_over(&samples, 0.0, 1.5) - 1.0 / 1.5).abs() < 1e-6);
    }

    #[test]
    fn never_upsamples() {
        let sample = decoded(8000, vec![vec![0.0; 4]]);
        let higher = ImportSettings {
            downsample_to: Some(16_574),
            ..ImportSettings::default()
        };
        assert_eq!(data(&sample, higher).len(), 4);
    }

    #[test]
    fn fractional_steps_keep_the_length() {
        let sample = decoded(44_100, vec![vec![0.0; 44_100]]);
        let to_c3 = ImportSettings {
            downsample_to: Some(16_574),
            ..ImportSettings::default()
        };
        assert_eq!(data(&sample, to_c3).len(), 16_574);
    }

    #[test]
    fn loop_points_follow_the_rate() {
        let mut sample = decoded(16_000, vec![vec![0.0; 100]]);
        sample.repeat = Some((20, 80));
        let instrument = sample.to_instrument("test", halved(false)).unwrap();
        assert_eq!(instrument.sample.unwrap().repeat, Some((10, 40)));
    }

    #[test]
    fn no_sound() {
        let sample = decoded(8000, vec![vec![]]);
        let result = sample.to_instrument("test", ImportSettings::default());
        assert!(matches!(result, Err(InstrumentLoadError::Malformed(_))));
    }
}
