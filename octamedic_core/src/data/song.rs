use std::sync::Arc;

use mmd::mmd::module::SONG_TRACKS;

use crate::data::{
    instrument::{InstrumentId, MAX_INSTRUMENTS, OctamedicInstrument},
    pattern::{OctamedicPattern, PatternId},
    tempo::OctamedicTempo,
    volume::{MAX_VOLUME, OctamedicVolume},
};

#[derive(Clone, Copy)]
pub struct SongId(pub u8);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OctamedicSongFlags {
    pub filter_on: bool,
    // volume command arg is hex instead of decimal
    pub volumes_hex: bool,
    // ST/NT/PT style slides
    pub st_slides: bool,
    // 5-8 channel mode, software mixed on the real thing
    pub eight_channel: bool,
    // the mouse pointer follows the play position
    pub jumping: bool,
    pub jump_every_8th_line: bool,
    pub hq_v2_compatible: bool,
    pub mixing: bool,
}

pub struct OctamedicSong {
    pub instruments: [Option<OctamedicInstrument>; MAX_INSTRUMENTS],
    pub flags: OctamedicSongFlags,
    pub global_transpose: i8,
    pub track_volumes: [OctamedicVolume; SONG_TRACKS],
    pub master_volume: OctamedicVolume,
    pub patterns: Vec<Arc<OctamedicPattern>>,
    name: String,
    sequence: Vec<PatternId>,
    tempo: OctamedicTempo,
}

impl OctamedicSong {
    pub fn new(name: impl AsRef<str>) -> Self {
        let name = name.as_ref();
        return Self {
            name: name.into(),
            instruments: std::array::from_fn(|_| None),
            sequence: vec![PatternId(0)],
            tempo: OctamedicTempo::new(),
            flags: OctamedicSongFlags::default(),
            global_transpose: 0,
            track_volumes: [OctamedicVolume::new(MAX_VOLUME); SONG_TRACKS],
            master_volume: OctamedicVolume::new(MAX_VOLUME),
            patterns: vec![Arc::new(OctamedicPattern::new())],
        };
    }

    pub fn get_pattern(&self, pattern_id: &PatternId) -> Option<&OctamedicPattern> {
        return self.patterns.get(pattern_id.0 as usize).map(Arc::as_ref);
    }

    pub fn get_shared_pattern(&self, pattern_id: &PatternId) -> Option<Arc<OctamedicPattern>> {
        return self.patterns.get(pattern_id.0 as usize).cloned();
    }

    pub fn name(&self) -> &str {
        return &self.name;
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.name = name.into();
    }

    pub fn patterns(&self) -> &[Arc<OctamedicPattern>] {
        return &self.patterns;
    }

    // copy on write if the engine still holds it
    pub fn get_pattern_mut(&mut self, pattern_id: PatternId) -> Option<&mut OctamedicPattern> {
        return self
            .patterns
            .get_mut(pattern_id.0 as usize)
            .map(Arc::make_mut);
    }

    pub fn ensure_pattern(&mut self, pattern_id: PatternId) -> &mut OctamedicPattern {
        let index = pattern_id.0 as usize;
        if index >= self.patterns.len() {
            self.patterns
                .resize_with(index + 1, || Arc::new(OctamedicPattern::new()));
        }
        return Arc::make_mut(&mut self.patterns[index]);
    }

    pub fn sequence(&self) -> &[PatternId] {
        return &self.sequence;
    }

    pub fn sequence_mut(&mut self) -> &mut Vec<PatternId> {
        return &mut self.sequence;
    }

    pub fn tempo(&self) -> OctamedicTempo {
        return self.tempo;
    }

    pub fn flags(&self) -> OctamedicSongFlags {
        return self.flags;
    }

    pub fn tempo_mut(&mut self) -> &mut OctamedicTempo {
        return &mut self.tempo;
    }

    pub fn set_instrument(&mut self, id: InstrumentId, instrument: Option<OctamedicInstrument>) {
        if let Some(slot) = id.index().and_then(|index| self.instruments.get_mut(index)) {
            *slot = instrument;
        }
    }

    pub fn instrument(&self, id: InstrumentId) -> Option<&OctamedicInstrument> {
        return self.instruments.get(id.index()?)?.as_ref();
    }

    /// Everything but the instruments back to how a new song starts.
    pub fn clear(&mut self) {
        let instruments = std::mem::replace(&mut self.instruments, std::array::from_fn(|_| None));
        *self = Self {
            instruments,
            ..Self::new("")
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::note::OctamedicNote;

    #[test]
    fn clearing_keeps_the_instruments() {
        let mut song = OctamedicSong::new("test");
        song.instruments[0] = Some(OctamedicInstrument::new("kept"));
        song.patterns.push(Arc::new(OctamedicPattern::new()));
        song.sequence_mut().push(PatternId(1));
        song.clear();
        assert_eq!(song.patterns.len(), 1);
        assert_eq!(song.sequence(), [PatternId(0)]);
        assert_eq!(song.name(), "");
        assert!(song.instruments[0].is_some());
    }

    #[test]
    fn pattern_cow() {
        let mut song = OctamedicSong::new("test");
        let sent = song.get_shared_pattern(&PatternId(0)).unwrap();
        let pattern = song.get_pattern_mut(PatternId(0)).unwrap();
        pattern.lines[0].tracks[0].note = OctamedicNote { value: 13 };
        assert_eq!(sent.lines[0].tracks[0].note.value, 0);
        assert_eq!(
            song.get_pattern(&PatternId(0)).unwrap().lines[0].tracks[0]
                .note
                .value,
            13
        );
    }
}
