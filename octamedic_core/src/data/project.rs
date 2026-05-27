use octamed::mmd::{
    conversion::{FromModule, ToModule},
    module::OctamedMMDBlockTable,
};

use crate::data::{
    pattern::OctamedicPattern,
    song::{OctamedicSong, SongId},
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
            OctamedMMDBlockTable::MMD0BlockTable { headers, blocks } => {
                blocks.iter().map(|b| OctamedicPattern::from(b)).collect()
            }
            OctamedMMDBlockTable::MMD1BlockTable { headers, blocks } => {
                blocks.iter().map(|b| OctamedicPattern::from(b)).collect()
            }
        };
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
