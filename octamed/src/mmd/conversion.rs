use crate::mmd::module::OctamedMMD;

pub trait ToModule {
    fn to_module(&mut self) -> OctamedMMD;
}
pub trait FromModule {
    fn from_module(&mut self, module: &OctamedMMD) -> Self;
}

pub trait ModuleWriter {
    fn write_module(&mut self, module: &OctamedMMD) -> Vec<u8>;
}
