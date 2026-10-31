use std::{
    array,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicI8, AtomicUsize, Ordering},
    },
};

use crate::{amiga::clock::AmigaClock, data::instrument::OctamedicSample};

// 4 tracks at full volume = full scale
const MONO_GAIN: f32 = 0.25;

// 2 tracks a side
const STEREO_GAIN: f32 = 0.5;

const VOICE_FULL_SCALE: f32 = 128.0 * 64.0;

const MAX_VOLUME: u8 = 64;

pub const SCOPE_CHANNELS: usize = 4;

pub const SCOPE_LENGTH: usize = 512;

/// Looped after a one-shot sample, so DMA can stay on.
pub static SILENCE: LazyLock<Arc<OctamedicSample>> = LazyLock::new(|| {
    return Arc::new(OctamedicSample {
        data: vec![0, 0],
        repeat: None,
    });
});

/// Paula channel register writes. Logged per tick so they can be diffed against the original
/// player.
#[derive(Clone, Debug, PartialEq)]
pub enum PaulaWrite {
    Period(u16),
    /// 0-64
    Volume(u8),
    /// `AUDxLC`/`AUDxLEN`, latched until DMA on or the current range ends. len is in bytes, not
    /// words like the real register, so odd-length samples keep their last byte.
    Sample {
        sample: Arc<OctamedicSample>,
        start: usize,
        len: usize,
    },
    DmaOn,
    DmaOff,
}

/// Last `SCOPE_LENGTH` frames of the 4 hardware channels, after volume. No locking, so a read
/// during a render can get a bit of the newest buffer at the front.
pub struct Scopes {
    samples: [[AtomicI8; SCOPE_LENGTH]; SCOPE_CHANNELS],
    next: AtomicUsize,
}

impl Scopes {
    /// Oldest first.
    pub fn read(&self, channel: usize) -> [i8; SCOPE_LENGTH] {
        let next = self.next.load(Ordering::Relaxed);
        let samples = &self.samples[channel];
        return array::from_fn(|i| samples[(next + i) % SCOPE_LENGTH].load(Ordering::Relaxed));
    }

    fn write(&self, channel: usize, at: usize, value: i8) {
        self.samples[channel][at % SCOPE_LENGTH].store(value, Ordering::Relaxed);
    }
}

impl Default for Scopes {
    fn default() -> Self {
        return Self {
            samples: array::from_fn(|_| array::from_fn(|_| AtomicI8::new(0))),
            next: AtomicUsize::new(0),
        };
    }
}

pub trait Renderer: Send {
    fn write(&mut self, channel: usize, write: PaulaWrite);

    /// Adds to `out`, interleaved.
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
    // in bytes, fractional
    position: f64,
}

#[derive(Default)]
struct Channel {
    period: u16,
    volume: u8,
    // next range, picked up when the current one ends
    latch: Option<SampleRange>,
    // None = dma off
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

