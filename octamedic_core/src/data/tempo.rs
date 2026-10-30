use octamed::utility::{frequency::Frequency, octamed_tempo::OctamedTempo};

use crate::amiga::{clock::AmigaClock, m68k::divu};

pub const MAX_BPM: u8 = 240;

pub const MAX_LINES_PER_BEAT: u8 = 32;
pub const MAX_TICKS_PER_LINE: u8 = 32;

const DEFAULT_TEMPO: u16 = 120;
const DEFAULT_LINES_PER_BEAT: u8 = 4;
const DEFAULT_TICKS_PER_LINE: u8 = 6;

const SECONDS_PER_MINUTE: f32 = 60.0;

/// Divided by the primary tempo for the timer count, on PAL and NTSC alike.
const TEMPO_TIMER_CONSTANT: u32 = 470_000;

/// Timer counts for primary tempos 1-10, which play like SoundTracker speeds. As in the original,
/// including the dip at 9.
const COMPATIBILITY_COUNTS: [u16; 10] = [
    2417, 4833, 7250, 9666, 12083, 14500, 16916, 19332, 21436, 24163,
];

/// BPM mode divides this by BPM × lines per beat: the E-clock × 2.5, rounded down.
fn bpm_timer_constant(clock: AmigaClock) -> u32 {
    const TIMES_TWO_AND_A_HALF: (u32, u32) = (5, 2);
    return clock.e_clock * TIMES_TWO_AND_A_HALF.0 / TIMES_TWO_AND_A_HALF.1;
}

/// BPM mode runs a tick on every 4th timer interrupt.
const BPM_INTERRUPTS_PER_TICK: u32 = 4;

#[derive(Clone, Copy)]
pub struct OctamedicTempo {
    pub primary_tempo: u16,
    pub lines_per_beat: u8,
    pub ticks_per_line: u8,
    pub is_bpm_mode: bool,
}

impl OctamedicTempo {
    pub fn new() -> Self {
        return Self {
            primary_tempo: DEFAULT_TEMPO,
            lines_per_beat: DEFAULT_LINES_PER_BEAT,
            ticks_per_line: DEFAULT_TICKS_PER_LINE,
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
            primary_tempo: primary_tempo.0,
            lines_per_beat: lines_per_beat.0,
            ticks_per_line: ticks_per_line.0,
            is_bpm_mode,
        };
    }

    pub fn as_bpm(&self) -> u16 {
        if self.is_bpm_mode {
            return self.primary_tempo;
        }
        let lines_per_beat = self.lines_per_beat.max(1) as f32;
        let lines_per_second = self.get_tick_rate().as_hertz() / self.ticks_per_line.max(1) as f32;
        let bpm = lines_per_second * SECONDS_PER_MINUTE / lines_per_beat;
        return bpm.round().clamp(1.0, u16::MAX as f32) as u16;
    }

    /// The CIA timer count OctaMED v4 uses for this tempo.
    pub fn timer_count(&self, clock: AmigaClock) -> u16 {
        if self.is_bpm_mode {
            let divisor = self.primary_tempo as u32 * self.lines_per_beat.max(1) as u32;
            // DIVU only takes MULU's low word
            return divu(bpm_timer_constant(clock), divisor as u16);
        }
        // never stored by OctaMED; the original would read past its table
        let tempo = self.primary_tempo.max(1);
        if tempo <= COMPATIBILITY_COUNTS.len() as u16 {
            return COMPATIBILITY_COUNTS[tempo as usize - 1];
        }
        return divu(TEMPO_TIMER_CONSTANT, tempo);
    }

    pub fn interrupts_per_tick(&self) -> u32 {
        return if self.is_bpm_mode {
            BPM_INTERRUPTS_PER_TICK
        } else {
            1
        };
    }

    pub fn tick_seconds(&self, clock: AmigaClock) -> f64 {
        let clock_cycles = self.timer_count(clock) as u32 * self.interrupts_per_tick();
        return clock_cycles as f64 / clock.e_clock as f64;
    }

