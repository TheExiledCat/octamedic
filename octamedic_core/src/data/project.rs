use std::sync::Arc;

use octamed::mmd::{
    conversion::{FromModule, ToModule},
    module::OctamedMMDBlockTable,
};

use crate::data::{
    instrument::{
        MAX_FINETUNE, MAX_INSTRUMENT_VOLUME, MIN_FINETUNE, OctamedicInstrument, OctamedicSample,
    },
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
}

impl OctamedicProject {
    pub fn new() -> Self {
        return Self {
            name: String::new(),
            songs: vec![OctamedicSong::new("unnamed")],
            palette: OctamedColorPalette::DEFAULT,
        };
    }

    pub fn get_song(&self, id: SongId) -> Option<&OctamedicSong> {
        return self.songs.get(id.0 as usize);
    }

    pub fn get_song_mut(&mut self, id: SongId) -> Option<&mut OctamedicSong> {
        return self.songs.get_mut(id.0 as usize);
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
    fn from_module(module: &octamed::mmd::module::OctamedMMD) -> Self {
        let mut s = Self {
            name: String::new(),
            songs: vec![],
            palette: OctamedColorPalette::DEFAULT,
        };
        if let Some(expansion) = &module.expansion_data {
            if !expansion.header.rgb_table_ptr.is_null() {
                s.palette = OctamedColorPalette::from_octamed(&expansion.color_palette);
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
            if let Some(extension) = extensions.and_then(|e| e.get(index)) {
                instrument.finetune = extension.fine_tune.0.clamp(MIN_FINETUNE, MAX_FINETUNE);
                instrument.hold = extension.hold.0;
                instrument.decay = extension.decay.0;
            }
            instrument.sample = module
                .sample_table
                .samples
                .get(index)
                .and_then(Option::as_ref)
                .map(|bytes| {
                    let bytes: Vec<i8> = bytes.iter().map(|b| b.0).collect();
                    Arc::new(OctamedicSample::from_octamed(&bytes, header, info))
                });
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

impl ToModule for OctamedicProject {
    fn to_module(&mut self) -> octamed::mmd::module::OctamedMMD {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use octamed::mmd::parser::OctamedMMDParser;

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
