use octamed::utility::bytes::{Byte, UByte, UWord};

use crate::data::{
    instrument::{InstrumentId, OctamedicInstrument},
    pattern::{OctamedicPattern, PatternId},
    tempo::OctamedicTempo,
    volume::OctamedicVolume,
};

#[derive(Clone, Copy)]

pub struct SongId(pub u8);

pub struct OctamedicSong {
    pub(crate) name: String,
    pub(crate) instruments: [Option<OctamedicInstrument>; 63],
    pub(crate) block_count: UWord,
    pub(crate) song_length: UWord,
    pub(crate) sequence: Vec<UByte>,
    pub(crate) tempo: OctamedicTempo,
    pub(crate) global_transpose: Byte,
    pub(crate) track_volumes: [OctamedicVolume; 16],
    pub(crate) master_volume: OctamedicVolume,
    pub(crate) patterns: Vec<OctamedicPattern>,
}

impl OctamedicSong {
    pub fn new(name: impl AsRef<str>) -> Self {

        let name = name.as_ref();

        return Self {
            name: name.into(),
            instruments: std::array::from_fn(|_| None),
            block_count: UWord(1),
            song_length: UWord(1),
            sequence: vec![UByte(0)],
            tempo: OctamedicTempo::new(),
            global_transpose: Byte(0),
            track_volumes: [OctamedicVolume::new(64); 16],
            master_volume: OctamedicVolume::new(64),
            patterns: vec![OctamedicPattern::new()],
        };
    }

    pub fn get_pattern(&self, pattern_id: &PatternId) -> Option<&OctamedicPattern> {

        return self.patterns.get(pattern_id.0 as usize);
    }

    pub fn get_sequence(&self) -> Vec<UByte> {

        return self.sequence.clone();
    }

    pub fn name(&self) -> &str {

        return &self.name;
    }

    pub fn patterns(&self) -> &[OctamedicPattern] {

        return &self.patterns;
    }

    pub fn get_pattern_mut(&mut self, pattern_id: PatternId) -> Option<&mut OctamedicPattern> {

        return self.patterns.get_mut(pattern_id.0 as usize);
    }

    // creates empty patterns up to `pattern_id` if the song has fewer
    pub fn ensure_pattern(&mut self, pattern_id: PatternId) -> &mut OctamedicPattern {

        let index = pattern_id.0 as usize;

        if index >= self.patterns.len() {

            self.patterns.resize_with(index + 1, OctamedicPattern::new);

            self.block_count = UWord(self.patterns.len() as u16);
        }

        return &mut self.patterns[index];
    }

    pub fn sequence(&self) -> &[UByte] {

        return &self.sequence;
    }

    pub fn tempo(&self) -> OctamedicTempo {

        return self.tempo;
    }

    pub fn tempo_mut(&mut self) -> &mut OctamedicTempo {

        return &mut self.tempo;
    }

    pub fn instrument(&self, id: InstrumentId) -> Option<&OctamedicInstrument> {

        return self.instruments.get(id.index()?)?.as_ref();
    }
}
