use octamed::utility::bytes::UByte;

#[derive(Clone, Copy, Debug)]

pub struct OctamedicVolume {
    value: UByte,
}

impl OctamedicVolume {
    pub fn new(value: u8) -> Self {

        let mut s = Self {
            value: UByte(0),
        };

        s.set(value);

        return s;
    }

    pub fn get(&self) -> u8 {

        return self.value.0;
    }

    pub fn set(&mut self, value: u8) {

        self.value.0 = value.clamp(0, 64);
    }
}
