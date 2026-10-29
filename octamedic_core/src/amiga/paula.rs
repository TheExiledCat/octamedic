use std::sync::{Arc, LazyLock};

use crate::{amiga::clock::AmigaClock, data::instrument::OctamedicSample};

/// So four tracks at full volume fill the output.
const VOICE_GAIN: f32 = 0.25;

/// An 8-bit sample at volume 64.
const VOICE_FULL_SCALE: f32 = 128.0 * 64.0;

const MAX_VOLUME: u8 = 64;

/// What a channel plays to stay silent with DMA on, like after a one-shot sample.
pub static SILENCE: LazyLock<Arc<OctamedicSample>> = LazyLock::new(|| {
    return Arc::new(OctamedicSample {
        data: vec![0, 0],
        repeat: None,
    });
});

/// A write to a Paula channel register. Logged per tick, these can be compared with the original
/// player's.
#[derive(Clone, Debug, PartialEq)]
pub enum PaulaWrite {
    /// `AUDxPER`
    Period(u16),
    /// `AUDxVOL`, 0-64
    Volume(u8),
    /// `AUDxLC` and `AUDxLEN`, taken when DMA turns on or the playing range ends. The length is
    /// in bytes, not words, so odd-length samples keep their last byte.
    Sample {
        sample: Arc<OctamedicSample>,
        start: usize,
        len: usize,
    },
    /// Starts the range last written.
    DmaOn,
    DmaOff,
}

/// Turns Paula writes into audio.
pub trait Renderer: Send {
    fn write(&mut self, channel: usize, write: PaulaWrite);

    /// Adds to `out` (interleaved, `channels` per frame).
    fn render(&mut self, out: &mut [f32], channels: usize);
}

#[derive(Clone)]
struct SampleRange {
    sample: Arc<OctamedicSample>,
    start: usize,
    len: usize,
}

impl SampleRange {
    fn end(&self) -> usize {
        return self.start + self.len;
    }
}

struct Playing {
    range: SampleRange,
    /// In bytes, with the fraction played.
    position: f64,
}

#[derive(Default)]
struct Channel {
    period: u16,
    volume: u8,
    /// Taken when the next range starts.
    latch: Option<SampleRange>,
    /// `None` while DMA is off.
    playing: Option<Playing>,
}

impl Channel {
    fn write(&mut self, write: PaulaWrite) {
        match write {
            PaulaWrite::Period(period) => self.period = period,
            PaulaWrite::Volume(volume) => self.volume = volume.min(MAX_VOLUME),
            PaulaWrite::Sample { sample, start, len } => {
                self.latch = Some(SampleRange { sample, start, len })
            }
            PaulaWrite::DmaOn => {
                if self.playing.is_none() {
                    self.playing = self.latch.clone().map(|range| Playing {
                        position: range.start as f64,
                        range,
                    });
                }
            }
            PaulaWrite::DmaOff => self.playing = None,
        }
    }

    /// The output over the next `step` bytes (one output frame). Each byte counts for the part
    /// of the frame it's held over, so bytes that don't line up with frames don't alias. When a
    /// range ends the latched one starts, which is how samples loop.
    fn next(&mut self, step: f64) -> Option<f32> {
        let Channel {
            volume,
            latch,
            playing,
            ..
        } = self;
        let playing = playing.as_mut()?;
        let mut remaining = step;
        let mut sum = 0.0;
        while remaining > 0.0 {
            if playing.range.len == 0 {
                break;
            }
            let index = playing.position as usize;
            // chip memory past the sample, read as zero
            let byte = playing.range.sample.data.get(index).copied().unwrap_or(0);
            let next_byte = (index + 1) as f64;
            let held = (next_byte - playing.position).min(remaining);
            sum += byte as f64 * held;
            remaining -= held;
            // snap to the next byte, so float error can't leave a sliver of this one
            playing.position = if held == next_byte - playing.position {
                next_byte
            } else {
                playing.position + held
            };
            if playing.position >= playing.range.end() as f64 {
                let overshoot = playing.position - playing.range.end() as f64;
                if let Some(range) = latch {
                    playing.range = range.clone();
                }
                playing.position = playing.range.start as f64 + overshoot;
            }
        }
        return Some((sum / step) as f32 * *volume as f32);
    }
}

