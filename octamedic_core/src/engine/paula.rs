use std::sync::{Arc, LazyLock};

use crate::data::instrument::OctamedicSample;

/// Paula's PAL clock in Hz, the colour clock a period counts: a channel plays
/// `PAULA_CLOCK / period` bytes a second.

pub const PAULA_CLOCK: u32 = 3_546_895;

/// How loud a single voice at full volume is in the mix, so four Amiga tracks fill the output.

const VOICE_GAIN: f32 = 0.25;

/// Full scale of a voice's output: an 8-bit sample times a volume of 64.

const VOICE_FULL_SCALE: f32 = 128.0 * 64.0;

const MAX_VOLUME: u8 = 64;

/// Two zero bytes. Players point a channel at them to play nothing, like a one-shot sample's
/// "loop", since a channel with DMA on always plays something.

pub static SILENCE: LazyLock<Arc<OctamedicSample>> = LazyLock::new(|| {

    return Arc::new(OctamedicSample {
        data: vec![0, 0],
        repeat: None,
    });
});

/// A write to one of a Paula channel's registers, which is all a player does to make sound.
/// Logging these per tick lets octamedic's player be compared with the original's.

#[derive(Clone, Debug, PartialEq)]

pub enum PaulaWrite {
    /// `AUDxPER`: how many clock cycles each byte is held for.
    Period(u16),
    /// `AUDxVOL`, 0 to 64.
    Volume(u8),
    /// `AUDxLC` and `AUDxLEN`: the block of sample data the channel reads next. Like on Paula,
    /// the channel only takes it when it starts a block: when DMA turns on, or when the block
    /// it's playing ends. The length is in bytes rather than Paula's words, so odd-length
    /// samples keep their last byte.
    Sample {
        sample: Arc<OctamedicSample>,
        start: usize,
        len: usize,
    },
    /// Turns the channel's DMA on, which starts the block last written.
    DmaOn,
    /// Turns the channel's DMA off, silencing it.
    DmaOff,
}

/// Turns Paula register writes into audio, the way a particular kind of hardware or software
/// player does.

pub trait Renderer: Send {
    fn write(&mut self, channel: usize, write: PaulaWrite);

    /// Adds every channel's output to `out`, an interleaved buffer with `channels` channels.

    fn render(&mut self, out: &mut [f32], channels: usize);
}

#[derive(Clone)]

struct Block {
    sample: Arc<OctamedicSample>,
    start: usize,
    len: usize,
}

impl Block {
    fn end(&self) -> usize {

        return self.start + self.len;
    }
}

struct Playing {
    block: Block,
    /// Position in the sample's data in bytes, with the fraction of the byte already played.
    position: f64,
}

#[derive(Default)]

struct Channel {
    period: u16,
    volume: u8,
    /// The block the channel takes when it starts its next one.
    latch: Option<Block>,
    /// The block being played; `None` while DMA is off.
    playing: Option<Playing>,
}

impl Channel {
    fn write(&mut self, write: PaulaWrite) {

        match write {
            PaulaWrite::Period(period) => self.period = period,
            PaulaWrite::Volume(volume) => self.volume = volume.min(MAX_VOLUME),
            PaulaWrite::Sample {
                sample,
                start,
                len,
            } => {
                self.latch = Some(Block {
                    sample,
                    start,
                    len,
                })
            }
            PaulaWrite::DmaOn => {
                if self.playing.is_none() {

                    self.playing = self.latch.clone().map(|block| Playing {
                        position: block.start as f64,
                        block,
                    });
                }
            }
            PaulaWrite::DmaOff => self.playing = None,
        }
    }

    /// The channel's output over the next `step` bytes (one output frame), moving past them.
    /// Paula holds each byte for exactly one period, which doesn't line up with the output
    /// frames; picking one byte per output frame would shift every step by up to a frame and
    /// alias. Instead each byte counts for the part of the output frame it is held over. When a
    /// block ends, the channel starts the latched one, which is how samples loop.

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

            if playing.block.len == 0 {

                break;
            }

            let index = playing.position as usize;

            // past the sample's data, chip memory would hold whatever is there; zero stands in
            let byte = playing.block.sample.data.get(index).copied().unwrap_or(0);

            let next_byte = (index + 1) as f64;

            let held = (next_byte - playing.position).min(remaining);

            sum += byte as f64 * held;

            remaining -= held;

            // landing on the next byte exactly, so float error can't leave a sliver of this one.
            playing.position = if held == next_byte - playing.position {

                next_byte
            } else {

                playing.position + held
            };

            if playing.position >= playing.block.end() as f64 {

                let overshoot = playing.position - playing.block.end() as f64;

                if let Some(block) = latch {

                    playing.block = block.clone();
                }

                playing.position = playing.block.start as f64 + overshoot;
            }
        }

        return Some((sum / step) as f32 * *volume as f32);
    }
}

/// Plays Paula channels the way the Amiga's own hardware does, for 1 to 4 channel songs.

pub struct PaulaRenderer {
    sample_rate: u32,
    channels: Vec<Channel>,
}

