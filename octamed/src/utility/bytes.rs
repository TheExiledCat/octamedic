use std::fmt::Display;

pub struct ULong(pub u32);
pub struct UWord(pub u16);
pub struct UByte(pub u8);
pub struct Word(pub i16);
pub struct Offset(pub u32);

impl Display for Offset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "${:08X}", self.0)
    }
}
