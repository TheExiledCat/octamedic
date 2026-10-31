pub use octamed::mmd::module::MAX_TRACKS;

pub const DEFAULT_LINES: usize = 64;
use octamed::{
    mmd::module::{OctamedMMD0Block, OctamedMMD0BlockLine, OctamedMMD1Block},
    utility::bytes::{UByte, ValueMap},
};

use crate::data::{
    command::CommandId, instrument::InstrumentId, note::OctamedicNote, track_count::TrackCount,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PatternId(pub u8);

impl PatternId {
    pub const ZERO: PatternId = PatternId(0);
}

impl From<UByte> for PatternId {
    fn from(value: UByte) -> Self {
        return Self(value.0);
    }
}

#[derive(Clone)]
pub struct OctamedicPattern {
    pub lines: Vec<OctamedicPatternLine>,
}

impl OctamedicPattern {
    pub fn new() -> Self {
        return Self {
            lines: vec![OctamedicPatternLine::new(TrackCount::default().tracks()); DEFAULT_LINES],
        };
    }

    fn from_octamed(lines: &[OctamedMMD0BlockLine]) -> Self {
        let lines = lines
            .iter()
            .map(|line| OctamedicPatternLine {
                tracks: line
                    .tracks
                    .iter()
                    .map(|t| OctamedicPatternTrack {
                        note: OctamedicNote::from_octamed(t.note_number),
                        instrument_id: t.instrument_number.remap(InstrumentId),
                        command_id: t.command_number.remap(CommandId),
                        command_value: t.command_value.0,
                    })
                    .collect(),
            })
            .collect();
        return Self { lines };
    }

    pub fn line_count(&self) -> usize {
        return self.lines.len();
    }

    /// Every line has the same number of tracks.
    pub fn track_count(&self) -> usize {
        return self.lines.first().map_or(0, |line| line.tracks.len());
    }

    pub fn track(&self, line: usize, track: usize) -> Option<&OctamedicPatternTrack> {
        return self.lines.get(line)?.tracks.get(track);
    }

    pub fn track_mut(&mut self, line: usize, track: usize) -> Option<&mut OctamedicPatternTrack> {
        return self.lines.get_mut(line)?.tracks.get_mut(track);
    }
}

impl Default for OctamedicPattern {
    fn default() -> Self {
        return Self::new();
    }
}

impl From<&OctamedMMD0Block> for OctamedicPattern {
    fn from(value: &OctamedMMD0Block) -> Self {
        return Self::from_octamed(&value.lines);
    }
}

impl From<&OctamedMMD1Block> for OctamedicPattern {
    fn from(value: &OctamedMMD1Block) -> Self {
        return Self::from_octamed(&value.lines);
    }
}

#[derive(Clone)]
pub struct OctamedicPatternLine {
    pub tracks: Vec<OctamedicPatternTrack>,
}

impl OctamedicPatternLine {
    pub(crate) fn new(track_count: usize) -> Self {
        return OctamedicPatternLine {
            tracks: vec![OctamedicPatternTrack::new(); track_count],
        };
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OctamedicPatternTrack {
    pub note: OctamedicNote,
    pub instrument_id: InstrumentId,
    pub command_id: CommandId,
    pub command_value: u8,
}

impl OctamedicPatternTrack {
    pub fn new() -> Self {
        return Self {
            note: OctamedicNote::new(),
            instrument_id: InstrumentId(0),
            command_id: CommandId(0),
            command_value: 0,
        };
    }
}

impl Default for OctamedicPatternTrack {
    fn default() -> Self {
        return Self::new();
    }
}
