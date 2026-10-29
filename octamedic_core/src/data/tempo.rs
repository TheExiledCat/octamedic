use octamed::utility::{
    bytes::{UByte, UWord, ValueMap},
    frequency::Frequency,
    octamed_tempo::OctamedTempo,
};

pub const MAX_BPM: u8 = 240;

/// What OctaMED v4 divides by the primary tempo for the CIA timer count, on any machine.

const TEMPO_TIMER_CONSTANT: u32 = 470_000;

/// The timer counts OctaMED v4 uses for primary tempos 1 to 10, which play like SoundTracker
/// speeds 1 to 10. Taken from the player as they are, including the step down at 9.

const COMPATIBILITY_COUNTS: [u16; 10] = [
    2417, 4833, 7250, 9666, 12083, 14500, 16916, 19332, 21436, 24163,
];

/// In BPM mode the timer interrupts four times per tick, and the player runs a tick on every
/// fourth.

const BPM_INTERRUPTS_PER_TICK: u32 = 4;

/// The clocks of a PAL or NTSC Amiga that playback timing depends on.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]

pub struct AmigaClock {
    /// The CIA timers' clock in Hz, the CPU clock divided by 10.
    pub e_clock: u32,
    /// What OctaMED v4 divides by BPM × lines per beat for the timer count in BPM mode: the
    /// E-clock times 2.5.
    pub bpm_timer_constant: u32,
    /// Paula's clock in Hz; a channel plays `paula_clock / period` bytes a second.
    pub paula_clock: u32,
}

impl AmigaClock {
    pub const PAL: Self = Self {
        e_clock: 709_379,
        bpm_timer_constant: 1_773_447,
        paula_clock: 3_546_895,
    };

    pub const NTSC: Self = Self {
        e_clock: 715_909,
        bpm_timer_constant: 1_789_772,
        paula_clock: 3_579_545,
    };
}

/// The 68000's `DIVU`: when the quotient doesn't fit in 16 bits, the division overflows and
/// leaves the dividend as it was, so its low word is what ends up in the timer.

fn divu(dividend: u32, divisor: u16) -> u16 {

    if divisor == 0 {

        // a 68000 traps here; OctaMED never divides by zero, so this only guards odd files
        return dividend as u16;
    }

    let quotient = dividend / divisor as u32;

    return u16::try_from(quotient).unwrap_or(dividend as u16);
}

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

    /// The count OctaMED v4 loads into the CIA timer for this tempo: a table for primary tempos
    /// 1 to 10 and `470000 / tempo` above that, or in BPM mode the machine's BPM constant
    /// divided by BPM × lines per beat.

    pub fn timer_count(&self, clock: AmigaClock) -> u16 {

        if self.is_bpm_mode {

            let divisor = self.primary_tempo.0 as u32 * self.lines_per_beat.0.max(1) as u32;

            // MULU's product is divided by its low word only
            return divu(clock.bpm_timer_constant, divisor as u16);
        }

        // OctaMED never stores tempo 0; its player would read past the table for it
        let tempo = self.primary_tempo.0.max(1);

        if tempo <= COMPATIBILITY_COUNTS.len() as u16 {

            return COMPATIBILITY_COUNTS[tempo as usize - 1];
        }

        return divu(TEMPO_TIMER_CONSTANT, tempo);
    }

    /// How many timer interrupts make up one player tick.

    pub fn interrupts_per_tick(&self) -> u32 {

        return if self.is_bpm_mode {

            BPM_INTERRUPTS_PER_TICK
        } else {

            1
        };
    }

    /// How long one tick lasts on the machine, in seconds.

    pub fn tick_seconds(&self, clock: AmigaClock) -> f64 {

        let clock_cycles = self.timer_count(clock) as u32 * self.interrupts_per_tick();

        return clock_cycles as f64 / clock.e_clock as f64;
    }

    /// Ticks per second on a PAL Amiga.

    pub fn get_tick_rate(&self) -> Frequency {

        return Frequency::hertz((1.0 / self.tick_seconds(AmigaClock::PAL)) as f32);
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

#[cfg(test)]

mod tests {

    use super::*;

    fn tempo(primary: u16) -> OctamedicTempo {

        let mut tempo = OctamedicTempo::new();

        tempo.is_bpm_mode = false;

        tempo.primary_tempo = UWord(primary);

        return tempo;
    }

    fn bpm(bpm: u16, lines_per_beat: u8) -> OctamedicTempo {

        let mut tempo = OctamedicTempo::new();

        tempo.primary_tempo = UWord(bpm);

        tempo.lines_per_beat = UByte(lines_per_beat);

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

        // 60 BPM with 8 lines a beat plays 8 lines a second at 6 ticks a line, a little faster
        // since the count is rounded down: 3694 instead of 3694.68
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
