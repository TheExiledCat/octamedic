use std::{ops::Range, sync::Arc};

pub use mmd::mmd::module::MAX_INSTRUMENTS;
use mmd::mmd::{
    module::{OctamedMMD0InstrumentType, OctamedMMD0Sample, OctamedMMD0SampleHeader},
    synth::OctamedMMD0SynthInstrument,
};

use crate::data::volume::MAX_VOLUME;

// same as paula's max
pub const MAX_INSTRUMENT_VOLUME: u8 = MAX_VOLUME;

// eighths of a semitone
pub const MIN_FINETUNE: i8 = -8;
pub const MAX_FINETUNE: i8 = 7;

// two digits on octamed's instrument panel
pub const MAX_HOLD_DECAY: u8 = 99;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InstrumentId(pub u8);

impl InstrumentId {
    // instruments are 1-based in patterns, 0 = none
    pub fn index(&self) -> Option<usize> {
        return (self.0 as usize).checked_sub(1);
    }
}

/// How many octave copies a sample holds, see docs/FILES.md.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SampleOctaves {
    #[default]
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    /// Plays two octaves lower than a normal sample.
    ExtraLow,
}

impl SampleOctaves {
    /// How many copies of the sound there are.
    pub fn copies(self) -> u32 {
        return match self {
            Self::One | Self::ExtraLow => 1,
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
            Self::Five => 5,
            Self::Six => 6,
            Self::Seven => 7,
        };
    }

    pub fn from_octamed(kind: OctamedMMD0InstrumentType) -> Self {
        return match kind {
            OctamedMMD0InstrumentType::SampleOct2 => Self::Two,
            OctamedMMD0InstrumentType::SampleOct3 => Self::Three,
            OctamedMMD0InstrumentType::SampleOct4 => Self::Four,
            OctamedMMD0InstrumentType::SampleOct5 => Self::Five,
            OctamedMMD0InstrumentType::SampleOct6 => Self::Six,
            OctamedMMD0InstrumentType::SampleOct7 => Self::Seven,
            OctamedMMD0InstrumentType::ExtSample => Self::ExtraLow,
            _ => Self::One,
        };
    }

    pub fn to_octamed(self) -> OctamedMMD0InstrumentType {
        return match self {
            Self::One => OctamedMMD0InstrumentType::Sample,
            Self::Two => OctamedMMD0InstrumentType::SampleOct2,
            Self::Three => OctamedMMD0InstrumentType::SampleOct3,
            Self::Four => OctamedMMD0InstrumentType::SampleOct4,
            Self::Five => OctamedMMD0InstrumentType::SampleOct5,
            Self::Six => OctamedMMD0InstrumentType::SampleOct6,
            Self::Seven => OctamedMMD0InstrumentType::SampleOct7,
            Self::ExtraLow => OctamedMMD0InstrumentType::ExtSample,
        };
    }
}

#[derive(Debug, PartialEq)]
pub struct OctamedicSample {
    /// Every octave's copy, shortest first.
    pub data: Vec<i8>,
    /// Loop start..end in bytes, of the shortest copy for a multi-octave sample.
    pub repeat: Option<(usize, usize)>,
    pub octaves: SampleOctaves,
}

/// Where one octave copy of a sample is, and its loop, in bytes from the start of the data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OctaveCopy {
    pub data: Range<usize>,
    pub repeat: Option<Range<usize>>,
}

impl OctamedicSample {
    /// A copy with `range` silenced. Like the rest of these, `range` has to be inside the sample.
    pub fn cleared(&self, range: Range<usize>) -> Self {
        let mut data = self.data.clone();
        data[range].fill(0);
        return Self { data, ..*self };
    }

    /// A copy with `inserted` put in before sample `at`, what was there moving along to make room.
    /// A loop moves with the samples it's on, and grows if it's put in the middle of it.
    pub fn with_inserted(&self, at: usize, inserted: &[i8]) -> Self {
        let mut data = self.data.clone();
        data.splice(at..at, inserted.iter().copied());
        let added = inserted.len();
        let repeat = self.repeat.map(|(start, end)| {
            let start = if start >= at { start + added } else { start };
            let end = if end > at { end + added } else { end };
            return (start, end);
        });
        return Self {
            data,
            repeat,
            octaves: self.octaves,
        };
    }

