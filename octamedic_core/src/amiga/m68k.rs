// 68000 DIVU: if the quotient overflows 16 bits you get the dividend's low word back
pub fn divu(dividend: u32, divisor: u16) -> u16 {
    if divisor == 0 {
        // a real 68000 traps
        return dividend as u16;
    }
    let quotient = dividend / divisor as u32;
    return u16::try_from(quotient).unwrap_or(dividend as u16);
}
