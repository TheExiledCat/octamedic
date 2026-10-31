use std::sync::Arc;

pub use octamed::mmd::module::MAX_INSTRUMENTS;
use octamed::mmd::module::{OctamedMMD0Sample, OctamedMMD0SampleHeader};

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

#[derive(Debug, PartialEq)]
pub struct OctamedicSample {
    pub data: Vec<i8>,
    /// Loop start..end in bytes.
    pub repeat: Option<(usize, usize)>,
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
        return Self { data, repeat };
    }
}

#[derive(Clone)]
pub struct OctamedicInstrument {
    pub name: String,
    pub volume: u8,
    pub transpose: i8,
    pub finetune: i8,
    pub hold: u8,
    pub decay: u8,
    // None for synths etc, can't play those yet
    pub sample: Option<Arc<OctamedicSample>>,
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
            sample: None,
        };
    }
}
