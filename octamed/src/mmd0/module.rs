use std::{ collections::HashMap, fmt::Display };

use crate::utility::{
    bytes::{ Byte, Offset, UByte, ULong, UWord, Word, bit_flag, bit_slice },
    note,
};

pub struct OctamedMMD0Header {
    pub id: ULong,
    pub module_length: ULong,
    pub song_ptr: Offset,
    pub player_seconds_num: UWord,
    pub player_sequence: UWord,
    pub block_array_ptr: Offset,
    pub flags: OctamedMMD0HeaderFlags,
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
pub struct OctamedMMD0HeaderFlags {
    pub load_to_fast_memory: bool,
}
impl OctamedMMD0HeaderFlags {
    pub fn from_byte(byte: UByte) -> Self {
        return Self { load_to_fast_memory: bit_flag(byte, 0x1) };
    }
}
impl Display for OctamedMMD0HeaderFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.load_to_fast_memory {
            writeln!(f, "LOAD_TO_FAST_MEM")?;
        } else {
            writeln!(f, "N/A")?;
        }

        return Ok(());
    }
}

impl Display for OctamedMMD0Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "OctaMED File Header")?;
        writeln!(f, "{}", (0..20).map(|_| "_").collect::<String>())?;

        let id_bytes = self.id.0.to_be_bytes();
        let id_text = str::from_utf8(&id_bytes).unwrap();
        writeln!(f, "Format: {}", id_text)?;
        writeln!(f, "Module Length: {} Bytes", self.module_length)?;
        writeln!(f, "Song struct Pointer: {}", self.song_ptr)?;
        writeln!(f, "Pattern blocks table Pointer: {}", self.block_array_ptr)?;
        writeln!(f, "Flags: {}", self.flags)?;
        writeln!(f, "Sample Array Pointer: {}", self.sample_array_ptr)?;
        writeln!(f, "Expansion Data Offset: {}", self.expansion_data_ptr)?;
        return writeln!(f, "Extra songs count: {}", self.extra_songs);
    }
}
pub struct OctamedMMD0 {
    pub header: OctamedMMD0Header,
    pub song: OctamedMMD0Song,
    pub block_table: OctamedMMD0BlockTable,
    pub sample_table: OctamedMMD0SampleTable,
    pub expansion_data: OctamedMMD0ExpansionData,
}

pub struct OctamedMMD0Song {
    pub samples: [OctamedMMD0Sample; 63],
    pub block_count: UWord,
    pub song_length: UWord,
    pub player_sequence_list: [UByte; 256],
    pub default_song_tempo: UWord,
    pub global_transpose: Byte,
    pub flags: OctamedMMD0SongFlags,
    pub pulses_per_line: UByte,
    pub track_volumes: [UByte; 16],
    pub master_volume: UByte,
    pub sample_count: UByte,
}
impl Display for OctamedMMD0Song {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Song Metadata")?;
        writeln!(f, "{}", (0..20).map(|_| "_").collect::<String>())?;

        writeln!(f, "Block count: {}", self.block_count)?;
        writeln!(f, "Song Length: {}", self.song_length)?;
        writeln!(f, "Default Tempo: {}", self.default_song_tempo)?;
        writeln!(f, "Global Transpose: {}", self.global_transpose)?;
        writeln!(f, "Flags:\n{}", self.flags)?;
        writeln!(f, "Pulses Per Line: {}", self.pulses_per_line)?;
        writeln!(f, "Master volume: {}", self.master_volume)?;
        writeln!(f, "Sample Count: {}", self.sample_count)?;

        writeln!(
            f,
            "Track Volumes: {:?}",
            self.track_volumes.map(|b| b.0)
        )?;
        writeln!(f)?;
        writeln!(f, "Sample info")?;
        // for i in 0..self.sample_count.0 as usize {
        //     let sample = self.samples[i];
        //     writeln!(f, "Sample: {}", i)?;
        //     writeln!(f, "Loop point: {}", sample.repeat)?;
        //     writeln!(f, "Loop Length: {}", sample.repeat_length)?;
        //     writeln!(f, "Midi Channel: {}", sample.midi_channel)?;
        //     writeln!(f, "Midi Preset: {}", sample.midi_preset)?;
        //     writeln!(f, "Volume: {}", sample.sample_volume)?;
        //     writeln!(f, "Transpose: {}", sample.sample_transpose)?;

