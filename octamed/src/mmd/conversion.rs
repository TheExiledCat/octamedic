use std::io::Write;

use crate::{
    mmd::module::{
        OctamedMMD,
        OctamedMMD0Block,
        OctamedMMD0BlockHeader,
        OctamedMMD0ColorPallete,
        OctamedMMD0Dump,
        OctamedMMD0ExpansionHeader,
        OctamedMMD0Header,
        OctamedMMD0Info,
        OctamedMMD0MidiCommands,
        OctamedMMD0NotationInfo,
        OctamedMMD0Rexx,
        OctamedMMD0SampleHeader,
        OctamedMMD0SampleTable,
        OctamedMMD0Song,
        OctamedMMD1Block,
        OctamedMMD1BlockHeader,
        OctamedMMD1BlockInfo,
        OctamedMMD1BlockInfoHeader,
        OctamedMMDBlockTable,
    },
    utility::bytes::{ IntoBytes, Offset, UWord },
};

pub trait ToModule {
    fn to_module(&mut self) -> OctamedMMD;
}
pub trait FromModule {
    fn from_module(&mut self, module: &OctamedMMD) -> Self;
}

pub trait BinarySize where Self: Sized {
    fn get_size(&self, mmd: &OctamedMMD) -> u32 {
        return size_of::<Self>() as u32;
    }
}
impl BinarySize for OctamedMMD0Header {}
impl BinarySize for OctamedMMD0Song {}
impl BinarySize for OctamedMMDBlockTable {
    fn get_size(&self, mmd: &OctamedMMD) -> u32 {
        let block_count = mmd.song.block_count;

        return (block_count.0 as u32) * (size_of::<Offset>() as u32);
    }
}
impl BinarySize for OctamedMMD0SampleTable {
    fn get_size(&self, mmd: &OctamedMMD) -> u32 {
        let sample_count = mmd.song.sample_count;

        return (sample_count.0 as u32) * (size_of::<Offset>() as u32);
    }
}
impl BinarySize for OctamedMMD0SampleHeader {
    fn get_size(&self, mmd: &OctamedMMD) -> u32 {
        return (size_of_val(&self.sample_length) + size_of_val(&self.sample_type)) as u32;
    }
}
impl BinarySize for OctamedMMD0ExpansionHeader {
    fn get_size(&self, mmd: &OctamedMMD) -> u32 {
        let size = size_of::<Self>();
        assert_eq!(size, 84);

        return size as u32;
    }
}
impl BinarySize for OctamedMMD0ColorPallete {
    fn get_size(&self, mmd: &OctamedMMD) -> u32 {
        return (size_of::<UWord>() as u32) * 8;
    }
}
impl BinarySize for OctamedMMD0NotationInfo {}
impl BinarySize for OctamedMMD0BlockHeader {}

impl BinarySize for OctamedMMD1BlockHeader {}
impl BinarySize for OctamedMMD1BlockInfoHeader {}
impl BinarySize for OctamedMMD0Info {}
impl BinarySize for OctamedMMD0Dump {}
impl BinarySize for OctamedMMD0Rexx {}
impl BinarySize for OctamedMMD0MidiCommands {}
pub trait BinaryWriter {
    fn write_bytes(&mut self, bytes: &impl IntoBytes) -> std::io::Result<usize>;
}

impl BinaryWriter for Vec<u8> {
    fn write_bytes(&mut self, bytes: &impl IntoBytes) -> std::io::Result<usize> {
        let bytes = bytes.as_bytes();
        self.write_all(&bytes)?;
        return Ok(bytes.len());
    }
}
