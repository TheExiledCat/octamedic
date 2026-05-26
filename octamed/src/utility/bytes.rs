use std::{ char, fmt::Display, io::SeekFrom };

use crate::mmd::module::{
        OctamedMMD0ColorPallete,
        OctamedMMD0HeaderFlags,
        OctamedMMD0InstrumentType,
        OctamedMMD0SongFlags,
    };

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
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct UByte(pub u8);
impl UByte {
    pub fn as_char(&self) -> char {
        return char::from_u32(self.0 as u32).unwrap();
    }
}
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

pub fn bit_flag(byte: impl IntoByte, mask: u8) -> bool {
    return bit_mask(byte, mask).0 != 0;
}
pub fn bit_mask(byte: impl IntoByte, mask: u8) -> UByte {
    return UByte(byte.as_byte().0 & mask);
}
pub fn bit_slice(byte: impl IntoByte, start: u8, end: u8) -> UByte {
    assert!(start < end && end <= 8);

    let mask = (1u8 << (end - start)) - 1;
    let value = (byte.as_byte().0 >> start) & mask;

    UByte(value)
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

pub trait ValueMap {
    type Value;
    fn map<F>(&self, f: F) -> Self where F: Fn(Self::Value) -> Self::Value;
}
impl ValueMap for UByte {
    type Value = u8;

    fn map<F>(&self, f: F) -> Self where F: Fn(Self::Value) -> Self::Value {
        return Self(f(self.0));
    }
}
impl ValueMap for UWord {
    type Value = u16;

    fn map<F>(&self, f: F) -> Self where F: Fn(Self::Value) -> Self::Value {
        return Self(f(self.0));
    }
}
impl ValueMap for ULong {
    type Value = u32;

    fn map<F>(&self, f: F) -> Self where F: Fn(Self::Value) -> Self::Value {
        return Self(f(self.0));
    }
}

pub trait IntoBytes {
    fn as_bytes(&self) -> Vec<u8>;
}

impl IntoBytes for ULong {
    fn as_bytes(&self) -> Vec<u8> {
        return self.0.to_be_bytes().to_vec();
    }
}
impl IntoBytes for UWord {
    fn as_bytes(&self) -> Vec<u8> {
        return self.0.to_be_bytes().to_vec();
    }
}
impl IntoBytes for UByte {
    fn as_bytes(&self) -> Vec<u8> {
        return self.0.to_be_bytes().to_vec();
    }
}

impl IntoBytes for Word {
    fn as_bytes(&self) -> Vec<u8> {
        return self.0.to_be_bytes().to_vec();
    }
}
impl IntoBytes for Byte {
    fn as_bytes(&self) -> Vec<u8> {
        return self.0.to_be_bytes().to_vec();
    }
}

impl IntoBytes for String {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = String::as_bytes(&self).to_vec();
        bytes.push(0);
        return bytes;
    }
}

impl IntoBytes for Offset {
    fn as_bytes(&self) -> Vec<u8> {
        self.0.to_be_bytes().to_vec()
    }
}
impl IntoBytes for OctamedMMD0HeaderFlags {
    fn as_bytes(&self) -> Vec<u8> {
        let val: u8 = if self.load_to_fast_memory { 1 } else { 0 };
        return val.to_be_bytes().to_vec();
    }
}
impl<T, const N: usize> IntoBytes for [T; N] where T: IntoBytes {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        for b in 0..N {
            let arr = self[b].as_bytes();
            bytes.extend(arr);
        }
        return bytes;
    }
}
impl IntoBytes for OctamedMMD0SongFlags {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        bytes.push(self.0.0);
        bytes.push(self.1.0);
        return bytes;
    }
}
impl<I> IntoBytes for Vec<I> where I: IntoBytes {
    fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![];
        for v in self {
            bytes.extend(v.as_bytes());
        }
        return bytes;
    }
}
impl<I> IntoBytes for &[I] where I: IntoBytes {
    fn as_bytes(&self) -> Vec<u8> {
        return self
            .iter()
            .map(|b| b.as_bytes())
            .flatten()
            .collect();
    }
}
impl IntoBytes for OctamedMMD0InstrumentType {
    fn as_bytes(&self) -> Vec<u8> {
        return (*self as i16).to_be_bytes().to_vec();
    }
}
impl IntoBytes for OctamedMMD0ColorPallete {
    fn as_bytes(&self) -> Vec<u8> {
        return self.colors
            .iter()
            .map(|c| c.value.as_bytes())
            .flatten()
            .collect();
    }
}