        //     writeln!(f)?;
        // }

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
pub struct OctamedMMD0BlockTable {
    pub headers: Vec<OctamedMMD0BlockHeader>,
    pub blocks: Vec<OctamedMMD0Block>,
}
pub struct OctamedMMD0Block {
    pub lines: Vec<OctamedMMD0BlockLine>,
}
pub struct OctamedMMD0BlockLine {
    pub tracks: Vec<OctamedMMD0TrackLine>,
}
pub struct OctamedMMD0TrackLine {
    pub note_number: UByte,
    pub instrument_number: UByte,
    pub command_number: UByte,
    pub command_value: UByte,
}

impl OctamedMMD0TrackLine {
    pub fn from_bytes(byte1: UByte, byte2: UByte, byte3: UByte) -> Self {
        let command_value = byte3;
        let command_number = UByte(byte2.0 & 0x0f);
        let note_number = UByte(byte1.0 & 0x3f);
        let xy = (byte1.0 >> 6) << 4;
        let iiii = byte2.0 >> 4;
        let instrument_number = UByte(xy | iiii);

        return Self { note_number, instrument_number, command_number, command_value };
    }
}
impl Display for OctamedMMD0TrackLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:3} {:01X}{:02X}{:02X}",
            note::MidiNote::from_octamed_note_number(self.note_number),
            self.instrument_number.0,
            self.command_number.0,
            self.command_value.0
        )
    }
}
pub struct OctamedMMD0BlockHeader {
    pub track_count: UByte,
    pub line_count: UByte,
}
pub struct OctamedMMD0SampleHeader {
    pub sample_length: ULong,
    pub sample_type: OctamedMMD0InstrumentType,
    pub is_16_bit: bool,
    pub is_stereo: bool,
}
pub struct OctamedMMD0SampleTable {
    pub headers: Vec<Option<OctamedMMD0SampleHeader>>,
    pub samples: Vec<Option<Vec<Byte>>>,
}
#[repr(i8)]
pub enum OctamedMMD0InstrumentType {
    Hybrid = -2,
    Synth,
    Sample,
    SampleOct5,
    SampleOct3,
    SampleOct2,
    SampleOct4,
    SampleOct6,
    SampleOct7,
    ExtSample,
}

impl OctamedMMD0InstrumentType {
    pub fn from_word(word: Word) -> Self {
        match word.0 {
            -2 => OctamedMMD0InstrumentType::Hybrid,
            -1 => OctamedMMD0InstrumentType::Synth,
            0 => OctamedMMD0InstrumentType::Sample,
            1 => OctamedMMD0InstrumentType::SampleOct5,
            2 => OctamedMMD0InstrumentType::SampleOct3,
            3 => OctamedMMD0InstrumentType::SampleOct2,
            4 => OctamedMMD0InstrumentType::SampleOct4,
            5 => OctamedMMD0InstrumentType::SampleOct6,
            6 => OctamedMMD0InstrumentType::SampleOct7,
            7 => OctamedMMD0InstrumentType::ExtSample,
            _ => panic!("Instrument type {} not known", word.0),
        }
    }
}
pub struct OctamedMMD0ExpansionData {}

pub struct OctamedMMD0SongFlags(UByte, UByte);

impl OctamedMMD0SongFlags {
    pub fn from_bytes(byte1: UByte, byte2: UByte) -> Self {
        return Self(byte1, byte2);
    }

    pub fn filter_is_on(&self) -> bool {
        return bit_flag(self.0, 0x01);
    }
    pub fn jumping_is_on(&self) -> bool {
        return bit_flag(self.0, 0x02);
    }
    pub fn jump_every_eight_lines(&self) -> bool {
        return bit_flag(self.0, 0x04);
    }
    pub fn song_samples_indicator(&self) -> bool {
        return bit_flag(self.0, 0x08);
    }
    pub fn volumes_are_hex(&self) -> bool {
        return bit_flag(self.0, 0x10);
    }
    pub fn use_st_sliding(&self) -> bool {
        return bit_flag(self.0, 0x20);
    }
    pub fn is_8_channels(&self) -> bool {
        return bit_flag(self.0, 0x40);
    }
    pub fn is_hq_v2_compatability(&self) -> bool {
        return bit_flag(self.0, 0x80);
    }
    pub fn bpm_beat_length(&self) -> UByte {
        return UByte(self.1.0 & 0x1f);
    }
    pub fn is_bpm_mode(&self) -> bool {
        return bit_flag(self.1, 0x20);
    }
    pub fn mixing_enabled(&self) -> bool {
        return bit_flag(self.1, 0x80);
    }
}

impl Display for OctamedMMD0SongFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.filter_is_on() {
            writeln!(f, "FILTER_ON")?;
        }
        if self.jumping_is_on() {
            writeln!(f, "JUMPING_ON")?;
        }
        if self.jump_every_eight_lines() {
            writeln!(f, "JUMP_8")?;
        }
        if self.song_samples_indicator() {
            writeln!(f, "SNG_SAMPLES")?;
        }
        if self.volumes_are_hex() {
            writeln!(f, "VOL_HEX")?;
        }
        if self.use_st_sliding() {
            writeln!(f, "ST_SLIDING")?;
        }
        if self.is_8_channels() {
            writeln!(f, "8_CHANNELS")?;
        }
        if self.is_hq_v2_compatability() {
            writeln!(f, "HQ_v2-v4")?;
        }

        return Ok(());
    }
}