    /// On a PAL Amiga.
    pub fn get_tick_rate(&self) -> Frequency {
        return Frequency::hertz((1.0 / self.tick_seconds(AmigaClock::PAL)) as f32);
    }

    pub fn set_tempo(&mut self, value: u16) {
        self.primary_tempo = value.clamp(1, MAX_BPM as u16);
    }

    pub fn set_lines_per_beat(&mut self, value: u8) {
        self.lines_per_beat = value.clamp(1, MAX_LINES_PER_BEAT);
    }

    pub fn set_ticks_per_line(&mut self, value: u8) {
        self.ticks_per_line = value.clamp(1, MAX_TICKS_PER_LINE);
    }
}

impl Default for OctamedicTempo {
    fn default() -> Self {
        return Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempo(primary: u16) -> OctamedicTempo {
        let mut tempo = OctamedicTempo::new();
        tempo.is_bpm_mode = false;
        tempo.primary_tempo = primary;
        return tempo;
    }

    fn bpm(bpm: u16, lines_per_beat: u8) -> OctamedicTempo {
        let mut tempo = OctamedicTempo::new();
        tempo.primary_tempo = bpm;
        tempo.lines_per_beat = lines_per_beat;
        return tempo;
    }

    #[test]
    fn tempos_1_to_10_use_the_compatibility_table() {
        let counts: Vec<u16> = (1..=10)
            .map(|t| tempo(t).timer_count(AmigaClock::PAL))
            .collect();
        assert_eq!(counts, COMPATIBILITY_COUNTS);
    }

    #[test]
    fn higher_tempos_divide_470000_dropping_the_remainder() {
        assert_eq!(tempo(11).timer_count(AmigaClock::PAL), 42727);
        assert_eq!(tempo(33).timer_count(AmigaClock::PAL), 14242);
        assert_eq!(tempo(240).timer_count(AmigaClock::PAL), 1958);
    }

    #[test]
    fn tempo_mode_uses_the_same_count_on_ntsc() {
        assert_eq!(
            tempo(33).timer_count(AmigaClock::NTSC),
            tempo(33).timer_count(AmigaClock::PAL)
        );
        assert!(tempo(33).tick_seconds(AmigaClock::NTSC) < tempo(33).tick_seconds(AmigaClock::PAL));
    }

    #[test]
    fn a_tempo_of_0_plays_like_1() {
        assert_eq!(tempo(0).timer_count(AmigaClock::PAL), 2417);
    }

    #[test]
    fn bpm_mode_divides_the_machines_constant_by_bpm_times_lines_per_beat() {
        // 1773447 / 500 and 1789772 / 500
        assert_eq!(bpm(125, 4).timer_count(AmigaClock::PAL), 3546);
        assert_eq!(bpm(125, 4).timer_count(AmigaClock::NTSC), 3579);
    }

    #[test]
    fn bpm_mode_ticks_every_fourth_interrupt() {
        let tempo = bpm(60, 8);
        assert_eq!(tempo.interrupts_per_tick(), 4);
        // 8 lines a second, a little faster since the count rounds down to 3694
        let lines_per_second = 1.0 / (tempo.tick_seconds(AmigaClock::PAL) * 6.0);
        assert!(
            (lines_per_second - 8.0015).abs() < 0.0001,
            "{lines_per_second}"
        );
    }

    #[test]
    fn very_slow_bpm_overflows_into_the_constants_low_word() {
        assert_eq!(bpm(28, 1).timer_count(AmigaClock::PAL), 63337);
        // 1773447 is $1B0F87
        assert_eq!(bpm(27, 1).timer_count(AmigaClock::PAL), 0x0F87);
    }

    #[test]
    fn the_tick_rate_follows_the_count() {
        let rate = tempo(33).get_tick_rate().as_hertz() as f64;
        assert!((rate - 709379.0 / 14242.0).abs() < 0.01, "{rate}");
    }
}
