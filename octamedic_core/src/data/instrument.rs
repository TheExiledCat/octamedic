use std::sync::Arc;

pub use octamed::mmd::module::MAX_INSTRUMENTS;
use octamed::mmd::module::{OctamedMMD0Sample, OctamedMMD0SampleHeader};

/// Also Paula's highest channel volume.
pub const MAX_INSTRUMENT_VOLUME: u8 = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InstrumentId(pub u8);

impl InstrumentId {
    /// Pattern instrument numbers start at 1; 0 means none.
    pub fn index(&self) -> Option<usize> {
        return (self.0 as usize).checked_sub(1);
    }
}

/// 8-bit signed sample data.
#[derive(Debug, PartialEq)]
pub struct OctamedicSample {
    pub data: Vec<i8>,
    /// Loop range in bytes; `None` for a one-shot sample.
    pub repeat: Option<(usize, usize)>,
}

impl OctamedicSample {
    /// 16-bit samples keep their high byte and stereo ones are mixed down, so everything plays
    /// as 8-bit mono.
    pub fn from_octamed(
        bytes: &[i8],
        header: &OctamedMMD0SampleHeader,
        info: &OctamedMMD0Sample,
    ) -> Self {
        let mono: Vec<i8> = if header.is_16_bit() {
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| pair[0])
                .collect()
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
        // stored in words
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
    /// -8 to 7, in eighths of a semitone.
    pub finetune: i8,
    /// OctaMED's hold and decay settings, as stored.
    pub hold: u8,
    pub decay: u8,
    /// `None` for instruments the engine can't play yet, like synths.
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
