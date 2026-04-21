use std::{ fmt::Display };

use crate::utility::{ bytes::UByte, frequency::Frequency };

#[derive(Clone, Copy)]
pub struct MidiNote {
    midi: i32,
}

impl MidiNote {
    pub fn new(midi: i32) -> Self {
        return Self { midi };
    }
    pub fn from_frequency(frequency: Frequency) -> Self {
        return Self { midi: (69.0 + 12.0 * (frequency.as_hertz() / 440.0).log2()).round() as i32 };
    }
    pub fn from_octamed_note_number(number: UByte) -> Self {
        if number.0 == 0 {
            return Self {
                midi: -1,
            };
        }

        let midi = (number.0 as i32) - 1;
        Self::new(midi)
    }

    pub fn get_midi(&self) -> i32 {
        return self.midi;
    }

    pub fn get_frequency(&self) -> Frequency {
        return Frequency::hertz(440.0 * (2f32).powf(((self.midi as f32) - 69.0) / 12.0));
    }
    pub fn get_note_name(&self) -> String {
        if self.midi < 0 {
            return "---".to_string();
        }

        let note_index = (self.midi % 12) as usize;
        let octave = ((self.midi as f32) / (NOTE_NAMES.len() as f32)).floor() + 1.0;

        let note = NOTE_NAMES[note_index];
        return if note.chars().count() > 1 {
            format!("{}{}", note, octave)
        } else {
            format!("{}-{}", note, octave)
        };
    }
    pub fn get_note_name_octamed(&self) -> String {
        let mut clone = self.clone();
        clone.midi = clone.midi;
        return clone.get_note_name();
    }
}

impl Display for MidiNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.get_note_name_octamed());
    }
}
const NOTE_NAMES: &[&'static str] = &[
    "C",
    "C#",
    "D",
    "D#",
    "E",
    "F",
    "F#",
    "G",
    "G#",
    "A",
    "A#",
    "B",
];
