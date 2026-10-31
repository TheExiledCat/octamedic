use std::fmt::Display;

use crate::utility::{
    bytes::UByte,
    frequency::{BASE_FREQ, Frequency},
};

pub const SEMITONES_PER_OCTAVE: u8 = 12;

/// The MIDI note of A above middle C, which `BASE_FREQ` is the pitch of.
const BASE_FREQ_MIDI: i32 = 69;

#[derive(Clone, Copy)]
pub struct MidiNote {
    midi: i32,
}

impl MidiNote {
    pub fn new(midi: i32) -> Self {
        return Self { midi };
    }

    pub fn from_frequency(frequency: Frequency) -> Self {
        return Self {
            midi: (BASE_FREQ_MIDI as f32
                + SEMITONES_PER_OCTAVE as f32 * (frequency.as_hertz() / BASE_FREQ as f32).log2())
            .round() as i32,
        };
    }

    pub fn from_octamed_note_number(number: UByte) -> Self {
        if number.0 == 0 {
            return Self { midi: -1 };
        }
        let midi = (number.0 as i32) - 1;
        Self::new(midi)
    }

    pub fn get_midi(&self) -> i32 {
        return self.midi;
    }

    pub fn get_frequency(&self) -> Frequency {
        let octaves = (self.midi - BASE_FREQ_MIDI) as f32 / SEMITONES_PER_OCTAVE as f32;
        return Frequency::hertz(BASE_FREQ as f32 * 2f32.powf(octaves));
    }

    pub fn get_note_name(&self) -> String {
        if self.midi < 0 {
            return "---".to_string();
        }
        let note_index = (self.midi % SEMITONES_PER_OCTAVE as i32) as usize;
        let octave = ((self.midi as f32) / (NOTE_NAMES.len() as f32)).floor() + 1.0;
        let note = NOTE_NAMES[note_index];
        return if note.chars().count() > 1 {
            format!("{}{}", note, octave)
        } else {
            format!("{}-{}", note, octave)
        };
    }

    pub fn get_note_name_octamed(&self) -> String {
        return self.get_note_name();
    }
}

impl Display for MidiNote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.get_note_name_octamed());
    }
}

const NOTE_NAMES: &[&str] = &[
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
