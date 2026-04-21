use std::{ fmt::Display, io::SeekFrom };

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct ULong(pub u32);
impl Display for ULong {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct UWord(pub u16);
impl Display for UWord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct UByte(pub u8);
impl Display for UByte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Into<Byte> for UByte {
    fn into(self) -> Byte {
        return Byte(self.0.cast_signed());
    }
}
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Byte(pub i8);
impl Display for Byte {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Word(pub i16);
impl Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Offset(pub u32);

impl Offset {
    pub fn is_null(&self) -> bool {
        return self.0 == 0;
    }
}
impl Display for Offset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:X}", self.0)
    }
}
impl Into<SeekFrom> for Offset {
    fn into(self) -> SeekFrom {
        return SeekFrom::Start(self.0 as u64);
    }
}

pub fn bit_mask(byte: impl IntoByte, mask: u8) -> bool {
    return (byte.as_byte().0 & mask) != 0;
}

pub trait IntoByte {
    fn as_byte(&self) -> UByte;
}
impl IntoByte for UByte {
    fn as_byte(&self) -> UByte {
        return *self;
    }
}
impl IntoByte for Word {
    fn as_byte(&self) -> UByte {
        return UByte(self.0 as u8);
    }
}
impl IntoByte for ULong {
    fn as_byte(&self) -> UByte {
        return UByte(self.0 as u8);
    }
}
