use std::fmt::Display;

use crate::utility::bytes::{ Byte, Offset, UByte, ULong, UWord, Word, bit_mask };

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
        return Self { load_to_fast_memory: bit_mask(byte, 0x1) };
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
        for i in 0..self.sample_count.0 as usize {
            let sample = self.samples[i];
            writeln!(f, "Sample: {}", i)?;
            writeln!(f, "Loop point: {}", sample.repeat)?;
            writeln!(f, "Loop Length: {}", sample.repeat_length)?;
            writeln!(f, "Midi Channel: {}", sample.midi_channel)?;
            writeln!(f, "Midi Preset: {}", sample.midi_preset)?;
            writeln!(f, "Volume: {}", sample.sample_volume)?;
            writeln!(f, "Transpose: {}", sample.sample_transpose)?;

            writeln!(f)?;
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
pub struct OctamedMMD0SampleHeader {
    pub sample_length: ULong,
    pub sample_type: OctamedMMD0InstrumentType,
}
pub struct OctamedMMD0SampleTable {
    pub headers: Vec<OctamedMMD0SampleHeader>,
    pub samples: Vec<Vec<u8>>,
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
    pub fn from_i8(byte: i8) -> Self {
        match byte {
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
            _ => panic!(),
        }
    }
}
pub struct OctamedMMD0ExpansionData {}

pub struct OctamedMMD0SongFlags {
    pub filter_is_on: bool,
    pub jumping_is_on: bool,
    pub jump_every_eight_lines: bool,
    pub song_samples_indicator: bool,
    pub volumes_are_hex: bool,
    pub use_st_sliding: bool,
    pub is_8_channels: bool,
    pub is_hq_v2_compatability: bool,
    pub bpm_beat_length: UByte,
    pub is_bpm_mode: bool,
    pub mixing_enabled: bool,
}

impl OctamedMMD0SongFlags {
    pub fn from_bytes(byte1: UByte, byte2: UByte) -> Self {
        let filter_is_on = bit_mask(byte1, 0x01);
        let jumping_is_on = bit_mask(byte1, 0x02);
        let jump_every_eight_lines = bit_mask(byte1, 0x04);
        let song_samples_indicator = bit_mask(byte1, 0x08);
        let volumes_are_hex = bit_mask(byte1, 0x10);
        let use_st_sliding = bit_mask(byte1, 0x20);
        let is_8_channels = bit_mask(byte1, 0x40);
        let is_hq_v2_compatability = bit_mask(byte1, 0x80);
        let bpm_beat_length = UByte(byte2.0 & 0x1f);
        let is_bpm_mode = bit_mask(byte2, 0x20);
        let mixing_enabled = bit_mask(byte2, 0x80);

        return Self {
            filter_is_on,
            jumping_is_on,
            jump_every_eight_lines,
            song_samples_indicator,
            volumes_are_hex,
            use_st_sliding,
            is_8_channels,
            is_hq_v2_compatability,
            bpm_beat_length,
            is_bpm_mode,
            mixing_enabled,
        };
    }
}

impl Display for OctamedMMD0SongFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.filter_is_on {
            writeln!(f, "FILTER_ON")?;
        }
        if self.jumping_is_on {
            writeln!(f, "JUMPING_ON")?;
        }
        if self.jump_every_eight_lines {
            writeln!(f, "JUMP_8")?;
        }
        if self.song_samples_indicator {
            writeln!(f, "SNG_SAMPLES")?;
        }
        if self.volumes_are_hex {
            writeln!(f, "VOL_HEX")?;
        }
        if self.use_st_sliding {
            writeln!(f, "ST_SLIDING")?;
        }
        if self.is_8_channels {
            writeln!(f, "8_CHANNELS")?;
        }
        if self.is_hq_v2_compatability {
            writeln!(f, "HQ_v2-v4")?;
        }

        return Ok(());
    }
}