    // one output frame = `step` bytes. each byte is weighted by how long it's held in the frame,
    // cheap box filter so it doesn't alias as badly. looping is just the latch taking over
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
            // paula reads past the end as zero
            let byte = playing.range.sample.data.get(index).copied().unwrap_or(0);
            let next_byte = (index + 1) as f64;
            let held = (next_byte - playing.position).min(remaining);
            sum += byte as f64 * held;
            remaining -= held;
            // snap, or float error leaves a sliver of the old byte
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

/// One channel per track. The real thing has 4 and OctaMED mixes 5-8 tracks in software, we
/// don't bother with that.
pub struct PaulaRenderer {
    sample_rate: u32,
    clock: AmigaClock,
    channels: Vec<Channel>,
    scopes: Arc<Scopes>,
}

impl PaulaRenderer {
    pub fn new(
        sample_rate: u32,
        clock: AmigaClock,
        channel_count: usize,
        scopes: Arc<Scopes>,
    ) -> Self {
        return Self {
            sample_rate,
            clock,
            channels: (0..channel_count).map(|_| Channel::default()).collect(),
            scopes,
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
        let frames = out.len() / channels;
        let start = self.scopes.next.load(Ordering::Relaxed);
        for (index, channel) in self.channels.iter_mut().enumerate() {
            let scope = (index < SCOPE_CHANNELS).then_some(index);
            if channel.playing.is_none() || channel.period == 0 {
                if let Some(scope) = scope {
                    (0..frames).for_each(|i| self.scopes.write(scope, start + i, 0));
                }
                continue;
            }
            let rate = self.clock.paula_clock as f64 / channel.period as f64;
            let step = rate / self.sample_rate as f64;
            for (i, frame) in out.chunks_exact_mut(channels).enumerate() {
                let value = channel.next(step).unwrap_or(0.0);
                if let Some(scope) = scope {
                    let level = (value / MAX_VOLUME as f32).round() as i8;
                    self.scopes.write(scope, start + i, level);
                }
                mix(frame, index, value / VOICE_FULL_SCALE);
            }
        }
        let next = (start + frames) % SCOPE_LENGTH;
        self.scopes.next.store(next, Ordering::Relaxed);
    }
}

// amiga stereo is LRRL, extra tracks repeat it. anything past 2 output channels stays silent
fn mix(frame: &mut [f32], channel: usize, value: f32) {
    match frame {
        [mono] => *mono += value * MONO_GAIN,
        [left, right, ..] => match channel % 4 {
            0 | 3 => *left += value * STEREO_GAIN,
            _ => *right += value * STEREO_GAIN,
        },
        [] => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FULL: f32 = 127.0 * 64.0 / VOICE_FULL_SCALE * MONO_GAIN;

    // period 5 = one byte per frame
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

    // same order as OctamedicEngine::trigger
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
        return PaulaRenderer::new(SAMPLE_RATE, AmigaClock::PAL, 4, Arc::default());
    }

    #[test]
    fn stereo_lrrl() {
        let mut renderer = renderer();
        let data = sample(vec![127; 4]);
        for channel in 0..4 {
            renderer.write(channel, range(&data, 0, 4));
            renderer.write(channel, PaulaWrite::Period(5));
            renderer.write(channel, PaulaWrite::Volume(64));
            renderer.write(channel, PaulaWrite::DmaOn);
            let mut out = vec![0.0; 2];
            renderer.render(&mut out, 2);
            let side = if matches!(channel, 0 | 3) { 0 } else { 1 };
            assert!(
                out[side] > 0.0 && out[1 - side] == 0.0,
                "{channel}: {out:?}"
            );
            renderer.write(channel, PaulaWrite::DmaOff);
        }
    }

    #[test]
    fn full_side() {
        let mut renderer = renderer();
        let data = sample(vec![127; 4]);
        for channel in [0, 3] {
            renderer.write(channel, range(&data, 0, 4));
            renderer.write(channel, PaulaWrite::Period(5));
            renderer.write(channel, PaulaWrite::Volume(64));
            renderer.write(channel, PaulaWrite::DmaOn);
        }
        let mut out = vec![0.0; 2];
        renderer.render(&mut out, 2);
        assert_near(&out, &[127.0 / 128.0, 0.0]);
    }

    #[test]
    fn scope_order() {
        let scopes = Arc::new(Scopes::default());
        let mut renderer = PaulaRenderer::new(SAMPLE_RATE, AmigaClock::PAL, 4, scopes.clone());
        let data = sample(vec![10, 20, 30, 40]);
        play(&mut renderer, range(&data, 0, 4), 5, range(&data, 0, 4));
        render(&mut renderer, SCOPE_LENGTH + 2);
        let scope = scopes.read(0);
        assert_eq!(scope[..4], [30, 40, 10, 20]);
        assert_eq!(scope[SCOPE_LENGTH - 2..], [10, 20]);
        assert!(scopes.read(1).iter().all(|&sample| sample == 0));
    }

    #[test]
    fn scope_stopped() {
        let scopes = Arc::new(Scopes::default());
        let mut renderer = PaulaRenderer::new(SAMPLE_RATE, AmigaClock::PAL, 4, scopes.clone());
        let data = sample(vec![100; 4]);
        play(&mut renderer, range(&data, 0, 4), 5, range(&data, 0, 4));
        render(&mut renderer, 8);
        renderer.write(0, PaulaWrite::DmaOff);
        render(&mut renderer, SCOPE_LENGTH);
        assert!(scopes.read(0).iter().all(|&sample| sample == 0));
    }

    #[test]
    fn one_shot() {
        let mut renderer = renderer();
        let data = sample(vec![127, 127]);
        play(&mut renderer, range(&data, 0, 2), 5, range(&SILENCE, 0, 2));
        assert_near(&render(&mut renderer, 4), &[FULL, FULL, 0.0, 0.0]);
    }

    #[test]
    fn looped() {
        let mut renderer = renderer();
        let data = sample(vec![0, 127, -127]);
        play(&mut renderer, range(&data, 0, 3), 5, range(&data, 1, 2));
        assert_near(&render(&mut renderer, 5), &[0.0, FULL, -FULL, FULL, -FULL]);
    }

    #[test]
    fn double_period() {
        let mut renderer = renderer();
        let data = sample(vec![0, 127]);
        play(&mut renderer, range(&data, 0, 2), 10, range(&SILENCE, 0, 2));
        assert_near(&render(&mut renderer, 4), &[0.0, 0.0, FULL, FULL]);
    }

    #[test]
    fn byte_weighting() {
        let mut renderer = renderer();
        let data = sample(vec![0, 127]);
        // 1.6 frames per byte
        play(&mut renderer, range(&data, 0, 2), 8, range(&SILENCE, 0, 2));
        let out = render(&mut renderer, 5);
        assert_near(&out, &[0.0, FULL * 0.4, FULL, FULL * 0.2, 0.0]);
    }

    #[test]
    fn averages_fast_bytes() {
        let mut renderer = renderer();
        let data = sample(vec![127, -127, 127, -127, 127]);
        // 2.5 bytes per output frame
        play(&mut renderer, range(&data, 0, 5), 2, range(&SILENCE, 0, 2));
        assert_near(&render(&mut renderer, 2), &[FULL * 0.2, FULL * 0.2]);
    }

    #[test]
    fn range_latch() {
        let mut renderer = renderer();
        let first = sample(vec![127, 127, 127]);
        let second = sample(vec![-127]);
        play(&mut renderer, range(&first, 0, 3), 5, range(&first, 0, 3));
        render(&mut renderer, 1);
        renderer.write(0, range(&second, 0, 1));
        assert_near(&render(&mut renderer, 4), &[FULL, FULL, -FULL, -FULL]);
    }

    #[test]
    fn dma_on_twice() {
        let mut renderer = renderer();
        let data = sample(vec![0, 127, -127]);
        play(&mut renderer, range(&data, 0, 3), 5, range(&data, 0, 3));
        render(&mut renderer, 1);
        renderer.write(0, PaulaWrite::DmaOn);
        assert_near(&render(&mut renderer, 2), &[FULL, -FULL]);
    }

    #[test]
    fn dma_off_on() {
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
    fn period_change() {
        let mut renderer = renderer();
        let data = sample(vec![127, -127, 127, -127]);
        play(&mut renderer, range(&data, 0, 4), 5, range(&data, 0, 4));
        render(&mut renderer, 1);
        renderer.write(0, PaulaWrite::Period(10));
        assert_near(&render(&mut renderer, 2), &[-FULL, -FULL]);
    }

    #[test]
    fn volume() {
        let mut renderer = renderer();
        let data = sample(vec![127]);
        play(&mut renderer, range(&data, 0, 1), 5, range(&data, 0, 1));
        renderer.write(0, PaulaWrite::Volume(32));
        assert_near(&render(&mut renderer, 1), &[FULL / 2.0]);
        renderer.write(0, PaulaWrite::Volume(200));
        assert_near(&render(&mut renderer, 1), &[FULL]);
    }

    #[test]
    fn bad_channel() {
        let mut renderer = renderer();
        renderer.write(9, PaulaWrite::DmaOn);
        assert_near(&render(&mut renderer, 1), &[0.0]);
    }
}
