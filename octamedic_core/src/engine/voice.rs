use std::sync::Arc;

use crate::data::{instrument::OctamedicSample, pattern::MAX_TRACKS};

/// How loud a single voice at full volume is in the mix, so four Amiga tracks fill the output.

const VOICE_GAIN: f32 = 0.25;

/// Full scale of a voice's output: an 8-bit sample times a volume of 64.

const VOICE_FULL_SCALE: f32 = 128.0 * 64.0;

/// A sample playing on a track, stepped through like an Amiga Paula channel.

struct Voice {
    sample: Arc<OctamedicSample>,
    /// Position in the sample in frames. Like Paula, each byte is held until the next one;
    /// there's no interpolation between them.
    position: f64,
    /// The rate the sample is played back at in Hz.
    rate: f64,
    /// 0-64, like a Paula channel volume.
    volume: u8,
}

impl Voice {
    /// The voice's output over the next `step` sample frames (one output frame), moving past
    /// them. Paula holds each byte for exactly `1 / rate` seconds, which doesn't line up with the
    /// output frames; picking one byte per output frame would shift every step by up to a frame
    /// and alias. Instead each byte counts for the part of the output frame it is held over.
    /// `None` once a one-shot sample has ended.

    fn next(&mut self, step: f64) -> Option<f32> {

        let data = &self.sample.data;

        if self.position as usize >= data.len() {

            return None;
        }

        let mut remaining = step;

        let mut sum = 0.0;

        while remaining > 0.0 {

            let index = self.position as usize;

            // past the end of a one-shot sample, the rest of the frame is silent.
            let Some(&byte) = data.get(index) else {

                break;
            };

            let next_byte = (index + 1) as f64;

            let held = (next_byte - self.position).min(remaining);

            sum += byte as f64 * held;

            remaining -= held;

            // landing on the next byte exactly, so float error can't leave a sliver of this one.
            self.position = if held == next_byte - self.position {

                next_byte
            } else {

                self.position + held
            };

            if let Some((start, end)) = self.sample.repeat
                && self.position >= end as f64
            {

                self.position = start as f64 + (self.position - end as f64);
            }
        }

        return Some((sum / step) as f32 * self.volume as f32);
    }
}

/// Tracks which sample each track is playing and mixes them into the output.

pub struct Voices {
    //each track's voice. when None, no sample is playing on that track.
    tracks: Vec<Option<Voice>>,
}

impl Voices {
    pub fn new() -> Self {

        return Self::default();
    }

    /// Starts `sample` on the track at `rate` Hz, cutting whatever the track was playing.

    pub fn note_on(&mut self, track: usize, sample: Arc<OctamedicSample>, rate: f64, volume: u8) {

        let Some(slot) = self.tracks.get_mut(track) else {

            return;
        };

        *slot = Some(Voice {
            sample,
            position: 0.0,
            rate,
            volume,
        });
    }

    /// Stops the track's sample.

    pub fn release(&mut self, track: usize) {

        if let Some(voice) = self.tracks.get_mut(track) {

            *voice = None;
        }
    }

    /// Stops every track.

    pub fn release_all(&mut self) {

        self.tracks.iter_mut().for_each(|voice| *voice = None);
    }

    /// The playback rate of the track's sample, if one is playing.

    pub fn rate(&self, track: usize) -> Option<f64> {

        return self.tracks.get(track)?.as_ref().map(|voice| voice.rate);
    }

    /// Adds every playing voice to `out`, an interleaved buffer with `channels` channels.
    /// This is the mixdown: each voice is its 8-bit bytes times a 0-64 volume, only turned into
    /// `f32` here.

    pub fn render(&mut self, out: &mut [f32], channels: usize, sample_rate: u32) {

        for slot in &mut self.tracks {

            let Some(voice) = slot else {

                continue;
            };

            let step = voice.rate / sample_rate as f64;

            for frame in out.chunks_exact_mut(channels) {

                let Some(value) = voice.next(step) else {

                    *slot = None;

                    break;
                };

                let value = value / VOICE_FULL_SCALE * VOICE_GAIN;

                frame.iter_mut().for_each(|sample| *sample += value);
            }
        }
    }
}

impl Default for Voices {
    fn default() -> Self {

        Self {
            tracks: (0..MAX_TRACKS).map(|_| None).collect(),
        }
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    const FULL: f32 = 127.0 * 64.0 / VOICE_FULL_SCALE * VOICE_GAIN;

    fn sample(data: Vec<i8>, repeat: Option<(usize, usize)>) -> Arc<OctamedicSample> {

        return Arc::new(OctamedicSample {
            data,
            repeat,
        });
    }

    #[test]

    fn one_shot_sample_stops_at_its_end() {

        let mut voices = Voices::new();

        voices.note_on(0, sample(vec![127, 127], None), 100.0, 64);

        let mut out = [0.0; 4];

        voices.render(&mut out, 1, 100);

        assert_eq!(out, [FULL, FULL, 0.0, 0.0]);

        assert!(voices.tracks[0].is_none());
    }

    #[test]

    fn looped_sample_wraps_into_the_loop() {

        let mut voices = Voices::new();

        voices.note_on(0, sample(vec![0, 127, -127], Some((1, 3))), 100.0, 64);

        let mut out = [0.0; 5];

        voices.render(&mut out, 1, 100);

        assert_eq!(out, [0.0, FULL, -FULL, FULL, -FULL]);
    }

    #[test]

    fn half_rate_holds_each_byte_without_interpolating() {

        let mut voices = Voices::new();

        voices.note_on(0, sample(vec![0, 127], None), 50.0, 64);

        let mut out = [0.0; 4];

        voices.render(&mut out, 1, 100);

        assert_eq!(out, [0.0, 0.0, FULL, FULL]);
    }

    #[test]

    fn a_byte_change_inside_a_frame_is_weighted_by_how_long_each_byte_is_held() {

        let mut voices = Voices::new();

        // 2.5 output frames per byte, so the second byte starts halfway through frame 2.
        voices.note_on(0, sample(vec![0, 127], None), 40.0, 64);

        let mut out = [0.0; 5];

        voices.render(&mut out, 1, 100);

        assert_eq!(out, [0.0, 0.0, FULL / 2.0, FULL, FULL]);
    }

    #[test]

    fn faster_than_the_output_rate_averages_the_bytes_in_a_frame() {

        let mut voices = Voices::new();

        voices.note_on(0, sample(vec![127, -127, 127, 127], None), 200.0, 64);

        let mut out = [0.0; 3];

        voices.render(&mut out, 1, 100);

        assert_eq!(out, [0.0, FULL, 0.0]);
    }

    #[test]

    fn a_one_shot_ending_inside_a_frame_is_silent_for_the_rest_of_it() {

        let mut voices = Voices::new();

        voices.note_on(0, sample(vec![127, 127, 127], None), 200.0, 64);

        let mut out = [0.0; 3];

        voices.render(&mut out, 1, 100);

        assert_eq!(out, [FULL, FULL / 2.0, 0.0]);
    }

    #[test]

    fn volume_scales_like_paula() {

        let mut voices = Voices::new();

        voices.note_on(0, sample(vec![127], None), 100.0, 32);

        let mut out = [0.0; 1];

        voices.render(&mut out, 1, 100);

        assert_eq!(out, [FULL / 2.0]);
    }

    #[test]

    fn every_output_channel_gets_the_voice() {

        let mut voices = Voices::new();

        voices.note_on(0, sample(vec![127], None), 100.0, 64);

        let mut out = [0.0; 2];

        voices.render(&mut out, 2, 100);

        assert_eq!(out, [FULL, FULL]);
    }
}
