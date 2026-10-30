use std::fmt::Display;

use octamedic_util::base32;

use crate::utility::{
    bytes::{Byte, Offset, UByte, ULong, UWord, Word, bit_flag},
    note,
    octamed_tempo::OctamedTempo,
};

/// Instrument slots in a module.
pub const MAX_INSTRUMENTS: usize = 63;

/// Tracks the song keeps a volume and notation settings for.
pub const SONG_TRACKS: usize = 16;

/// Most tracks an MMD1 block can have.
pub const MAX_TRACKS: usize = 64;

pub const MMD0_ID: u32 = u32::from_be_bytes(*b"MMD0");
pub const MMD1_ID: u32 = u32::from_be_bytes(*b"MMD1");
pub const MMD2_ID: u32 = u32::from_be_bytes(*b"MMD2");
pub const MMD3_ID: u32 = u32::from_be_bytes(*b"MMD3");

pub const SEQUENCE_LENGTH: usize = 256;

pub const PALETTE_COLORS: usize = 8;

pub const AMIGA_CHANNELS: usize = 4;

/// How long an instrument's name usually is, without its closing NUL.
pub const INSTRUMENT_NAME_LENGTH: usize = 40;

// unused space in the structures, kept so they're written back the same
pub const HEADER_RESERVED: usize = 3;
pub const BLOCK_INFO_RESERVED: usize = 5;
pub const EXPANSION_RESERVED: usize = 3;

pub const NOTATION_SELECTED_TRACK_WORDS: usize = 5;

pub enum OctamedMMDType {
    MMD0,
    MMD1,
    MMD2,
    MMD3,
}

impl OctamedMMD0Header {
    /// Bytes in the file.
    pub const SIZE: usize = 52;
}

pub struct OctamedMMD0Header {
    pub id: ULong,
    pub module_length: ULong,
    pub song_ptr: Offset,
    pub player_seconds_num: UWord,
    pub player_sequence: UWord,
    pub block_array_ptr: Offset,
    pub flags: OctamedMMD0HeaderFlags,
    pub reserved: [UByte; HEADER_RESERVED],
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
        return Self {
            load_to_fast_memory: bit_flag(byte, Self::LOAD_TO_FAST_MEM_FLAG),
        };
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
        writeln!(f, "Format: {}", String::from_utf8_lossy(&id_bytes))?;
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
    /// `None` for a module built in memory with an unknown id; parsed modules always have one.
    pub fn get_type(&self) -> Option<OctamedMMDType> {
        return match self.header.id.0 {
            MMD0_ID => Some(OctamedMMDType::MMD0),
            MMD1_ID => Some(OctamedMMDType::MMD1),
            MMD2_ID => Some(OctamedMMDType::MMD2),
            MMD3_ID => Some(OctamedMMDType::MMD3),
            _ => None,
        };
    }
}

pub struct OctamedMMD0Song {
    pub samples: [OctamedMMD0Sample; MAX_INSTRUMENTS],
    pub block_count: UWord,
    pub song_length: UWord,
    pub player_sequence_list: [UByte; SEQUENCE_LENGTH],
    pub primary_tempo: UWord,
    pub global_transpose: Byte,
    pub flags: OctamedMMD0SongFlags,
    pub secondary_tempo: UByte,
    pub track_volumes: [UByte; SONG_TRACKS],
    pub master_volume: UByte,
    pub sample_count: UByte,
}

