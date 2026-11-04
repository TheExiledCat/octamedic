use std::sync::Arc;

use mmd::mmd::{
    conversion::{FromModule, ToModule},
    module::{
        AMIGA_CHANNELS, OctamedMMD, OctamedMMD0Sample, OctamedMMD0SampleHeader,
        OctamedMMDBlockTable,
    },
};

use crate::data::{
    instrument::{
        InstrumentKind, MAX_FINETUNE, MAX_INSTRUMENT_VOLUME, MIN_FINETUNE, OctamedicInstrument,
        OctamedicSample, OctamedicSynth, SampleOctaves,
    },
    notation::OctamedicNotation,
    palette::OctamedColorPalette,
    pattern::{OctamedicPattern, PatternId},
    song::{OctamedicSong, OctamedicSongFlags, SongId},
    tempo::OctamedicTempo,
    volume::OctamedicVolume,
};

pub struct OctamedicProject {
    name: String,
    songs: Vec<OctamedicSong>,
    palette: OctamedColorPalette,
    pub annotation: String,
    /// Channels split into two in 5-8 channel mode.
    pub split_channels: [bool; AMIGA_CHANNELS],
    pub notation: OctamedicNotation,
}

impl OctamedicProject {
    pub fn new() -> Self {
        return Self {
            name: String::new(),
            songs: vec![OctamedicSong::new("unnamed")],
            palette: OctamedColorPalette::DEFAULT,
            annotation: String::new(),
            split_channels: [true; AMIGA_CHANNELS],
            notation: OctamedicNotation::default(),
        };
    }

    pub fn get_song(&self, id: SongId) -> Option<&OctamedicSong> {
        return self.songs.get(id.0 as usize);
    }

    pub fn get_song_mut(&mut self, id: SongId) -> Option<&mut OctamedicSong> {
        return self.songs.get_mut(id.0 as usize);
    }

    pub fn songs(&self) -> &[OctamedicSong] {
        return &self.songs;
    }

    pub fn song_count(&self) -> usize {
        return self.songs.len();
    }

    pub fn palette(&self) -> OctamedColorPalette {
        return self.palette;
    }
}

impl Default for OctamedicProject {
    fn default() -> Self {
        return Self::new();
    }
}

