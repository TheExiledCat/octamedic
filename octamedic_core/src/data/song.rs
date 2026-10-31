use std::sync::Arc;

use octamed::mmd::module::SONG_TRACKS;

use crate::data::{
    instrument::{InstrumentId, MAX_INSTRUMENTS, OctamedicInstrument},
    pattern::{OctamedicPattern, PatternId},
    tempo::OctamedicTempo,
    volume::{MAX_VOLUME, OctamedicVolume},
};

#[derive(Clone, Copy)]
pub struct SongId(pub u8);

/// The song flags that change how it plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OctamedicSongFlags {
    /// The Amiga's audio filter starts on.
    pub filter_on: bool,
    /// The volume command's argument is hex rather than decimal.
    pub volumes_hex: bool,
    /// SoundTracker/NoiseTracker/ProTracker compatible slides.
    pub st_slides: bool,
    /// A 5-8 channel song, played through the Octa mixer.
    pub eight_channel: bool,
}

pub struct OctamedicSong {
    pub(crate) name: String,
    pub(crate) instruments: [Option<OctamedicInstrument>; MAX_INSTRUMENTS],
    pub(crate) sequence: Vec<PatternId>,
    pub(crate) tempo: OctamedicTempo,
    pub(crate) flags: OctamedicSongFlags,
    pub(crate) global_transpose: i8,
    pub(crate) track_volumes: [OctamedicVolume; SONG_TRACKS],
    pub(crate) master_volume: OctamedicVolume,
    pub(crate) patterns: Vec<Arc<OctamedicPattern>>,
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

    /// For handing to the engine without copying it.
    pub fn get_shared_pattern(&self, pattern_id: &PatternId) -> Option<Arc<OctamedicPattern>> {
        return self.patterns.get(pattern_id.0 as usize).cloned();
    }

    pub fn name(&self) -> &str {
        return &self.name;
    }

    pub fn patterns(&self) -> &[Arc<OctamedicPattern>] {
        return &self.patterns;
    }

    /// Copies the pattern first if the engine still has it.
    pub fn get_pattern_mut(&mut self, pattern_id: PatternId) -> Option<&mut OctamedicPattern> {
        return self
            .patterns
            .get_mut(pattern_id.0 as usize)
            .map(Arc::make_mut);
    }

    // creates empty patterns up to `pattern_id` if the song has fewer
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::note::OctamedicNote;

    #[test]
    fn editing_a_pattern_the_engine_has_leaves_its_copy_alone() {
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
