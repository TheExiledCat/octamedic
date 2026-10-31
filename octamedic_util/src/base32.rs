//! Base 32 the way OctaMED shows instrument numbers: 0-9 then A-V, so 01..0V, 10..1V.

pub const BASE: u8 = 32;

const DIGITS: &[u8; BASE as usize] = b"0123456789ABCDEFGHIJKLMNOPQRSTUV";

pub fn digit(value: u32) -> char {
    return DIGITS[(value % BASE as u32) as usize] as char;
}

pub fn digit_value(c: char) -> Option<u32> {
    return c.to_digit(BASE as u32);
}

/// Pads to `width` with `fill` on the left.
pub fn format(value: u32, width: usize, fill: char) -> String {
    let mut digits = Vec::new();
    let mut rest = value;
    loop {
        digits.push(digit(rest));
        rest /= BASE as u32;
        if rest == 0 {
            break;
        }
    }
    let padding = width.saturating_sub(digits.len());
    return std::iter::repeat_n(fill, padding)
        .chain(digits.into_iter().rev())
        .collect();
}

/// Case insensitive. Empty string is `None`.
pub fn parse(text: &str) -> Option<u32> {
    if text.is_empty() {
        return None;
    }
    return text.chars().try_fold(0u32, |value, c| {
        value.checked_mul(BASE as u32)?.checked_add(digit_value(c)?)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits() {
        assert_eq!(digit(0), '0');
        assert_eq!(digit(10), 'A');
        assert_eq!(digit(31), 'V');
        assert_eq!(digit(32), '0');
        assert_eq!(digit_value('v'), Some(31));
        assert_eq!(digit_value('W'), None);
    }

    #[test]
    fn two_digits() {
        assert_eq!(format(1, 2, '0'), "01");
        assert_eq!(format(31, 2, '0'), "0V");
        assert_eq!(format(32, 2, '0'), "10");
        assert_eq!(format(63, 2, '0'), "1V");
        assert_eq!(format(5, 2, ' '), " 5");
        assert_eq!(format(1024, 2, '0'), "100");
    }

    #[test]
    fn roundtrip() {
        for value in [0, 1, 31, 32, 63, 1000] {
            assert_eq!(parse(&format(value, 2, '0')), Some(value));
        }
        assert_eq!(parse("1v"), Some(63));
        assert_eq!(parse(""), None);
        assert_eq!(parse("1W"), None);
    }
}
