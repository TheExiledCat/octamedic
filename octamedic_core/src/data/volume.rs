pub const MAX_VOLUME: u8 = 64;

#[derive(Clone, Copy, Debug)]
pub struct OctamedicVolume {
    value: u8,
}

impl OctamedicVolume {
    pub fn new(value: u8) -> Self {
        let mut s = Self { value: 0 };
        s.set(value);
        return s;
    }

    pub fn get(&self) -> u8 {
        return self.value;
    }

    pub fn set(&mut self, value: u8) {
        self.value = value.min(MAX_VOLUME);
    }
}
