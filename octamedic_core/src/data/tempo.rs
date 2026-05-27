use octamed::utility::{
    bytes::{UByte, UWord},
    octamed_tempo::OctamedTempo,
};
#[derive(Clone, Copy)]
pub struct OctamedicTempo {
    pub primary_tempo: UWord,
    pub lines_per_beat: UByte,
    pub ticks_per_line: UByte,
    pub is_bpm_mode: bool,
}

impl OctamedicTempo {
    pub fn new() -> Self {
        return Self {
            primary_tempo: UWord(120),
            lines_per_beat: UByte(4),
            ticks_per_line: UByte(6),
            is_bpm_mode: true,
        };
    }
    pub fn from_octamed(tempo: &OctamedTempo) -> Self {
        let OctamedTempo {
            primary_tempo,
            lines_per_beat,
            ticks_per_line,
            is_bpm_mode,
        } = *tempo;
        return Self {
            primary_tempo,
            lines_per_beat,
            ticks_per_line,
            is_bpm_mode,
        };
    }
}