impl OctamedMMD0Song {
    /// Bytes in the file.
    pub const SIZE: usize = 788;

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
        writeln!(f, "Track Volumes: {:?}", self.track_volumes.map(|b| b.0))?;
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
    /// Bytes in the file: repeat and repeat length, MIDI channel and preset, volume, transpose.
    pub const SIZE: usize = 8;

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

impl Default for OctamedMMD0Sample {
    fn default() -> Self {
        return Self::new();
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
    /// Bytes a track takes on a line: note, instrument and command packed in 3 in MMD0, and a
    /// byte each in MMD1.
    pub const MMD0_SIZE: usize = 3;

    pub const MMD1_SIZE: usize = 4;

    pub const BLOCK_LINE_COMMAND_NUMBER_MASK_MMD0: u8 = 0x0f;

    pub const BLOCK_LINE_NOTE_NUMBER_MASK_MMD0: u8 = 0x3f;

    pub const BLOCK_LINE_INSTRUMENT_NUMBER_MASK_MMD1: u8 = 0x3f;

    pub const BLOCK_LINE_NOTE_NUMBER_MASK_MMD1: u8 = 0x7f;

    /// In MMD0, the note takes byte 1's low 6 bits, and the instrument's high bits the rest.
    pub const MMD0_NOTE_BITS: u32 = 6;

    /// In MMD0, the instrument's low bits take byte 2's high 4.
    pub const MMD0_INSTRUMENT_LOW_BITS: u32 = 4;

    pub fn from_bytes_mmd0(byte1: UByte, byte2: UByte, byte3: UByte) -> Self {
        let command_value = byte3;
        let command_number = UByte(byte2.0 & Self::BLOCK_LINE_COMMAND_NUMBER_MASK_MMD0);
        let note_number = UByte(byte1.0 & Self::BLOCK_LINE_NOTE_NUMBER_MASK_MMD0);
        // leftmost 2 bits of byte 1 need to be prepended to byte 2 to get the value for the instrument number
        let xy = (byte1.0 >> Self::MMD0_NOTE_BITS) << Self::MMD0_INSTRUMENT_LOW_BITS;
        let iiii = byte2.0 >> Self::MMD0_INSTRUMENT_LOW_BITS;
        let instrument_number = UByte(xy | iiii);
        return Self {
            note_number,
            instrument_number,
            command_number,
            command_value,
        };
    }

    pub fn from_bytes_mmd1(byte1: UByte, byte2: UByte, byte3: UByte, byte4: UByte) -> Self {
        let command_value = byte4;
        let command_number = byte3;
        let note_number = UByte(byte1.0 & Self::BLOCK_LINE_NOTE_NUMBER_MASK_MMD1);
        let instrument_number = UByte(byte2.0 & Self::BLOCK_LINE_INSTRUMENT_NUMBER_MASK_MMD1);
        return Self {
            note_number,
            instrument_number,
            command_number,
            command_value,
        };
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
    /// Two base-32 digits with a space for a leading zero, as OctaMED shows instruments.
    fn as_inst_string(&self) -> String {
        if self.0 as usize > MAX_INSTRUMENTS {
            return "??".to_owned();
        }
        return base32::format(self.0 as u32, 2, ' ');
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
    pub reserved: [ULong; BLOCK_INFO_RESERVED],
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
                write!(
                    f,
                    "{}",
                    match *self {
                        OctamedMMD0InstrumentType::Sample => "1 Octave",
                        OctamedMMD0InstrumentType::SampleOct5 => "5 Octave",
                        OctamedMMD0InstrumentType::SampleOct3 => "3 Octave",
                        OctamedMMD0InstrumentType::SampleOct2 => "2 Octave",
                        OctamedMMD0InstrumentType::SampleOct4 => "4 Octave",
                        OctamedMMD0InstrumentType::SampleOct6 => "6 Octave",
                        OctamedMMD0InstrumentType::SampleOct7 => "7 Octave",
                        OctamedMMD0InstrumentType::ExtSample => "9 Octave",
                        _ => unreachable!(),
                    }
                )?;
                write!(f, " Sample")
            }
        }
    }
}

impl OctamedMMD0InstrumentType {
    pub fn from_word(word: Word) -> Option<Self> {
        return Some(match word.0 {
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
            _ => return None,
        });
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

impl OctamedMMD0ExpansionHeader {
    /// Bytes in the file.
    pub const SIZE: usize = 84;
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
    pub jump_mask: ULong,      // obsolete
    pub rgb_table_ptr: Offset, // 8 uword array pointer
    pub channel_split: [UByte; AMIGA_CHANNELS],
    pub notation_info_ptr: Offset,
    pub song_name_char_array_ptr: Offset,
    pub song_name_length: ULong,
    pub mmd_dump_ptr: Offset,
    pub mmd_info_ptr: Offset,
    pub mmd_rexx_ptr: Offset,
    pub mmd_midi_commands_ptr: Offset,
    pub reserved: [ULong; EXPANSION_RESERVED],
    pub tag_end: ULong,
}

/// An `InstrExt` entry. v4 writes only these four fields; later versions add more, which are kept
/// as they are in `rest`.
pub struct OctamedMMD0ExternalInstrument {
    pub hold: UByte,
    pub decay: UByte,
    pub supress_midi_off: UByte,
    /// -8 to 7
    pub fine_tune: Byte,
    pub rest: Vec<UByte>,
}

impl OctamedMMD0ExternalInstrument {
    pub const V4_SIZE: usize = 4;

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![
            self.hold.0,
            self.decay.0,
            self.supress_midi_off.0,
            self.fine_tune.0 as u8,
        ];
        bytes.extend(self.rest.iter().map(|b| b.0));
        return bytes;
    }
}

pub struct OctamedMMD0InstrumentInfo {
    pub name: String,
}

pub struct OctamedMMD0ColorPallete {
    pub colors: [OctamedMMD0Color; PALETTE_COLORS],
}

impl OctamedMMD0ColorPallete {
    pub fn from_bytes(bytes: [UWord; PALETTE_COLORS]) -> Self {
        let mut colors = [OctamedMMD0Color { value: UWord(0) }; PALETTE_COLORS];
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
    /// The bits a colour uses: 4 each for red, green and blue.
    pub const MASK: u16 = 0x0fff;

    const R_SHIFT: u32 = 8;

    const G_SHIFT: u32 = 4;

    const B_SHIFT: u32 = 0;

    const CHANNEL_MAX: u16 = 0xf;

    /// The rgb values in in half byte accuracy, where 0 = 0 and 15 = 255 / lowest - maximum brightness
    pub fn as_rgb_4(&self) -> (UByte, UByte, UByte) {
        let [r, g, b] = Self::rgb(self.value.0);
        return (UByte(r), UByte(g), UByte(b));
    }

    /// An Amiga colour word as 8-bit red, green and blue, with each 4-bit channel stretched
    /// to the full range.
    pub fn rgb(color: u16) -> [u8; 3] {
        let channel = |shift: u32| {
            let level = (color >> shift) & Self::CHANNEL_MAX;
            (level * u8::MAX as u16 / Self::CHANNEL_MAX) as u8
        };
        return [
            channel(Self::R_SHIFT),
            channel(Self::G_SHIFT),
            channel(Self::B_SHIFT),
        ];
    }
}

//TODO
pub struct OctamedMMD0NotationInfo {
    pub sharp_count: UByte,
    pub flags: UByte,
    pub selected_tracks: [UWord; NOTATION_SELECTED_TRACK_WORDS],
    pub shown_tracks: [UByte; SONG_TRACKS],
    pub ghosted_tracks: [UByte; SONG_TRACKS],
    pub note_transposes: [Byte; MAX_INSTRUMENTS],
    pub _pad: UByte,
}

impl OctamedMMD0NotationInfo {
    pub fn new() -> Self {
        return Self {
            sharp_count: UByte(0),
            flags: UByte(0),
            selected_tracks: [UWord(0); NOTATION_SELECTED_TRACK_WORDS],
            shown_tracks: [UByte(0); SONG_TRACKS],
            ghosted_tracks: [UByte(0); SONG_TRACKS],
            note_transposes: [Byte(0); MAX_INSTRUMENTS],
            _pad: UByte(0),
        };
    }
}

impl Default for OctamedMMD0NotationInfo {
    fn default() -> Self {
        return Self::new();
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

    // in lines, 1-32; the file stores one less, so 0 means 1 line
    pub fn bpm_beat_length(&self) -> UByte {
        return UByte((self.1.0 & Self::BPM_BEAT_LENGTH_MASK) + 1);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpm_beat_length_is_one_more_than_the_stored_bits() {
        let flags = |flags2: u8| OctamedMMD0SongFlags::from_bytes(UByte(0), UByte(flags2));
        assert_eq!(flags(0x00).bpm_beat_length(), UByte(1));
        assert_eq!(flags(0x23).bpm_beat_length(), UByte(4));
        assert_eq!(flags(0x2F).bpm_beat_length(), UByte(16));
        assert_eq!(flags(0x1F).bpm_beat_length(), UByte(32));
    }
}
