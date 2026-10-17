use octamed::{
    mmd::module::{OctamedMMD0Block, OctamedMMD1Block},
    utility::bytes::{UByte, UWord, ValueMap},
};

use crate::data::{command::CommandId, instrument::InstrumentId, note::OctamedicNote};

/// Most tracks a pattern can have.

pub const MAX_TRACKS: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]

pub struct PatternId(pub u8);

impl From<UByte> for PatternId {
    fn from(value: UByte) -> Self {

        return Self(value.0 as u8);
    }
}

#[derive(Clone)]

pub struct OctamedicPattern {
    pub track_count: UWord,
    pub line_count: UWord,
    pub lines: Vec<OctamedicPatternLine>,
}

impl OctamedicPattern {
    pub fn new() -> Self {

        let line_count = UWord(64);

        let track_count = UWord(4);

        return Self {
            track_count,
            line_count,
            lines: vec![OctamedicPatternLine::new(track_count); line_count.0 as usize],
        };
    }
}

impl OctamedicPattern {
    pub fn line_count(&self) -> usize {

        return self.lines.len();
    }

    pub fn track(&self, line: usize, track: usize) -> Option<&OctamedicPatternTrack> {

        return self.lines.get(line)?.tracks.get(track);
    }

    pub fn track_mut(&mut self, line: usize, track: usize) -> Option<&mut OctamedicPatternTrack> {

        return self.lines.get_mut(line)?.tracks.get_mut(track);
    }
}

impl From<&OctamedMMD0Block> for OctamedicPattern {
    fn from(value: &OctamedMMD0Block) -> Self {

        let track_count = UWord(value.lines.first().unwrap().tracks.len() as u16);

        let line_count = UWord(value.lines.len() as u16);

        let s = Self {
            track_count,
            line_count,
            lines: value
                .lines
                .iter()
                .map(|l| OctamedicPatternLine {
                    tracks: l
                        .tracks
                        .iter()
                        .map(|t| OctamedicPatternTrack {
                            note: OctamedicNote::from_octamed(t.note_number),
                            instrument_id: t.instrument_number.remap(|i| InstrumentId(i)),
                            command_id: t.command_number.remap(|c| CommandId(c)),
                            command_value: t.command_value,
                        })
                        .collect(),
                })
                .collect(),
        };

        return s;
    }
}

impl From<&OctamedMMD1Block> for OctamedicPattern {
    fn from(value: &OctamedMMD1Block) -> Self {

        let track_count = UWord(value.lines.first().unwrap().tracks.len() as u16);

        let line_count = UWord(value.lines.len() as u16);

        let s = Self {
            track_count,
            line_count,
            lines: value
                .lines
                .iter()
                .map(|l| OctamedicPatternLine {
                    tracks: l
                        .tracks
                        .iter()
                        .map(|t| OctamedicPatternTrack {
                            note: OctamedicNote::from_octamed(t.note_number),
                            instrument_id: t.instrument_number.remap(|i| InstrumentId(i)),
                            command_id: t.command_number.remap(|c| CommandId(c)),
                            command_value: t.command_value,
                        })
                        .collect(),
                })
                .collect(),
        };

        return s;
    }
}

#[derive(Clone)]

pub struct OctamedicPatternLine {
    pub tracks: Vec<OctamedicPatternTrack>,
}

impl OctamedicPatternLine {
    pub(crate) fn new(track_count: UWord) -> Self {

        return OctamedicPatternLine {
            tracks: vec![OctamedicPatternTrack::new(); track_count.0 as usize],
        };
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]

pub struct OctamedicPatternTrack {
    pub note: OctamedicNote,
    pub instrument_id: InstrumentId,
    pub command_id: CommandId,
    pub command_value: UByte,
}

impl OctamedicPatternTrack {
    pub fn new() -> Self {

        return Self {
            note: OctamedicNote::new(),
            instrument_id: InstrumentId(0),
            command_id: CommandId(0),
            command_value: UByte(0),
        };
    }
}
