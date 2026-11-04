use mmd::mmd::module::{
    INSTRUMENT_NAME_LENGTH, OctamedMMD0ExpansionHeader, OctamedMMD0ExternalInstrument,
    OctamedMMD0Header, OctamedMMD0Song, OctamedMMDTrackLine, PALETTE_COLORS,
};

use crate::data::{
    instrument::{InstrumentKind, OctamedicInstrument, OctamedicSynth},
    pattern::OctamedicPattern,
    project::OctamedicProject,
    song::OctamedicSong,
};

// sizes from format.md that the mmd structs don't have as constants
const POINTER: usize = 4;
const SAMPLE_HEADER: usize = 6;
const SYNTH_HEADER: usize = 534;
const WAVEFORM_HEADER: usize = 2;
const BLOCK_HEADER: usize = 8;
const BLOCK_INFO: usize = 36;
const NOTATION_INFO: usize = 108;

impl OctamedicProject {
    /// Sample data and synth waveforms, which have to fit in chip memory on an Amiga.
    pub fn chip_bytes(&self) -> usize {
        return self.songs().iter().map(OctamedicSong::chip_bytes).sum();
    }

    /// About what it takes saved as an MMD1 module, with every song in it.
    pub fn module_bytes(&self) -> usize {
        let songs: usize = self.songs().iter().map(OctamedicSong::module_bytes).sum();
        let annotation = text_bytes(&self.annotation);
        let palette = PALETTE_COLORS * 2;
        return songs + annotation + palette + NOTATION_INFO;
    }
}

impl OctamedicSong {
    pub fn chip_bytes(&self) -> usize {
        return self
            .instruments
            .iter()
            .flatten()
            .map(OctamedicInstrument::data_bytes)
            .sum();
    }

    // each song is a whole module of its own in the file
    pub fn module_bytes(&self) -> usize {
        let blocks: usize = self.patterns().iter().map(|block| block_bytes(block)).sum();
        let used = self
            .instruments
            .iter()
            .rposition(Option::is_some)
            .map_or(0, |last| last + 1);
        let instruments: usize = self
            .instruments
            .iter()
            .flatten()
            .map(instrument_bytes)
            .sum();
        let instrument_extras =
            used * (POINTER + OctamedMMD0ExternalInstrument::V4_SIZE + INSTRUMENT_NAME_LENGTH);
        return OctamedMMD0Header::SIZE
            + OctamedMMD0Song::SIZE
            + OctamedMMD0ExpansionHeader::SIZE
            + text_bytes(self.name())
            + self.patterns().len() * POINTER
            + blocks
            + instrument_extras
            + instruments;
    }
}

fn block_bytes(block: &OctamedicPattern) -> usize {
    let lines = block.line_count();
    let notes = lines * block.track_count() * OctamedMMDTrackLine::MMD1_SIZE;
    let highlighted = block.lines.iter().any(|line| line.highlighted);
    let info = if highlighted || !block.name.is_empty() {
        BLOCK_INFO + lines.div_ceil(32) * 4 + text_bytes(&block.name)
    } else {
        0
    };
    return BLOCK_HEADER + notes + info;
}

fn instrument_bytes(instrument: &OctamedicInstrument) -> usize {
    return match &instrument.kind {
        InstrumentKind::Empty => 0,
        InstrumentKind::Sample(sample) => SAMPLE_HEADER + sample.data.len(),
        InstrumentKind::Synth(synth) => synth_bytes(synth),
        InstrumentKind::Hybrid(synth, sample) => {
            synth_bytes(synth) + SAMPLE_HEADER + sample.data.len()
        }
    };
}

fn synth_bytes(synth: &OctamedicSynth) -> usize {
    let waveforms: usize = synth
        .waveforms
        .iter()
        .map(|waveform| WAVEFORM_HEADER + waveform.len())
        .sum();
    return SYNTH_HEADER + waveforms;
}

fn text_bytes(text: &str) -> usize {
    return if text.is_empty() { 0 } else { text.len() + 1 };
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::data::{
        instrument::{InstrumentId, OctamedicSample, SampleOctaves},
        pattern::PatternId,
        song::SongId,
    };

    fn sample(bytes: usize) -> OctamedicInstrument {
        let mut instrument = OctamedicInstrument::new("sample");
        instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data: vec![0; bytes],
            repeat: None,
            octaves: SampleOctaves::One,
        }));
        return instrument;
    }

    #[test]
    fn samples_go_in_chip_memory_and_the_module() {
        let mut project = OctamedicProject::new();
        let (chip, module) = (project.chip_bytes(), project.module_bytes());
        let song = project.get_song_mut(SongId(0)).unwrap();
        song.set_instrument(InstrumentId(3), Some(sample(1000)));
        assert_eq!(project.chip_bytes(), chip + 1000);
        let instrument_extras =
            3 * (POINTER + OctamedMMD0ExternalInstrument::V4_SIZE + INSTRUMENT_NAME_LENGTH);
        assert_eq!(
            project.module_bytes(),
            module + SAMPLE_HEADER + 1000 + instrument_extras
        );
    }

    #[test]
    fn blocks_take_four_bytes_a_note() {
        let mut song = OctamedicSong::new("");
        let before = song.module_bytes();
        let block = song.ensure_pattern(PatternId(1));
        let notes = block.line_count() * block.track_count();
        assert_eq!(
            song.module_bytes() - before,
            POINTER + BLOCK_HEADER + notes * 4
        );
    }

    #[test]
    fn a_named_block_has_block_info() {
        let mut block = OctamedicPattern::new();
        let plain = block_bytes(&block);
        block.name = "intro".to_owned();
        assert_eq!(block_bytes(&block), plain + BLOCK_INFO + 2 * 4 + 6);
    }
}
