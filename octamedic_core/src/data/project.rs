use std::sync::Arc;

use octamed::mmd::{
    conversion::{FromModule, ToModule},
    module::OctamedMMDBlockTable,
};

use crate::data::{
    instrument::{MAX_INSTRUMENT_VOLUME, OctamedicInstrument, OctamedicSample},
    pattern::OctamedicPattern,
    song::{OctamedicSong, SongId},
    tempo::OctamedicTempo,
    volume::OctamedicVolume,
};

pub struct OctamedicProject {
    name: String,
    songs: Vec<OctamedicSong>,
}

impl OctamedicProject {
    pub fn new() -> Self {

        return Self {
            name: String::new(),
            songs: vec![OctamedicSong::new("unnamed")],
        };
    }

    pub fn get_song(&self, id: SongId) -> Option<&OctamedicSong> {

        return self.songs.get(id.0 as usize);
    }
}

impl FromModule for OctamedicProject {
    fn from_module(module: &octamed::mmd::module::OctamedMMD) -> Self {

        let mut s = Self {
            name: String::new(),
            songs: vec![],
        };

        s.name = module
            .expansion_data
            .as_ref()
            .map(|e| e.song_name.clone())
            .unwrap_or(String::new());

        let mut song = OctamedicSong::new(&s.name);

        let patterns: Vec<OctamedicPattern> = match &module.block_table {
            OctamedMMDBlockTable::MMD0BlockTable {
                headers,
                blocks,
            } => blocks.iter().map(|b| OctamedicPattern::from(b)).collect(),
            OctamedMMDBlockTable::MMD1BlockTable {
                headers,
                blocks,
            } => blocks.iter().map(|b| OctamedicPattern::from(b)).collect(),
        };

        let mmd_song = &module.song;

        let names = module.expansion_data.as_ref().map(|e| &e.instrument_infos);

        for (index, slot) in song.instruments.iter_mut().enumerate() {

            let Some(Some(header)) = module.sample_table.headers.get(index) else {

                continue;
            };

            let info = &mmd_song.samples[index];

            let name = names
                .and_then(|names| names.get(index))
                .map_or(String::new(), |info| info.name.clone());

            let mut instrument = OctamedicInstrument::new(name);

            instrument.volume = info.sample_volume.0.min(MAX_INSTRUMENT_VOLUME);

            instrument.transpose = info.sample_transpose.0;

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

        song.sequence = mmd_song.player_sequence_list[..song_length].to_vec();

        song.song_length = mmd_song.song_length;

        song.block_count = mmd_song.block_count;

        song.tempo = OctamedicTempo::from_octamed(&mmd_song.get_tempo());

        song.global_transpose = mmd_song.global_transpose;

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
