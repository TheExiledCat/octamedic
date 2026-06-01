use octamed::utility::bytes::UByte;

#[derive(Clone, Copy)]
pub struct OctamedicNote {
    /// Raw OctaMED note number. 0 = no note, 1–128 = chromatic scale.
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