    pub fn reversed(&self, range: Range<usize>) -> Self {
        let mut data = self.data.clone();
        data[range].reverse();
        return Self { data, ..*self };
    }

    /// A copy with `range`'s volume faded linearly from `from` percent at its first sample to `to`
    /// percent at its last, clipping at the limits.
    pub fn with_volume_changed(&self, range: Range<usize>, from: u32, to: u32) -> Self {
        let mut data = self.data.clone();
        let steps = range.len().saturating_sub(1).max(1) as f64;
        for (step, sample) in data[range].iter_mut().enumerate() {
            let percent = from as f64 + (to as f64 - from as f64) * step as f64 / steps;
            let scaled = (*sample as f64 * percent / 100.0).round();
            *sample = scaled.clamp(i8::MIN as f64, i8::MAX as f64) as i8;
        }
        return Self { data, ..*self };
    }

    /// A copy with `other` mixed in from sample `at`, each sample halfway between the two so it
    /// can't clip. It's made longer if `other` runs past the end.
    pub fn mixed(&self, at: usize, other: &[i8]) -> Self {
        let mut data = self.data.clone();
        let end = at + other.len();
        if end > data.len() {
            data.resize(end, 0);
        }
        for (sample, &mixed_in) in data[at..end].iter_mut().zip(other) {
            *sample = ((*sample as i16 + mixed_in as i16) / 2) as i8;
        }
        return Self { data, ..*self };
    }

    pub fn resized(&self, length: usize) -> Self {
        if length <= self.data.len() {
            return self.without(length..self.data.len());
        }
        let mut data = self.data.clone();
        data.resize(length, 0);
        return Self { data, ..*self };
    }

    /// A copy with `range` cut out, the rest closing up behind it. A loop moves with the samples
    /// it's on, and goes once there's nothing of it left.
    pub fn without(&self, range: Range<usize>) -> Self {
        let mut data = self.data.clone();
        data.drain(range.clone());
        let moved = |at: usize| {
            if at >= range.end {
                return at - range.len();
            }
            return at.min(range.start);
        };
        let repeat = self
            .repeat
            .map(|(start, end)| (moved(start), moved(end)))
            .filter(|(start, end)| start < end);
        return Self {
            data,
            repeat,
            octaves: self.octaves,
        };
    }

    /// Copy `copy`, counting from the shortest. Each is twice as long as the one before, from
    /// `length / (2^copies - 1)` up, and the loop doubles with it. A plain sample's copy 0 is all
    /// of it.
    pub fn octave_copy(&self, copy: u32) -> OctaveCopy {
        let copies = self.octaves.copies();
        let copy = copy.min(copies - 1);
        let shortest = self.data.len() / ((1 << copies) - 1);
        let start = shortest * ((1 << copy) - 1);
        let end = (start + (shortest << copy)).min(self.data.len());
        let repeat = self.repeat.and_then(|(from, to)| {
            let from = (start + (from << copy)).min(end);
            let to = (start + (to << copy)).min(end);
            (from < to).then_some(from..to)
        });
        return OctaveCopy {
            data: start..end,
            repeat,
        };
    }

    pub fn with_loop(&self, repeat: Option<(usize, usize)>) -> Self {
        return Self {
            data: self.data.clone(),
            repeat,
            ..*self
        };
    }

    /// How far a loop can reach: it's in the shortest octave copy, which for a plain sample is
    /// all of it.
    pub fn loop_limit(&self) -> usize {
        return self.octave_copy(0).data.end;
    }

    /// The nearest silent sample past `from`, towards the end or the start.
    pub fn zero_from(&self, from: usize, forward: bool) -> Option<usize> {
        let silent = |&index: &usize| self.data.get(index) == Some(&0);
        if forward {
            return (from + 1..self.data.len()).find(silent);
        }
        return (0..from.min(self.data.len())).rev().find(silent);
    }

