use std::fmt::Display;

use crate::utility::{
    bytes::{ Byte, Offset, UByte, ULong, UWord, Word, bit_flag },
    note,
    octamed_tempo::OctamedTempo,
};
pub enum OctamedMMDType {
    MMD0,
    MMD1,
    MMD2,
    MMD3,
}
pub struct OctamedMMD0Header {
    pub id: ULong,
    pub module_length: ULong,
    pub song_ptr: Offset,
    pub player_seconds_num: UWord,
    pub player_sequence: UWord,
    pub block_array_ptr: Offset,
    pub flags: OctamedMMD0HeaderFlags,
    pub reserved: [UByte; 3],
    pub sample_array_ptr: Offset,
    pub reserved2: ULong,
    pub expansion_data_ptr: Offset,
    pub reserved3: ULong,
    pub player_state: UWord,
    pub player_block: UWord,
    pub player_line: UWord,
    pub player_sequence_num: UWord,
    pub active_play_line: Word,
    pub counter: UByte,
    pub extra_songs: UByte,
}
pub struct OctamedMMD0HeaderFlags {
    pub load_to_fast_memory: bool,
}
impl OctamedMMD0HeaderFlags {
    const LOAD_TO_FAST_MEM_FLAG: u8 = 0x1;

    pub fn from_byte(byte: UByte) -> Self {
        return Self { load_to_fast_memory: bit_flag(byte, Self::LOAD_TO_FAST_MEM_FLAG) };
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
pub struct OctamedMMD {
    pub header: OctamedMMD0Header,
    pub song: OctamedMMD0Song,
    pub block_table: OctamedMMDBlockTable,
    pub sample_table: OctamedMMD0SampleTable,
    pub expansion_data: Option<OctamedMMD0Expansion>,
}
impl OctamedMMD {
    pub fn get_type(&self) -> OctamedMMDType {
        match self.header.id.0 {
            0x4d4d4430 => OctamedMMDType::MMD0,
            0x4d4d4431 => OctamedMMDType::MMD1,
            0x4d4d4432 => OctamedMMDType::MMD2,
            0x4d4d4433 => OctamedMMDType::MMD3,
            _ => panic!(),
        }
    }
}
pub struct OctamedMMD0Song {
    pub samples: [OctamedMMD0Sample; 63],
    pub block_count: UWord,
    pub song_length: UWord,
    pub player_sequence_list: [UByte; 256],
    pub primary_tempo: UWord,
    pub global_transpose: Byte,
    pub flags: OctamedMMD0SongFlags,
    pub secondary_tempo: UByte,
    pub track_volumes: [UByte; 16],
    pub master_volume: UByte,
    pub sample_count: UByte,
}
impl OctamedMMD0Song {
    pub fn get_tempo(&self) -> OctamedTempo {
        return OctamedTempo::from_song(self);
    }
}
impl Display for OctamedMMD0Song {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Song Metadata")?;
        writeln!(f, "{}", (0..20).map(|_| "_").collect::<String>())?;

        writeln!(f, "Block count: {}", self.block_count)?;
        writeln!(f, "Song Length: {}", self.song_length)?;
        writeln!(f, "Default Tempo: {}", self.primary_tempo)?;
        writeln!(f, "Global Transpose: {}", self.global_transpose)?;
        writeln!(f, "Flags:\n{}", self.flags)?;
        writeln!(f, "Pulses Per Line: {}", self.secondary_tempo)?;
        writeln!(f, "Master volume: {}", self.master_volume)?;
        writeln!(f, "Sample Count: {}", self.sample_count)?;

        writeln!(
            f,
            "Track Volumes: {:?}",
            self.track_volumes.map(|b| b.0)
        )?;
        writeln!(f)?;
        // writeln!(f, "Sample info")?;
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
pub enum OctamedMMDBlockTable {
    MMD0BlockTable {
        headers: Vec<OctamedMMD0BlockHeader>,
        blocks: Vec<OctamedMMD0Block>,
    },
    MMD1BlockTable {
        headers: Vec<OctamedMMD1BlockHeader>,
        blocks: Vec<OctamedMMD1Block>,
    },
}

pub struct OctamedMMD0Block {
    pub lines: Vec<OctamedMMD0BlockLine>,
}
pub struct OctamedMMD1Block {
    pub lines: Vec<OctamedMMD0BlockLine>,
    pub info: Option<OctamedMMD1BlockInfo>,
}
pub struct OctamedMMD0BlockLine {
    pub tracks: Vec<OctamedMMDTrackLine>,
}
pub struct OctamedMMDTrackLine {
    pub note_number: UByte,
    pub instrument_number: UByte,
    pub command_number: UByte,
    pub command_value: UByte,
}

impl OctamedMMDTrackLine {
    pub const BLOCK_LINE_COMMAND_NUMBER_MASK_MMD0: u8 = 0x0f;
    pub const BLOCK_LINE_NOTE_NUMBER_MASK_MMD0: u8 = 0x3f;

    pub const BLOCK_LINE_INSTRUMENT_NUMBER_MASK_MMD1: u8 = 0x3f;
    pub const BLOCK_LINE_NOTE_NUMBER_MASK_MMD1: u8 = 0x7f;
    pub fn from_bytes_mmd0(byte1: UByte, byte2: UByte, byte3: UByte) -> Self {
        let command_value = byte3;
        let command_number = UByte(byte2.0 & Self::BLOCK_LINE_COMMAND_NUMBER_MASK_MMD0);
        let note_number = UByte(byte1.0 & Self::BLOCK_LINE_NOTE_NUMBER_MASK_MMD0);
        let xy = (byte1.0 >> 6) << 4; // leftmost 2 bits of byte 1 need to be prepended to byte 2 to get the value for the instrument number
        let iiii = byte2.0 >> 4;
        let instrument_number = UByte(xy | iiii);

        return Self { note_number, instrument_number, command_number, command_value };
    }
    pub fn from_bytes_mmd1(byte1: UByte, byte2: UByte, byte3: UByte, byte4: UByte) -> Self {
        let command_value = byte4;
        let command_number = byte3;
        let note_number = UByte(byte1.0 & Self::BLOCK_LINE_NOTE_NUMBER_MASK_MMD1);

        let instrument_number = UByte(byte2.0 & Self::BLOCK_LINE_INSTRUMENT_NUMBER_MASK_MMD1);

        return Self { note_number, instrument_number, command_number, command_value };
    }
}
impl Display for OctamedMMDTrackLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:3}{}{:02X}{:02X}",
            note::MidiNote::from_octamed_note_number(self.note_number),
            self.instrument_number.as_inst_string(),
            self.command_number.0,
            self.command_value.0
        )
    }
}

