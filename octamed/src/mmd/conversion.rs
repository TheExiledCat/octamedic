use std::io::Write;

use crate::{
    mmd::module::{
        OctamedMMD,
        OctamedMMD0Block,
        OctamedMMD0BlockHeader,
        OctamedMMD0Header,
        OctamedMMD0Song,
        OctamedMMD1Block,
        OctamedMMD1BlockHeader,
        OctamedMMD1BlockInfo,
        OctamedMMD1BlockInfoHeader,
        OctamedMMDBlockTable,
    },
    utility::bytes::{ IntoBytes, Offset },
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
impl BinarySize for OctamedMMD0BlockHeader {}

impl BinarySize for OctamedMMD1BlockHeader {}
impl BinarySize for OctamedMMD1BlockInfoHeader {}
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
