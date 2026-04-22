use std::fmt::Display;

use crate::{
    mmd0::module::OctamedMMD0Song,
    utility::{ bytes::{ UByte, UWord, ValueMap }, frequency::Frequency },
};

const BPM_COMPATIBILITY_VALUES: &[u8] = &[195, 97, 65, 49, 39, 32, 28, 24, 22, 20];
const DEFAULT_TICKS_PER_LINE: u8 = 6;
#[derive(Clone, Copy)]
pub struct OctamedTempo {
    primary_tempo: UWord,
    lines_per_beat: UByte,
    ticks_per_line: UByte,
    is_bpm_mode: bool,
}

impl OctamedTempo {
    pub fn from_song(song: &OctamedMMD0Song) -> Self {
        let primary_tempo = song.primary_tempo;
        let ticks_per_line = song.secondary_tempo;
        let is_bpm_mode = song.flags.is_bpm_mode();
        let lines_per_beat = song.flags.bpm_beat_length();
        return Self { primary_tempo, ticks_per_line, is_bpm_mode, lines_per_beat };
    }
    pub fn is_bpm_mode(&self) -> bool {
        return self.is_bpm_mode;
    }
    pub fn get_tick_rate(mut self) -> Frequency {
        if self.is_bpm_mode {
            return Frequency::hertz(
                ((self.primary_tempo.0 as f32) * (self.lines_per_beat.0 as f32)) / 10.0
            );
        }

        if self.primary_tempo.0 <= 10 {
            self.primary_tempo = UWord(
                BPM_COMPATIBILITY_VALUES[(self.primary_tempo.0 - 1) as usize] as u16
            );
        }
        return Frequency::hertz(1.0 / ((0.474326 / (self.primary_tempo.0 as f32)) * 1.3968255));
    }

    pub fn set_tempo(&mut self, value: UWord) {
        self.primary_tempo = value.map(|v| v.clamp(1, 240));
    }
    pub fn set_lines_per_beat(&mut self, value: UByte) {
        self.lines_per_beat = value.map(|v| v.clamp(1, 32));
    }
    pub fn set_ticks_per_line(&mut self, value: UByte) {
        self.ticks_per_line = value.map(|v| v.clamp(1, 32));
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