trait AsInstrumentString {
    fn as_inst_string(&self) -> String;
}
impl AsInstrumentString for UByte {
    fn as_inst_string(&self) -> String {
        let val = self.0;
        let ascii_start = 65;
        let mut text = String::with_capacity(2);
        match val {
            0..=31 => {
                text.push(' ');

                if val < 10 {
                    text.push(*val.to_string().chars().collect::<Vec<char>>().first().unwrap());
                } else {
                    let index = val - 10;
                    let c = char::from_u32((ascii_start as u32) + (index as u32)).unwrap();
                    text.push(c);
                }
            }
            32..=63 => {
                text.push('1');
                let val = val - 31;
                if val < 10 {
                    text.push(*val.to_string().chars().collect::<Vec<char>>().first().unwrap());
                } else {
                    let index = val - 10;
                    let c = char::from_u32((ascii_start as u32) + (index as u32)).unwrap();
                    text.push(c);
                }
            }
            _ => {
                panic!();
            }
        }
        return text;
    }
}
pub struct OctamedMMD0BlockHeader {
    pub track_count: UByte,
    pub line_count: UByte,
}
pub struct OctamedMMD1BlockHeader {
    pub track_count: UWord,
    pub line_count: UWord,
    pub info_ptr: Offset,
}

pub struct OctamedMMD1BlockInfo {
    pub header: OctamedMMD1BlockInfoHeader,
    pub block_name: String,
    pub highlight_mask: OctamedMMD1HighlightMask,
    pub page_table: OctamedMMD2BlockCommandPageTable, //octamed v6
}
pub struct OctamedMMD1BlockInfoHeader {
    pub highlight_mask_array_ptr: Offset,
    pub block_name_string_ptr: Offset,
    pub block_name_length: ULong,
    pub page_table_ptr: Offset,
    pub reserved: [ULong; 5],
}
//todo
pub struct OctamedMMD1HighlightMask {}
pub struct OctamedMMD2BlockCommandPageTable {}
pub struct OctamedMMD0SampleHeader {
    pub sample_length: ULong,
    pub sample_type: OctamedMMD0InstrumentType,
}
impl OctamedMMD0SampleHeader {
    pub const SIXTEEN_BIT_SAMPLE: i16 = 0x10;
    pub const STEREO_SAMPLE: i16 = 0x20;
    pub fn is_16_bit(&self) -> bool {
        return ((self.sample_type as i16) & Self::SIXTEEN_BIT_SAMPLE) != 0;
    }
    pub fn is_stereo(&self) -> bool {
        return ((self.sample_type as i16) & Self::STEREO_SAMPLE) != 0;
    }
}
pub struct OctamedMMD0SampleTable {
    pub headers: Vec<Option<OctamedMMD0SampleHeader>>,
    pub samples: Vec<Option<Vec<Byte>>>,
}
#[repr(i16)]
#[derive(Clone, Copy, PartialEq, Eq)]
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