impl PaulaRenderer {
    pub fn new(sample_rate: u32, channel_count: usize) -> Self {

        return Self {
            sample_rate,
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

            let rate = PAULA_CLOCK as f64 / channel.period as f64;

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

    /// A fifth of Paula's clock, so a period of 5 plays one byte per output frame.

    const SAMPLE_RATE: u32 = PAULA_CLOCK / 5;

    fn sample(data: Vec<i8>) -> Arc<OctamedicSample> {

        return Arc::new(OctamedicSample {
            data,
            repeat: None,
        });
    }

    fn block(sample: &Arc<OctamedicSample>, start: usize, len: usize) -> PaulaWrite {

        return PaulaWrite::Sample {
            sample: sample.clone(),
            start,
            len,
        };
    }

    /// A player's note on: the first block and period, DMA on, then the loop to latch.

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

        return PaulaRenderer::new(SAMPLE_RATE, 4);
    }

    #[test]

    fn a_one_shot_sample_falls_into_silence() {

        let mut renderer = renderer();

        let data = sample(vec![127, 127]);

        play(&mut renderer, block(&data, 0, 2), 5, block(&SILENCE, 0, 2));

        assert_near(&render(&mut renderer, 4), &[FULL, FULL, 0.0, 0.0]);
    }

    #[test]

    fn a_looped_sample_wraps_into_its_loop() {

        let mut renderer = renderer();

        let data = sample(vec![0, 127, -127]);

        play(&mut renderer, block(&data, 0, 3), 5, block(&data, 1, 2));

        assert_near(&render(&mut renderer, 5), &[0.0, FULL, -FULL, FULL, -FULL]);
    }

    #[test]

    fn double_the_period_holds_each_byte_twice_as_long() {

        let mut renderer = renderer();

        let data = sample(vec![0, 127]);

        play(&mut renderer, block(&data, 0, 2), 10, block(&SILENCE, 0, 2));

        assert_near(&render(&mut renderer, 4), &[0.0, 0.0, FULL, FULL]);
    }

    #[test]

    fn a_byte_change_inside_a_frame_is_weighted_by_how_long_each_byte_is_held() {

        let mut renderer = renderer();

        let data = sample(vec![0, 127]);

        // 1.6 output frames per byte, so the bytes change partway through frames
        play(&mut renderer, block(&data, 0, 2), 8, block(&SILENCE, 0, 2));

        let out = render(&mut renderer, 5);

        assert_near(&out, &[0.0, FULL * 0.4, FULL, FULL * 0.2, 0.0]);
    }

    #[test]

    fn faster_than_the_output_rate_averages_the_bytes_in_a_frame() {

        let mut renderer = renderer();

        let data = sample(vec![127, -127, 127, -127, 127]);

        // 2.5 bytes per output frame
        play(&mut renderer, block(&data, 0, 5), 2, block(&SILENCE, 0, 2));

        assert_near(&render(&mut renderer, 2), &[FULL * 0.2, FULL * 0.2]);
    }

    #[test]

    fn a_new_block_waits_until_the_playing_one_ends() {

        let mut renderer = renderer();

        let first = sample(vec![127, 127, 127]);

        let second = sample(vec![-127]);

        play(&mut renderer, block(&first, 0, 3), 5, block(&first, 0, 3));

        render(&mut renderer, 1);

        renderer.write(0, block(&second, 0, 1));

        assert_near(&render(&mut renderer, 4), &[FULL, FULL, -FULL, -FULL]);
    }

    #[test]

    fn dma_on_again_while_playing_changes_nothing() {

        let mut renderer = renderer();

        let data = sample(vec![0, 127, -127]);

        play(&mut renderer, block(&data, 0, 3), 5, block(&data, 0, 3));

        render(&mut renderer, 1);

        renderer.write(0, PaulaWrite::DmaOn);

        assert_near(&render(&mut renderer, 2), &[FULL, -FULL]);
    }

    #[test]

    fn dma_off_silences_the_channel_and_dma_on_restarts_the_latched_block() {

        let mut renderer = renderer();

        let data = sample(vec![127, -127]);

        play(&mut renderer, block(&data, 0, 2), 5, block(&data, 0, 2));

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

        play(&mut renderer, block(&data, 0, 4), 5, block(&data, 0, 4));

        render(&mut renderer, 1);

        renderer.write(0, PaulaWrite::Period(10));

        assert_near(&render(&mut renderer, 2), &[-FULL, -FULL]);
    }

    #[test]

    fn volume_scales_like_paula() {

        let mut renderer = renderer();

        let data = sample(vec![127]);

        play(&mut renderer, block(&data, 0, 1), 5, block(&data, 0, 1));

        renderer.write(0, PaulaWrite::Volume(32));

        assert_near(&render(&mut renderer, 1), &[FULL / 2.0]);

        renderer.write(0, PaulaWrite::Volume(200));

        assert_near(&render(&mut renderer, 1), &[FULL]);
    }

    #[test]

    fn every_output_channel_gets_the_voice() {

        let mut renderer = renderer();

        let data = sample(vec![127]);

        play(&mut renderer, block(&data, 0, 1), 5, block(&data, 0, 1));

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