/// Paula's own channels, for songs of up to 4 channels.
pub struct PaulaRenderer {
    sample_rate: u32,
    clock: AmigaClock,
    channels: Vec<Channel>,
}

impl PaulaRenderer {
    pub fn new(sample_rate: u32, clock: AmigaClock, channel_count: usize) -> Self {
        return Self {
            sample_rate,
            clock,
            channels: (0..channel_count).map(|_| Channel::default()).collect(),
        };
    }
}

impl Renderer for PaulaRenderer {
    fn write(&mut self, channel: usize, write: PaulaWrite) {
        if let Some(channel) = self.channels.get_mut(channel) {
            channel.write(write);
        }
    }

    fn render(&mut self, out: &mut [f32], channels: usize) {
        for channel in &mut self.channels {
            if channel.playing.is_none() || channel.period == 0 {
                continue;
            }
            let rate = self.clock.paula_clock as f64 / channel.period as f64;
            let step = rate / self.sample_rate as f64;
            for frame in out.chunks_exact_mut(channels) {
                let Some(value) = channel.next(step) else {
                    break;
                };
                let value = value / VOICE_FULL_SCALE * VOICE_GAIN;
                frame.iter_mut().for_each(|sample| *sample += value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: f32 = 127.0 * 64.0 / VOICE_FULL_SCALE * VOICE_GAIN;

    /// A period of 5 plays one byte per frame.
    const SAMPLE_RATE: u32 = AmigaClock::PAL.paula_clock / 5;

    fn sample(data: Vec<i8>) -> Arc<OctamedicSample> {
        return Arc::new(OctamedicSample { data, repeat: None });
    }

    fn range(sample: &Arc<OctamedicSample>, start: usize, len: usize) -> PaulaWrite {
        return PaulaWrite::Sample {
            sample: sample.clone(),
            start,
            len,
        };
    }

    /// A note on, as `OctamedicEngine::trigger` does it.
    fn play(renderer: &mut PaulaRenderer, first: PaulaWrite, period: u16, then: PaulaWrite) {
        renderer.write(0, PaulaWrite::DmaOff);
        renderer.write(0, first);
        renderer.write(0, PaulaWrite::Period(period));
        renderer.write(0, PaulaWrite::Volume(64));
        renderer.write(0, PaulaWrite::DmaOn);
        renderer.write(0, then);
    }

    fn render(renderer: &mut PaulaRenderer, frames: usize) -> Vec<f32> {
        let mut out = vec![0.0; frames];
        renderer.render(&mut out, 1);
        return out;
    }

    fn assert_near(actual: &[f32], expected: &[f32]) {
        let close = actual.len() == expected.len()
            && actual
                .iter()
                .zip(expected)
                .all(|(a, e)| (a - e).abs() < 1e-6);
        assert!(close, "{actual:?} != {expected:?}");
    }

    fn renderer() -> PaulaRenderer {
        return PaulaRenderer::new(SAMPLE_RATE, AmigaClock::PAL, 4);
    }

    #[test]
    fn a_one_shot_sample_falls_into_silence() {
        let mut renderer = renderer();
        let data = sample(vec![127, 127]);
        play(&mut renderer, range(&data, 0, 2), 5, range(&SILENCE, 0, 2));
        assert_near(&render(&mut renderer, 4), &[FULL, FULL, 0.0, 0.0]);
    }

    #[test]
    fn a_looped_sample_wraps_into_its_loop() {
        let mut renderer = renderer();
        let data = sample(vec![0, 127, -127]);
        play(&mut renderer, range(&data, 0, 3), 5, range(&data, 1, 2));
        assert_near(&render(&mut renderer, 5), &[0.0, FULL, -FULL, FULL, -FULL]);
    }

    #[test]
    fn double_the_period_holds_each_byte_twice_as_long() {
        let mut renderer = renderer();
        let data = sample(vec![0, 127]);
        play(&mut renderer, range(&data, 0, 2), 10, range(&SILENCE, 0, 2));
        assert_near(&render(&mut renderer, 4), &[0.0, 0.0, FULL, FULL]);
    }

    #[test]
    fn a_byte_change_inside_a_frame_is_weighted_by_how_long_each_byte_is_held() {
        let mut renderer = renderer();
        let data = sample(vec![0, 127]);
        // 1.6 output frames per byte, so the bytes change partway through frames
        play(&mut renderer, range(&data, 0, 2), 8, range(&SILENCE, 0, 2));
        let out = render(&mut renderer, 5);
        assert_near(&out, &[0.0, FULL * 0.4, FULL, FULL * 0.2, 0.0]);
    }

    #[test]
    fn faster_than_the_output_rate_averages_the_bytes_in_a_frame() {
        let mut renderer = renderer();
        let data = sample(vec![127, -127, 127, -127, 127]);
        // 2.5 bytes per output frame
        play(&mut renderer, range(&data, 0, 5), 2, range(&SILENCE, 0, 2));
        assert_near(&render(&mut renderer, 2), &[FULL * 0.2, FULL * 0.2]);
    }

    #[test]
    fn a_new_range_waits_until_the_playing_one_ends() {
        let mut renderer = renderer();
        let first = sample(vec![127, 127, 127]);
        let second = sample(vec![-127]);
        play(&mut renderer, range(&first, 0, 3), 5, range(&first, 0, 3));
        render(&mut renderer, 1);
        renderer.write(0, range(&second, 0, 1));
        assert_near(&render(&mut renderer, 4), &[FULL, FULL, -FULL, -FULL]);
    }

    #[test]
    fn dma_on_again_while_playing_changes_nothing() {
        let mut renderer = renderer();
        let data = sample(vec![0, 127, -127]);
        play(&mut renderer, range(&data, 0, 3), 5, range(&data, 0, 3));
        render(&mut renderer, 1);
        renderer.write(0, PaulaWrite::DmaOn);
        assert_near(&render(&mut renderer, 2), &[FULL, -FULL]);
    }

    #[test]
    fn dma_off_silences_the_channel_and_dma_on_restarts_the_latched_range() {
        let mut renderer = renderer();
        let data = sample(vec![127, -127]);
        play(&mut renderer, range(&data, 0, 2), 5, range(&data, 0, 2));
        render(&mut renderer, 1);
        renderer.write(0, PaulaWrite::DmaOff);
        assert_near(&render(&mut renderer, 2), &[0.0, 0.0]);
        renderer.write(0, PaulaWrite::DmaOn);
        assert_near(&render(&mut renderer, 2), &[FULL, -FULL]);
    }

    #[test]
    fn a_new_period_applies_straight_away() {
        let mut renderer = renderer();
        let data = sample(vec![127, -127, 127, -127]);
        play(&mut renderer, range(&data, 0, 4), 5, range(&data, 0, 4));
        render(&mut renderer, 1);
        renderer.write(0, PaulaWrite::Period(10));
        assert_near(&render(&mut renderer, 2), &[-FULL, -FULL]);
    }

    #[test]
    fn volume_scales_like_paula() {
        let mut renderer = renderer();
        let data = sample(vec![127]);
        play(&mut renderer, range(&data, 0, 1), 5, range(&data, 0, 1));
        renderer.write(0, PaulaWrite::Volume(32));
        assert_near(&render(&mut renderer, 1), &[FULL / 2.0]);
        renderer.write(0, PaulaWrite::Volume(200));
        assert_near(&render(&mut renderer, 1), &[FULL]);
    }

    #[test]
    fn every_output_channel_gets_the_voice() {
        let mut renderer = renderer();
        let data = sample(vec![127]);
        play(&mut renderer, range(&data, 0, 1), 5, range(&data, 0, 1));
        let mut out = [0.0; 2];
        renderer.render(&mut out, 2);
        assert_near(&out, &[FULL, FULL]);
    }

    #[test]
    fn writes_to_channels_that_do_not_exist_are_ignored() {
        let mut renderer = renderer();
        renderer.write(9, PaulaWrite::DmaOn);
        assert_near(&render(&mut renderer, 1), &[0.0]);
    }
}
