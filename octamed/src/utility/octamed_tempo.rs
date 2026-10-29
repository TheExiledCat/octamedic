use std::fmt::Display;

use crate::{
    mmd::module::OctamedMMD0Song,
    utility::bytes::{UByte, UWord},
};

/// A song's tempo fields as stored. Timing from them is octamedic_core's job.
#[derive(Clone, Copy)]
pub struct OctamedTempo {
    pub primary_tempo: UWord,
    pub lines_per_beat: UByte,
    pub ticks_per_line: UByte,
    pub is_bpm_mode: bool,
}

impl OctamedTempo {
    pub fn from_song(song: &OctamedMMD0Song) -> Self {
        let primary_tempo = song.primary_tempo;
        let ticks_per_line = song.secondary_tempo;
        let is_bpm_mode = song.flags.is_bpm_mode();
        let lines_per_beat = song.flags.bpm_beat_length();
        return Self {
            primary_tempo,
            ticks_per_line,
            is_bpm_mode,
            lines_per_beat,
        };
    }
}

impl Display for OctamedTempo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:03}/", self.primary_tempo)?;
        if self.is_bpm_mode {
            return write!(f, "{:02}", self.lines_per_beat.0);
        } else {
            return write!(f, "{:02X}", self.ticks_per_line.0);
        }
    }
}
