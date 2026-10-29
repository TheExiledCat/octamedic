/// ProTracker's periods for C-1 to B-3.
const PERIODS: [u16; 36] = [
    856, 808, 762, 720, 678, 640, 604, 570, 538, 508, 480, 453, //
    428, 404, 381, 360, 339, 320, 302, 285, 269, 254, 240, 226, //
    214, 202, 190, 180, 170, 160, 151, 143, 135, 127, 120, 113,
];

/// OctaMED's note number of C-1.
const FIRST_NOTE: i32 = 1;

const NOTES_PER_OCTAVE: i32 = 12;

/// Octaves outside the table double or halve the nearest one's periods, until OctaMED's own
/// table is known. `None` for notes too high to have a period.
pub fn note_period(note: i32) -> Option<u16> {
    let index = note - FIRST_NOTE;
    let last = PERIODS.len() as i32 - 1;
    if (0..=last).contains(&index) {
        return Some(PERIODS[index as usize]);
    }
    if index < 0 {
        let octaves_below = (-index + NOTES_PER_OCTAVE - 1) / NOTES_PER_OCTAVE;
        let in_table = index + octaves_below * NOTES_PER_OCTAVE;
        let period = (PERIODS[in_table as usize] as u32).checked_shl(octaves_below as u32)?;
        return u16::try_from(period).ok();
    }
    let octaves_above = (index - last + NOTES_PER_OCTAVE - 1) / NOTES_PER_OCTAVE;
    let in_table = index - octaves_above * NOTES_PER_OCTAVE;
    let period = PERIODS[in_table as usize].checked_shr(octaves_above as u32)?;
    return (period > 0).then_some(period);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_octaves_use_protrackers_periods() {
        assert_eq!(note_period(1), Some(856));
        assert_eq!(note_period(13), Some(428));
        assert_eq!(note_period(25), Some(214));
        assert_eq!(note_period(36), Some(113));
    }

    #[test]
    fn notes_above_the_table_halve_the_period_per_octave() {
        assert_eq!(note_period(37), Some(107));
        assert_eq!(note_period(49), Some(53));
        assert_eq!(note_period(48), Some(56));
    }

    #[test]
    fn notes_below_the_table_double_the_period_per_octave() {
        assert_eq!(note_period(0), Some(906));
        assert_eq!(note_period(-11), Some(1712));
        assert_eq!(note_period(-23), Some(3424));
    }

    #[test]
    fn notes_far_below_the_table_run_out_of_period_bits() {
        assert_eq!(note_period(-200), None);
    }

    #[test]
    fn notes_too_high_for_a_period_have_none() {
        assert_eq!(note_period(36 + 12 * 6), Some(1));
        assert_eq!(note_period(36 + 12 * 7), None);
        assert_eq!(note_period(36 + 12 * 16), None);
    }
}
