use std::sync::Arc;

use mmd::mmd::module::SONG_TRACKS;

use crate::data::{
    command::SET_VOLUME,
    instrument::{InstrumentId, MAX_INSTRUMENTS, OctamedicInstrument},
    pattern::{OctamedicPattern, PatternId},
    tempo::OctamedicTempo,
    volume::{MAX_VOLUME, OctamedicVolume, set_volume_argument, set_volume_value},
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

    /// Rewrites every volume command for VOL's HEX (`hex`) or DEC, so each plays at the same
    /// volume it did in the other. Returns the blocks that changed. Doesn't change the song's flag.
    pub fn convert_volume_commands(&mut self, hex: bool) -> Vec<PatternId> {
        let mut changed = Vec::new();
        for (index, pattern) in self.patterns.iter_mut().enumerate() {
            let has_set_volume = pattern
                .lines
                .iter()
                .flat_map(|line| &line.tracks)
                .any(|cell| cell.command_id.0 == SET_VOLUME);
            if !has_set_volume {
                continue;
            }
            let cells = Arc::make_mut(pattern)
                .lines
                .iter_mut()
                .flat_map(|line| &mut line.tracks)
                .filter(|cell| cell.command_id.0 == SET_VOLUME);
            for cell in cells {
                let volume = set_volume_value(cell.command_value, !hex);
                cell.command_value = set_volume_argument(volume, hex);
            }
            changed.push(PatternId(index as u8));
        }
        return changed;
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

    /// Inserts `pattern` as block `index`, shifting every later block (and the sequence entries
    /// pointing to them) up by one, and returns its new id.
    pub fn insert_pattern(&mut self, index: usize, pattern: OctamedicPattern) -> PatternId {
        let index = index.min(self.patterns.len());
        self.patterns.insert(index, Arc::new(pattern));
        for id in &mut self.sequence {
            if id.0 as usize >= index {
                id.0 += 1;
            }
        }
        return PatternId(index as u8);
    }

    /// Removes block `index`, shifting every later block (and the sequence entries pointing to
    /// them) down by one. Entries that played it are dropped, leaving block 0 if nothing's left.
    pub fn remove_pattern(&mut self, index: usize) {
        if index >= self.patterns.len() {
            return;
        }
        self.patterns.remove(index);
        self.sequence.retain(|id| id.0 as usize != index);
        for id in &mut self.sequence {
            if id.0 as usize > index {
                id.0 -= 1;
            }
        }
        if self.sequence.is_empty() {
            self.sequence.push(PatternId(0));
        }
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
    use crate::data::{command::CommandId, note::OctamedicNote};

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

    // blocks 0, 1, 2, sequence 0, 1, 2, 0
    fn three_block_song() -> OctamedicSong {
        let mut song = OctamedicSong::new("test");
        song.patterns.push(Arc::new(OctamedicPattern::new()));
        song.patterns.push(Arc::new(OctamedicPattern::new()));
        *song.sequence_mut() = vec![PatternId(0), PatternId(1), PatternId(2), PatternId(0)];
        return song;
    }

    #[test]
    fn insert_pattern_shifts_later_blocks_and_sequence() {
        let mut song = three_block_song();
        let id = song.insert_pattern(1, OctamedicPattern::new());
        assert_eq!(id, PatternId(1));
        assert_eq!(song.patterns.len(), 4);
        // the old 1 and 2 moved to 2 and 3; the old 0 (used twice) stayed put
        assert_eq!(
            song.sequence(),
            [PatternId(0), PatternId(2), PatternId(3), PatternId(0)]
        );
    }

    #[test]
    fn insert_pattern_at_the_end_appends() {
        let mut song = three_block_song();
        let id = song.insert_pattern(song.patterns.len(), OctamedicPattern::new());
        assert_eq!(id, PatternId(3));
        assert_eq!(
            song.sequence(),
            [PatternId(0), PatternId(1), PatternId(2), PatternId(0)]
        );
    }

    #[test]
    fn remove_pattern_shifts_later_blocks_and_drops_its_entries() {
        let mut song = three_block_song();
        song.remove_pattern(1);
        assert_eq!(song.patterns.len(), 2);
        // the entry that played block 1 is gone, the old 2 now being 1
        assert_eq!(song.sequence(), [PatternId(0), PatternId(1), PatternId(0)]);
    }

    #[test]
    fn remove_pattern_keeps_the_sequence_playing_something() {
        let mut song = three_block_song();
        *song.sequence_mut() = vec![PatternId(2), PatternId(2)];
        song.remove_pattern(2);
        assert_eq!(song.patterns.len(), 2);
        assert_eq!(song.sequence(), [PatternId(0)]);
    }

    #[test]
    fn converting_volume_commands() {
        let mut song = OctamedicSong::new("test");
        song.patterns.push(Arc::new(OctamedicPattern::new()));
        let cell = &mut song.ensure_pattern(PatternId(0)).lines[0].tracks[0];
        cell.command_id = CommandId(SET_VOLUME);
        cell.command_value = 0x40;
        let other = &mut song.ensure_pattern(PatternId(0)).lines[1].tracks[0];
        other.command_id = CommandId(0x0F);
        other.command_value = 0x40;
        // decimal 40 is hex 28, and only volume commands change. block 1 has none
        assert_eq!(song.convert_volume_commands(true), [PatternId(0)]);
        let lines = &song.patterns[0].lines;
        assert_eq!(lines[0].tracks[0].command_value, 0x28);
        assert_eq!(lines[1].tracks[0].command_value, 0x40);
        song.convert_volume_commands(false);
        assert_eq!(song.patterns[0].lines[0].tracks[0].command_value, 0x40);
    }
}
