use std::sync::Arc;

pub use octamed::mmd::module::MAX_INSTRUMENTS;
use octamed::mmd::{
    module::{OctamedMMD0InstrumentType, OctamedMMD0Sample, OctamedMMD0SampleHeader},
    synth::OctamedMMD0SynthInstrument,
};

use crate::data::volume::MAX_VOLUME;

// same as paula's max
pub const MAX_INSTRUMENT_VOLUME: u8 = MAX_VOLUME;

// eighths of a semitone
pub const MIN_FINETUNE: i8 = -8;
pub const MAX_FINETUNE: i8 = 7;

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
}

#[derive(Debug, PartialEq)]
pub struct OctamedicSample {
    /// Every octave's copy, shortest first.
    pub data: Vec<i8>,
    /// Loop start..end in bytes, of the shortest copy for a multi-octave sample.
    pub repeat: Option<(usize, usize)>,
    pub octaves: SampleOctaves,
}

impl OctamedicSample {
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
