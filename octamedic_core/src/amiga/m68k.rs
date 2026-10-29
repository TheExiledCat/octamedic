/// The 68000's `DIVU`: a quotient too big for 16 bits overflows and leaves the dividend's low
/// word.
pub fn divu(dividend: u32, divisor: u16) -> u16 {
    if divisor == 0 {
        // a real 68000 traps
        return dividend as u16;
    }
    let quotient = dividend / divisor as u32;
    return u16::try_from(quotient).unwrap_or(dividend as u16);
}
