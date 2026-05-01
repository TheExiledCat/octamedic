use std::io::Write;

use crate::{ mmd::module::{ OctamedMMD, OctamedMMD0Header }, utility::bytes::IntoBytes };

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

pub trait BinaryWriter {
    fn write_bytes(&mut self, bytes: impl IntoBytes) -> std::io::Result<usize>;
}

impl BinaryWriter for Vec<u8> {
    fn write_bytes(&mut self, mut bytes: impl IntoBytes) -> std::io::Result<usize> {
        let bytes = bytes.as_bytes();
        self.write_all(&bytes)?;
        return Ok(bytes.len());
    }
}
