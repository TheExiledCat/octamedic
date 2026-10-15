use octamed::utility::{bytes::UByte, frequency::Frequency};

use crate::data::note::OctamedicNote;

struct Voice {}

pub struct Voices {
    //each channels voice. when None, no sample is playing on that voice.
    channels: Vec<Option<Voice>>,
}

impl Voices {
    pub fn new() -> Self {

        return Self::default();
    }

    pub fn note_on(&mut self, channel: UByte, note: &OctamedicNote) {

        todo!();
    }

    /// Stops the track's note;

    pub fn release(&mut self, channel: UByte) {

        todo!()
    }
}

impl Default for Voices {
    fn default() -> Self {

        Self {
            channels: Default::default(),
        }
    }
}
