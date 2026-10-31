use octamed::utility::bytes::UByte;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OctamedicNote {
    /// Raw OctaMED note number, 0 = none.
    pub value: u8,
}

impl OctamedicNote {
    pub fn new() -> Self {
        Self { value: 0 }
    }

    pub fn from_octamed(note: UByte) -> Self {
        Self { value: note.0 }
    }
}

impl Default for OctamedicNote {
    fn default() -> Self {
        return Self::new();
    }
}
