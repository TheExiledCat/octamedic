use octamed::utility::bytes::UByte;

#[derive(Clone, Copy)]

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

    pub fn set(&mut self, value: u8) {

        self.value.0 = value.clamp(0, 64);
    }
}