impl Display for OctamedMMD0InstrumentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OctamedMMD0InstrumentType::Hybrid => write!(f, "Hybrid"),
            OctamedMMD0InstrumentType::Synth => write!(f, "Synth"),

            _ => {
                write!(f, "{}", match *self {
                    OctamedMMD0InstrumentType::Sample => "1 Octave",
                    OctamedMMD0InstrumentType::SampleOct5 => "5 Octave",
                    OctamedMMD0InstrumentType::SampleOct3 => "3 Octave",
                    OctamedMMD0InstrumentType::SampleOct2 => "2 Octave",
                    OctamedMMD0InstrumentType::SampleOct4 => "4 Octave",
                    OctamedMMD0InstrumentType::SampleOct6 => "6 Octave",
                    OctamedMMD0InstrumentType::SampleOct7 => "7 Octave",
                    OctamedMMD0InstrumentType::ExtSample => "9 Octave",
                    _ => unreachable!(),
                })?;
                write!(f, " Sample")
            }
        }
    }
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

pub struct OctamedMMD0Expansion {
    pub header: OctamedMMD0ExpansionHeader,
    pub external_instruments: Vec<OctamedMMD0ExternalInstrument>,
    pub annotation: String,
    pub instrument_infos: Vec<OctamedMMD0InstrumentInfo>,
    pub color_pallete: OctamedMMD0ColorPallete,
    pub notation_info: OctamedMMD0NotationInfo,
    pub song_name: String,
    pub mmd_dump: OctamedMMD0Dump,
    pub mmd_info: OctamedMMD0Info,
    pub mmd_rexx: OctamedMMD0Rexx,
    pub mmd_midi_commands: OctamedMMD0MidiCommands,
}
pub struct OctamedMMD0ExpansionHeader {
    pub next_module_ptr: Offset,
    pub expanded_instruments_array_ptr: Offset,
    pub expanded_instruments_array_length: UWord,
    pub extpanded_instruments_struct_size: UWord,
    pub annotation_text_char_array_ptr: Offset,
    pub annotation_text_length: ULong, // includes null terminator
    pub instrument_info_ptr: Offset,
    pub instrument_info_array_length: UWord,
    pub instrument_info_struct_size: UWord,
    pub jump_mask: ULong, // obsolete
    pub rgb_table_ptr: Offset, // 8 uword array pointer
    pub channel_split: [UByte; 4],
    pub notation_info_ptr: Offset,
    pub song_name_char_array_ptr: Offset,
    pub song_name_length: ULong,
    pub mmd_dump_ptr: Offset,
    pub mmd_info_ptr: Offset,
    pub mmd_rexx_ptr: Offset,
    pub mmd_midi_commands_ptr: Offset,
    pub reserved: [ULong; 3],
    pub tag_end: ULong,
}

pub struct OctamedMMD0ExternalInstrument {
    pub hold: UByte,
    pub decay: UByte,
    pub supress_midi_off: UByte,
    pub fine_tune: Byte,

    /* Below fields saved by >= V5 
        UBYTE default_pitch;
        UBYTE instr_flags;
        UWORD long_midi_preset;
        /* Below fields saved by >= V5.02 */
        UBYTE output_device;
        UBYTE reserved;
        /* Below fields saved by >= V7 */
        ULONG long_repeat;
        ULONG long_replen;
                        */
}
pub struct OctamedMMD0InstrumentInfo {
    pub name: String,
}

