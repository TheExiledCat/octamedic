//! Numbers in base 32, as OctaMED shows instruments: digits 0-9 then A-V, so 1 to 63 runs 01 to
//! 0V and then 10 to 1V.

pub const BASE: u8 = 32;

const DIGITS: &[u8; 32] = b"0123456789ABCDEFGHIJKLMNOPQRSTUV";

/// The digit for `value`'s lowest 5 bits.
pub fn digit(value: u32) -> char {
    return DIGITS[(value % 32) as usize] as char;
}

/// What a digit is worth, in either case; `None` for anything that isn't one.
pub fn digit_value(c: char) -> Option<u32> {
    return c.to_digit(32);
}

/// `value` in at least `width` digits, with `fill` in front where it has fewer.
pub fn format(value: u32, width: usize, fill: char) -> String {
    let mut digits = Vec::new();
    let mut rest = value;
    loop {
        digits.push(digit(rest));
        rest /= 32;
        if rest == 0 {
            break;
        }
    }
    let padding = width.saturating_sub(digits.len());
    return std::iter::repeat_n(fill, padding)
        .chain(digits.into_iter().rev())
        .collect();
}

/// Reads digits in either case; `None` if any character isn't one, or it's empty.
pub fn parse(text: &str) -> Option<u32> {
    if text.is_empty() {
        return None;
    }
    return text.chars().try_fold(0u32, |value, c| {
        value.checked_mul(32)?.checked_add(digit_value(c)?)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_run_0_to_v() {
        assert_eq!(digit(0), '0');
        assert_eq!(digit(10), 'A');
        assert_eq!(digit(31), 'V');
        assert_eq!(digit(32), '0');
        assert_eq!(digit_value('v'), Some(31));
        assert_eq!(digit_value('W'), None);
    }

    #[test]
    fn instruments_are_two_digits() {
        assert_eq!(format(1, 2, '0'), "01");
        assert_eq!(format(31, 2, '0'), "0V");
        assert_eq!(format(32, 2, '0'), "10");
        assert_eq!(format(63, 2, '0'), "1V");
        assert_eq!(format(5, 2, ' '), " 5");
        assert_eq!(format(1024, 2, '0'), "100");
    }

    #[test]
    fn parsing_reads_what_formatting_writes() {
        for value in [0, 1, 31, 32, 63, 1000] {
            assert_eq!(parse(&format(value, 2, '0')), Some(value));
        }
        assert_eq!(parse("1v"), Some(63));
        assert_eq!(parse(""), None);
        assert_eq!(parse("1W"), None);
    }
}
