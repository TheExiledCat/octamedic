use octamed::utility::bytes::{Byte, UByte, UWord};

use crate::data::{
    instrument::OctamedicInstrument,
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
            instruments: [None; 63],
            block_count: UWord(1),
            song_length: UWord(1),
            sequence: vec![UByte(0)],
            tempo: OctamedicTempo::new(),
            global_transpose: Byte(0),
            track_volumes: [OctamedicVolume::new(0); 16],
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
}
