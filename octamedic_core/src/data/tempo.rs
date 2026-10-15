use octamed::utility::{
    bytes::{UByte, UWord, ValueMap},
    frequency::Frequency,
    octamed_tempo::OctamedTempo,
};

const BPM_COMPATIBILITY_VALUES: &[u8] = &[195, 97, 65, 49, 39, 32, 28, 24, 22, 20];

const TEMPO_TICK_FACTOR: f32 = 0.474326 * 1.3968255;

pub const MAX_BPM: u8 = 240;

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


    pub fn as_bpm(&self) -> UWord {

        if self.is_bpm_mode {

            return self.primary_tempo;
        }

        let lines_per_beat = self.lines_per_beat.0.max(1) as f32;

        let bpm = self.get_tick_rate().as_hertz() * 10.0 / lines_per_beat;

        return UWord(bpm.round().clamp(1.0, u16::MAX as f32) as u16);
    }

    // Source: https://github.com/neumatho/NostalgicPlayer/blob/main/Source/Agents/Players/OctaMed/Implementation/Mixer.cs Line 150, thank you thomas neumann :)
    pub fn get_tick_rate(&self) -> Frequency {

        if self.is_bpm_mode {

            return Frequency::hertz(
                ((self.primary_tempo.0 as f32) * (self.lines_per_beat.0 as f32)) / 10.0,
            );
        }

        if self.primary_tempo.0 <= 10 {

            todo!();

            self.primary_tempo =
                UWord(BPM_COMPATIBILITY_VALUES[(self.primary_tempo.0 - 1) as usize] as u16);
        }

        return Frequency::hertz(1.0 / ((0.474326 / (self.primary_tempo.0 as f32)) * 1.3968255));
    }

    pub fn set_tempo(&mut self, value: UWord) {

        self.primary_tempo = value.map(|v| v.clamp(1, MAX_BPM as u16));
    }

    pub fn set_lines_per_beat(&mut self, value: UByte) {

        self.lines_per_beat = value.map(|v| v.clamp(1, 32));
    }

    pub fn set_ticks_per_line(&mut self, value: UByte) {

        self.ticks_per_line = value.map(|v| v.clamp(1, 32));
    }
}