    pub fn with_value(&self, index: usize, value: i8) -> Self {
        let mut data = self.data.clone();
        data[index] = value;
        return Self { data, ..*self };
    }
    /// Everything ends up 8-bit mono: 16-bit keeps the high byte, stereo gets mixed down.
    pub fn from_octamed(
        bytes: &[i8],
        header: &OctamedMMD0SampleHeader,
        info: &OctamedMMD0Sample,
    ) -> Self {
        let mono: Vec<i8> = if header.is_16_bit() {
            bytes.chunks_exact(2).map(|pair| pair[0]).collect()
        } else {
            bytes.to_vec()
        };
        let data = if header.is_stereo() {
            let (left, right) = mono.split_at(mono.len() / 2);
            left.iter()
                .zip(right)
                .map(|(&l, &r)| ((l as i16 + r as i16) / 2) as i8)
                .collect()
        } else {
            mono
        };
        // repeat is in words
        let bytes_per_frame = if header.is_16_bit() { 2 } else { 1 };
        let start = info.repeat.0 as usize * 2 / bytes_per_frame;
        let length = info.repeat_length.0 as usize * 2 / bytes_per_frame;
        let end = (start + length).min(data.len());
        let repeat = (info.repeat_length.0 > 1 && start < end).then_some((start, end));
        let octaves = SampleOctaves::from_octamed(header.sample_type);
        return Self {
            data,
            repeat,
            octaves,
        };
    }
}

/// The volume and waveform programs and their waveforms. Table bytes below $80 are values,
/// the rest commands, see format.md.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OctamedicSynth {
    pub default_decay: u8,
    pub volume_speed: u8,
    pub waveform_speed: u8,
    pub volume_table: Vec<u8>,
    pub waveform_table: Vec<u8>,
    pub waveforms: Vec<Vec<i8>>,
}

