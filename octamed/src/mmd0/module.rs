use std::fmt::Display;

use crate::utility::bytes::{ Byte, Offset, UByte, ULong, UWord, Word };

pub struct OctamedMMD0Header {
    pub id: ULong,
    pub module_length: ULong,
    pub song_ptr: Offset,
    pub player_seconds_num: UWord,
    pub player_sequence: UWord,
    pub block_array_ptr: Offset,
    pub flags: UByte,
    pub reserved: [u8; 3],
    pub sample_array_ptr: Offset,
    pub reserved2: ULong,
    pub expansion_data_ptr: Offset,
    pub reserved3: ULong,
    pub player_state: UWord,
    pub player_block: UWord,
    pub player_line: UWord,
    pub player_sequence_num: UWord,
    pub actual_play_line: Word,
    pub counter: UByte,
    pub extra_songs: UByte,
}

impl Display for OctamedMMD0Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "OctaMED File Header")?;
        writeln!(f, "{}", (0..20).map(|_| "_").collect::<String>());

        let id_bytes = self.id.0.to_be_bytes();
        let id_text = str::from_utf8(&id_bytes).unwrap();
        writeln!(f, "ID: {}", id_text);
        writeln!(f, "File Length: {} Bytes", self.module_length);
        writeln!(f, "Song struct Pointer: {}", self.song_ptr);
        writeln!(f, "Pattern blocks table Pointer: {}", self.block_array_ptr);
        writeln!(f, "Flags: {:08b}", self.flags.0);
        writeln!(f, "Sample Array Pointer: {}", self.sample_array_ptr);
        writeln!(f, "Expansion Data Offset: {}", self.expansion_data_ptr);
        return writeln!(f, "Extra songs count: {}", self.extra_songs);
    }
}
pub struct OctamedMMD0 {
    pub header: OctamedMMD0Header,
    pub song: OctamedMMD0Song,
    pub block_table: OctamedMMD0BlockTable,
    pub sample_table: OctamedMMD0SampleTable,
}

pub struct OctamedMMD0Song {
    pub samples: [OctamedMMD0Sample; 63],
    pub block_count: UWord,
    pub song_length: UWord,
    pub player_sequence_list: [UByte; 256],
    pub default_song_tempo: UWord,
    pub global_transpose: Byte,
    pub flags_byte: UByte,
    pub flags2_byte: UByte,
    pub pulses_per_line: UByte,
    pub track_volumes: [UByte; 16],
    pub master_volume: UByte,
    pub sample_count: UByte,
}
impl Display for OctamedMMD0Song {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Song Metadata");
        writeln!(f, "Block count: {}", self.block_count);
        writeln!(f, "Song Length: {}", self.song_length);
        writeln!(f, "Default Tempo: {}", self.default_song_tempo);
        writeln!(f, "Global Transpose: {}", self.global_transpose);
        writeln!(f, "Flags1: {:08b}", self.flags_byte.0);
        writeln!(f, "Flags2: {:08b}", self.flags2_byte.0);
        writeln!(f, "Pulses Per Line: {}", self.pulses_per_line);
        writeln!(f, "Master volume: {}", self.master_volume);
        writeln!(f, "Sample Count: {}", self.sample_count);

        writeln!(
            f,
            "Track Volumes: {:?}",
            self.track_volumes.map(|b| b.0)
        );
        writeln!(f);
        writeln!(f, "Sample info");
        for i in 0..self.sample_count.0 as usize {
            let sample = self.samples[i];
            writeln!(f, "Sample: {}", i);
            writeln!(f, "Loop point: {}", sample.repeat);
            writeln!(f, "Loop Length: {}", sample.repeat_length);
            writeln!(f, "Midi Channel: {}", sample.midi_channel);
            writeln!(f, "Midi Preset: {}", sample.midi_preset);
            writeln!(f, "Volume: {}", sample.sample_volume);
            writeln!(f, "Transpose: {}", sample.sample_transpose);

            writeln!(f);
        }

        return Ok(());
    }
}
#[derive(Clone, Copy)]
pub struct OctamedMMD0Sample {
    pub repeat: UWord,
    pub repeat_length: UWord,
    pub midi_channel: UByte,
    pub midi_preset: UByte,
    pub sample_volume: UByte,
    pub sample_transpose: Byte,
}
impl OctamedMMD0Sample {
    pub fn new() -> Self {
        return Self {
            repeat: UWord(0),
            repeat_length: UWord(0),
            midi_channel: UByte(1),
            midi_preset: UByte(1),
            sample_volume: UByte(u8::MAX),
            sample_transpose: Byte(0),
        };
    }
}
pub struct OctamedMMD0BlockTable {}
pub struct OctamedMMD0SampleTable {}
pub struct OctamedMMD0ExpansionData {}
pub enum MMDKind {
    MMD0(),
    MMD1(),
    MMD2(),
    MMD3(),
}