pub struct OctamedMMD0ColorPallete {
    pub colors: [OctamedMMD0Color; 8],
}
impl OctamedMMD0ColorPallete {
    pub fn from_bytes(bytes: [UWord; 8]) -> Self {
        let mut colors = [OctamedMMD0Color { value: UWord(0) }; 8];
        for i in 0..bytes.len() {
            colors[i] = OctamedMMD0Color { value: bytes[i] };
        }
        Self { colors }
    }
}
#[derive(Clone, Copy)]
pub struct OctamedMMD0Color {
    pub value: UWord,
}
impl OctamedMMD0Color {
    const R_MASK: u16 = 0x0f00;
    const G_MASK: u16 = 0x00f0;
    const B_MASK: u16 = 0x000f;
    /// The rgb values in in half byte accuracy, where 0 = 0 and 15 = 255 / lowest - maximum brightness
    pub fn as_rgb_4(&self) -> (UByte, UByte, UByte) {
        let r = (self.value.0 & Self::R_MASK) >> 8;
        let g = (self.value.0 & Self::G_MASK) >> 4;
        let b = self.value.0 & Self::B_MASK;

        return (
            UByte(((r * 255) / 15) as u8),
            UByte(((g * 255) / 15) as u8),
            UByte(((b * 255) / 15) as u8),
        );
    }
}

//TODO
pub struct OctamedMMD0NotationInfo {
    pub sharp_count: UByte,
    pub flags: UByte,
    pub selected_tracks: [UWord; 5],
    pub shown_tracks: [UByte; 16],
    pub ghosted_tracks: [UByte; 16],
    pub note_transposes: [Byte; 63],
    pub _pad: UByte,
}
impl OctamedMMD0NotationInfo {
    pub fn new() -> Self {
        return Self {
            sharp_count: UByte(0),
            flags: UByte(0),
            selected_tracks: [UWord(0); 5],
            shown_tracks: [UByte(0); 16],
            ghosted_tracks: [UByte(0); 16],
            note_transposes: [Byte(0); 63],
            _pad: UByte(0),
        };
    }
}
pub struct OctamedMMD0Dump {}
pub struct OctamedMMD0Info {}
pub struct OctamedMMD0Rexx {}
pub struct OctamedMMD0MidiCommands {}
//
pub struct OctamedMMD0SongFlags(pub UByte, pub UByte);

impl OctamedMMD0SongFlags {
    //byte1
    const FILTER_ON_FLAG: u8 = 0x01;
    const JUMP_ON_FLAG: u8 = 0x02;
    const JUMP_EIGHT_FLAG: u8 = 0x04;
    const SONG_SAMPLES_MODULE_FLAG: u8 = 0x08;
    const VOLUMES_ARE_HEX_FLAG: u8 = 0x10;
    const USE_ST_SLIDING_FLAG: u8 = 0x20;
    const IS_EIGHT_CHANNEL_FLAG: u8 = 0x40;
    const IS_HQ_V2_COMPATIBILITY_FLAG: u8 = 0x80;
    const BPM_BEAT_LENGTH_MASK: u8 = 0x1f;
    //byte2
    const IS_BPM_MODE_FLAG: u8 = 0x20;
    const MIXING_ENABLED_FLAG: u8 = 0x80;

    pub fn from_bytes(byte1: UByte, byte2: UByte) -> Self {
        return Self(byte1, byte2);
    }

    pub fn filter_is_on(&self) -> bool {
        return bit_flag(self.0, Self::FILTER_ON_FLAG);
    }
    pub fn jumping_is_on(&self) -> bool {
        return bit_flag(self.0, Self::JUMP_ON_FLAG);
    }
    pub fn jump_every_eight_lines(&self) -> bool {
        return bit_flag(self.0, Self::JUMP_EIGHT_FLAG);
    }
    pub fn song_samples_indicator(&self) -> bool {
        return bit_flag(self.0, Self::SONG_SAMPLES_MODULE_FLAG);
    }
    pub fn volumes_are_hex(&self) -> bool {
        return bit_flag(self.0, Self::VOLUMES_ARE_HEX_FLAG);
    }
    pub fn use_st_sliding(&self) -> bool {
        return bit_flag(self.0, Self::USE_ST_SLIDING_FLAG);
    }
    pub fn is_8_channels(&self) -> bool {
        return bit_flag(self.0, Self::IS_EIGHT_CHANNEL_FLAG);
    }
    pub fn is_hq_v2_compatability(&self) -> bool {
        return bit_flag(self.0, Self::IS_HQ_V2_COMPATIBILITY_FLAG);
    }
    pub fn bpm_beat_length(&self) -> UByte {
        return UByte(self.1.0 & Self::BPM_BEAT_LENGTH_MASK);
    }
    pub fn is_bpm_mode(&self) -> bool {
        return bit_flag(self.1, Self::IS_BPM_MODE_FLAG);
    }
    pub fn mixing_enabled(&self) -> bool {
        return bit_flag(self.1, Self::MIXING_ENABLED_FLAG);
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