impl FromModule for OctamedicProject {
    fn from_module(module: &mmd::mmd::module::OctamedMMD) -> Self {
        let mut s = Self {
            songs: vec![],
            ..Self::new()
        };
        if let Some(expansion) = &module.expansion_data {
            if !expansion.header.rgb_table_ptr.is_null() {
                s.palette = OctamedColorPalette::from_octamed(&expansion.color_palette);
            }
            s.annotation = expansion.annotation.clone();
            // a byte that isn't 0 means not split
            s.split_channels = expansion.header.channel_split.map(|byte| byte.0 == 0);
            if !expansion.header.notation_info_ptr.is_null() {
                s.notation = OctamedicNotation::from_octamed(&expansion.notation_info);
            }
        }
        s.name = module
            .expansion_data
            .as_ref()
            .map(|e| e.song_name.clone())
            .unwrap_or_default();
        let mut song = OctamedicSong::new(&s.name);
        let patterns: Vec<OctamedicPattern> = match &module.block_table {
            OctamedMMDBlockTable::MMD0BlockTable { headers: _, blocks } => {
                blocks.iter().map(OctamedicPattern::from).collect()
            }
            OctamedMMDBlockTable::MMD1BlockTable { headers: _, blocks } => {
                blocks.iter().map(OctamedicPattern::from).collect()
            }
        };
        let patterns = patterns.into_iter().map(Arc::new).collect();
        let mmd_song = &module.song;
        let names = module.expansion_data.as_ref().map(|e| &e.instrument_infos);
        let extensions = module
            .expansion_data
            .as_ref()
            .map(|e| &e.external_instruments);
        for (index, slot) in song.instruments.iter_mut().enumerate() {
            let Some(Some(header)) = module.sample_table.headers.get(index) else {
                continue;
            };
            let info = &mmd_song.samples[index];
            let name = names
                .and_then(|names| names.get(index))
                // NUL padded
                .map_or(String::new(), |info| {
                    info.name.split('\0').next().unwrap_or_default().to_owned()
                });
            let mut instrument = OctamedicInstrument::new(name);
            instrument.volume = info.sample_volume.0.min(MAX_INSTRUMENT_VOLUME);
            instrument.transpose = info.sample_transpose.0;
            instrument.midi.channel = info.midi_channel.0;
            instrument.midi.preset = info.midi_preset.0;
            if let Some(extension) = extensions.and_then(|e| e.get(index)) {
                instrument.finetune = extension.fine_tune.0.clamp(MIN_FINETUNE, MAX_FINETUNE);
                instrument.hold = extension.hold.0;
                instrument.decay = extension.decay.0;
                instrument.midi.suppress_note_off = extension.supress_midi_off.0 != 0;
            }
            instrument.kind = instrument_kind(module, index, header, info);
            *slot = Some(instrument);
        }
        let song_length =
            (mmd_song.song_length.0 as usize).min(mmd_song.player_sequence_list.len());
        *song.sequence_mut() = mmd_song.player_sequence_list[..song_length]
            .iter()
            .map(|&id| PatternId::from(id))
            .collect();
        *song.tempo_mut() = OctamedicTempo::from_octamed(&mmd_song.get_tempo());
        let flags = &mmd_song.flags;
        song.flags = OctamedicSongFlags {
            filter_on: flags.filter_is_on(),
            volumes_hex: flags.volumes_are_hex(),
            st_slides: flags.use_st_sliding(),
            eight_channel: flags.is_8_channels(),
            jumping: flags.jumping_is_on(),
            jump_every_8th_line: flags.jump_every_eight_lines(),
            hq_v2_compatible: flags.is_hq_v2_compatability(),
            mixing: flags.mixing_enabled(),
        };
        song.global_transpose = mmd_song.global_transpose.0;
        song.master_volume = OctamedicVolume::new(mmd_song.master_volume.0);
        for (volume, mmd_volume) in song.track_volumes.iter_mut().zip(mmd_song.track_volumes) {
            *volume = OctamedicVolume::new(mmd_volume.0);
        }
        //single song for now
        song.patterns = patterns;
        s.songs = vec![song];
        return s;
    }
}

fn instrument_kind(
    module: &OctamedMMD,
    index: usize,
    header: &OctamedMMD0SampleHeader,
    info: &OctamedMMD0Sample,
) -> InstrumentKind {
    let table = &module.sample_table;
    if let Some(Some(synth)) = table.synths.get(index) {
        let synth_part = Arc::new(OctamedicSynth::from_octamed(synth));
        return match &synth.hybrid_sample {
            Some(bytes) => {
                let sample = OctamedicSample {
                    data: bytes.iter().map(|b| b.0).collect(),
                    repeat: None,
                    octaves: SampleOctaves::One,
                };
                InstrumentKind::Hybrid(synth_part, Arc::new(sample))
            }
            None => InstrumentKind::Synth(synth_part),
        };
    }
    let Some(Some(bytes)) = table.samples.get(index) else {
        return InstrumentKind::Empty;
    };
    let bytes: Vec<i8> = bytes.iter().map(|b| b.0).collect();
    return InstrumentKind::Sample(Arc::new(OctamedicSample::from_octamed(
        &bytes, header, info,
    )));
}

impl ToModule for OctamedicProject {
    fn to_module(&mut self) -> mmd::mmd::module::OctamedMMD {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use mmd::mmd::parser::OctamedMMDParser;

    use super::*;

    fn example(name: &str) -> OctamedicProject {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../example_meds")
            .join(name);
        let module = OctamedMMDParser::new().parse_file(&path).unwrap().remove(0);
        return OctamedicProject::from_module(&module);
    }

    #[test]
    fn song_flags() {
        let project = example("example.mmd0");
        let flags = project.get_song(SongId(0)).unwrap().flags();
        assert_eq!(
            flags,
            OctamedicSongFlags {
                eight_channel: true,
                ..OctamedicSongFlags::default()
            }
        );
    }

    #[test]
    fn finetune() {
        let project = example("example.mmd1");
        let song = project.get_song(SongId(0)).unwrap();
        let finetune = |index: usize| song.instruments[index].as_ref().unwrap().finetune;
        assert_eq!((finetune(0), finetune(1), finetune(8)), (-1, 0, 1));
    }
}