impl OctamedicSynth {
    /// A hybrid's first waveform is its sample, so it's left out here.
    pub fn from_octamed(synth: &OctamedMMD0SynthInstrument) -> Self {
        return Self {
            default_decay: synth.default_decay.0,
            volume_speed: synth.volume_speed.0,
            waveform_speed: synth.waveform_speed.0,
            volume_table: synth.volume_table.iter().map(|byte| byte.0).collect(),
            waveform_table: synth.waveform_table.iter().map(|byte| byte.0).collect(),
            waveforms: synth
                .waveforms
                .iter()
                .skip(synth.is_hybrid() as usize)
                .map(|wave| wave.iter().map(|byte| byte.0).collect())
                .collect(),
        };
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum InstrumentKind {
    /// No sound of its own, like a MIDI only instrument.
    Empty,
    Sample(Arc<OctamedicSample>),
    Synth(Arc<OctamedicSynth>),
    Hybrid(Arc<OctamedicSynth>, Arc<OctamedicSample>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MidiSettings {
    /// 0 for none.
    pub channel: u8,
    pub preset: u8,
    pub suppress_note_off: bool,
}

#[derive(Clone)]
pub struct OctamedicInstrument {
    pub name: String,
    pub volume: u8,
    pub transpose: i8,
    pub finetune: i8,
    pub hold: u8,
    pub decay: u8,
    pub midi: MidiSettings,
    pub kind: InstrumentKind,
}

impl OctamedicInstrument {
    pub fn new(name: impl Into<String>) -> Self {
        return Self {
            name: name.into(),
            volume: MAX_INSTRUMENT_VOLUME,
            transpose: 0,
            finetune: 0,
            hold: 0,
            decay: 0,
            midi: MidiSettings::default(),
            kind: InstrumentKind::Empty,
        };
    }

    pub fn data_bytes(&self) -> usize {
        let waveforms =
            |synth: &OctamedicSynth| synth.waveforms.iter().map(Vec::len).sum::<usize>();
        return match &self.kind {
            InstrumentKind::Empty => 0,
            InstrumentKind::Sample(sample) => sample.data.len(),
            InstrumentKind::Synth(synth) => waveforms(synth),
            InstrumentKind::Hybrid(synth, sample) => waveforms(synth) + sample.data.len(),
        };
    }

    // the engine only plays plain samples so far
    pub fn sample(&self) -> Option<&Arc<OctamedicSample>> {
        return match &self.kind {
            InstrumentKind::Sample(sample) => Some(sample),
            _ => None,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ramp() -> OctamedicSample {
        return OctamedicSample {
            data: (1..=10).collect(),
            repeat: Some((6, 9)),
            octaves: SampleOctaves::One,
        };
    }

    #[test]
    fn clearing_silences_the_range() {
        let cleared = ramp().cleared(2..5);
        assert_eq!(cleared.data, [1, 2, 0, 0, 0, 6, 7, 8, 9, 10]);
        assert_eq!(cleared.repeat, Some((6, 9)));
    }

    #[test]
    fn changing_volume_fades_across_the_range() {
        let sample = OctamedicSample {
            data: vec![100; 5],
            repeat: None,
            octaves: SampleOctaves::One,
        };
        let faded = sample.with_volume_changed(1..4, 100, 0);
        assert_eq!(faded.data, [100, 100, 50, 0, 100]);
        let louder = sample.with_volume_changed(0..5, 200, 200);
        assert_eq!(louder.data, [127; 5]);
        let negative = OctamedicSample {
            data: vec![-100],
            ..sample
        };
        assert_eq!(negative.with_volume_changed(0..1, 50, 0).data, [-50]);
        assert_eq!(negative.with_volume_changed(0..1, 300, 0).data, [-128]);
    }

    #[test]
    fn resizing_pads_or_cuts() {
        let longer = ramp().resized(12);
        assert_eq!(longer.data[8..], [9, 10, 0, 0]);
        assert_eq!(longer.repeat, Some((6, 9)));
        let shorter = ramp().resized(7);
        assert_eq!(shorter.data, [1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(shorter.repeat, Some((6, 7)));
    }

    #[test]
    fn inserting_makes_room() {
        let pasted = ramp().with_inserted(2, &[0, 0]);
        assert_eq!(pasted.data, [1, 2, 0, 0, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(pasted.repeat, Some((8, 11)));
        // inside the loop it grows, at its end it doesn't
        assert_eq!(ramp().with_inserted(7, &[0]).repeat, Some((6, 10)));
        assert_eq!(ramp().with_inserted(9, &[0]).repeat, Some((6, 9)));
    }

    #[test]
    fn reversing_turns_the_range_round() {
        let reversed = ramp().reversed(2..6);
        assert_eq!(reversed.data, [1, 2, 6, 5, 4, 3, 7, 8, 9, 10]);
        assert_eq!(reversed.repeat, Some((6, 9)));
    }

    #[test]
    fn mixing_halves_both() {
        let mixed = ramp().mixed(8, &[4, 6, -10]);
        assert_eq!(mixed.data[7..], [8, 6, 8, -5]);
        assert_eq!(ramp().mixed(0, &[127]).data[0], 64);
    }

    #[test]
    fn deleting_closes_up_the_gap() {
        let deleted = ramp().without(2..5);
        assert_eq!(deleted.data, [1, 2, 6, 7, 8, 9, 10]);
        assert_eq!(deleted.repeat, Some((3, 6)));
        assert_eq!(ramp().without(7..8).repeat, Some((6, 8)));
        assert_eq!(ramp().without(4..7).repeat, Some((4, 6)));
        assert_eq!(ramp().without(5..9).repeat, None);
        assert_eq!(ramp().without(0..10).data, []);
    }

    #[test]
    fn octave_copies_double() {
        let sample = OctamedicSample {
            data: (0..15).collect(),
            repeat: Some((1, 2)),
            octaves: SampleOctaves::Three,
        };
        // 2 bytes, then 4, then 8, the odd byte left over
        let copy = |k| sample.octave_copy(k);
        assert_eq!(copy(0).data, 0..2);
        assert_eq!(copy(1).data, 2..6);
        assert_eq!(
            copy(2),
            OctaveCopy {
                data: 6..14,
                repeat: Some(10..14),
            }
        );
        assert_eq!(copy(9).data, 6..14);
        let plain = OctamedicSample {
            data: vec![0; 5],
            repeat: None,
            octaves: SampleOctaves::One,
        };
        assert_eq!(plain.octave_copy(0).data, 0..5);
    }

    #[test]
    fn finding_zeros() {
        let mut sample = ramp();
        sample.data[2] = 0;
        sample.data[7] = 0;
        assert_eq!(sample.zero_from(4, true), Some(7));
        assert_eq!(sample.zero_from(4, false), Some(2));
        assert_eq!(sample.zero_from(7, true), None);
        assert_eq!(sample.zero_from(2, false), None);
    }

    #[test]
    fn a_loop_stays_in_the_shortest_copy() {
        let mut sample = ramp();
        assert_eq!(sample.loop_limit(), 10);
        sample.data = vec![0; 21];
        sample.octaves = SampleOctaves::Three;
        assert_eq!(sample.loop_limit(), 3);
        assert_eq!(sample.with_loop(None).repeat, None);
    }
}
