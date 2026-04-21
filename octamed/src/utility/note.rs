use crate::utility::frequency::Frequency;

#[derive(Clone, Copy)]
pub struct MusicalNote {
    frequency: Frequency,
    base_frequency: Frequency,
}

impl MusicalNote {
    pub fn from_freq(frequency: Frequency) -> Self {
        return Self { frequency, base_frequency: Frequency::hertz(440.0) };
    }
    pub fn with_base(mut self, frequency: Frequency) -> Self {
        self.base_frequency = frequency;
        return self;
    }
    pub fn get_midi(&self) -> i32 {
        let midi =
            69.0 + 12.0 * (self.frequency.as_hertz() / self.base_frequency.as_hertz()).log2();

        return midi.round() as i32;
    }

    pub fn get_frequency(&self) -> Frequency {
        return self.frequency;
    }
    pub fn get_note_name(&self) -> String {
        let midi = self.get_midi();
        let note_index = (midi % 12).rem_euclid(12) as usize;
        let octave = midi / 12 - 1;

        let note = NOTE_NAMES[note_index];
        return if note.ends_with('#') {
            format!("{}{}", note, octave)
        } else {
            format!("{}-{}", note, octave)
        };
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
