use core::panic;
use std::{ collections::HashMap, fs::File, io::{ Read, Seek }, path::Path };

use crate::{
    mmd0::module::{
        OctamedMMD0,
        OctamedMMD0Block,
        OctamedMMD0BlockHeader,
        OctamedMMD0BlockLine,
        OctamedMMD0BlockTable,
        OctamedMMD0ColorPallete,
        OctamedMMD0Dump,
        OctamedMMD0Expansion,
        OctamedMMD0ExpansionHeader,
        OctamedMMD0Header,
        OctamedMMD0Info,
        OctamedMMD0InstrumentInfo,
        OctamedMMD0InstrumentType,
        OctamedMMD0MidiCommands,
        OctamedMMD0NotationInfo,
        OctamedMMD0Rexx,
        OctamedMMD0Sample,
        OctamedMMD0SampleHeader,
        OctamedMMD0SampleTable,
        OctamedMMD0Song,
        OctamedMMD0SongFlags,
        OctamedMMD0TrackLine,
    },
    utility::bytes::{ Byte, Offset, UByte, ULong, UWord, Word, bit_flag },
};

type Result<T> = std::io::Result<T>;
pub struct OctamedMMD0Parser;

impl OctamedMMD0Parser {
    pub fn parse_module<R: Read + Seek>(stream: &mut R, offset: Offset) -> Result<OctamedMMD0> {
        stream.seek(offset.into())?;
        let header = Self::parse_header_mmd0(stream)?;
        if
            header.song_ptr.is_null() ||
            header.block_array_ptr.is_null() ||
            header.sample_array_ptr.is_null() ||
            header.expansion_data_ptr.is_null()
        {
            panic!("Null pointer in file");
        }
        let song = Self::parse_song_mmd0(header.song_ptr, stream)?;

        let block_table = Self::parse_blocks(header.block_array_ptr, stream, &song)?;
        let sample_table = Self::parse_sample_table(header.sample_array_ptr, &song, stream)?;
        let expansion_data = Self::parse_expansion_data(header.expansion_data_ptr, stream)?;
        return Ok(OctamedMMD0 { song, block_table, header, sample_table, expansion_data });
    }

    pub fn parse_file(path: &Path) -> Result<Vec<OctamedMMD0>> {
        let mut file = File::open(path)?;
        let mut modules = vec![];
        let mut module = Self::parse_module(&mut file, Offset(0))?;
        loop {
            let extra_songs = module.header.extra_songs.0;
            modules.push(module);

            if extra_songs > 0 {
                //todo parse others
                // module = Self:: etc.
                module = Self::parse_module(&mut file, Offset(1))?;
            } else {
                break;
            }
        }
        return Ok(modules);
    }
    fn parse_ulong<R: Read>(stream: &mut R) -> Result<ULong> {
        let mut bytes = [0 as u8; 4];
        stream.read_exact(&mut bytes)?;

        return Ok(ULong(u32::from_be_bytes(bytes)));
    }
    fn parse_offset<R: Read>(stream: &mut R) -> Result<Offset> {
        let val = Self::parse_ulong(stream)?;
        return Ok(Offset(val.0));
    }
    fn parse_uword<R: Read>(stream: &mut R) -> Result<UWord> {
        let mut bytes = [0 as u8; 2];
        stream.read_exact(&mut bytes)?;

        return Ok(UWord(u16::from_be_bytes(bytes)));
    }
    fn parse_ubyte<R: Read>(stream: &mut R) -> Result<UByte> {
        let mut bytes = [0 as u8; 1];
        stream.read_exact(&mut bytes)?;

        return Ok(UByte(u8::from_be_bytes(bytes)));
    }
    fn parse_byte<R: Read>(stream: &mut R) -> Result<Byte> {
        let ubyte = Self::parse_ubyte(stream)?;
        let byte = Byte(ubyte.0.cast_signed());
        return Ok(byte);
    }
    fn parse_word<R: Read>(stream: &mut R) -> Result<Word> {
        let mut bytes = [0 as u8; 2];
        stream.read_exact(&mut bytes)?;

        return Ok(Word(i16::from_be_bytes(bytes)));
    }
    fn parse_exact<R: Read>(stream: &mut R, count: usize) -> Result<Vec<u8>> {
        let mut bytes = vec![0 as u8; count];
        stream.read_exact(&mut bytes)?;

        return Ok(bytes.to_vec());
    }
    pub fn parse_header_mmd0<R: Read + Seek>(stream: &mut R) -> Result<OctamedMMD0Header> {
        let id = Self::parse_ulong(stream)?;
        let length = Self::parse_ulong(stream)?;

        let song_ptr: Offset = Self::parse_offset(stream)?;
        let player_seconds_num = Self::parse_uword(stream)?;
        let player_sequence = Self::parse_uword(stream)?;

        let block_array_ptr: Offset = Self::parse_offset(stream)?;
        let flags = super::module::OctamedMMD0HeaderFlags::from_byte(Self::parse_ubyte(stream)?);
        let reserved = Self::parse_exact(stream, 3)?;
        let mut reserved_buffer = [0 as u8; 3];
        for (i, byte) in reserved.iter().enumerate() {
            reserved_buffer[i] = *byte;
        }
        let reserved = reserved_buffer;
        let sample_array_ptr = Self::parse_offset(stream)?;
        let reserved2 = Self::parse_ulong(stream)?;
        let expansion_data_ptr = Self::parse_offset(stream)?;
        let reserved3 = Self::parse_ulong(stream)?;

        let player_state = Self::parse_uword(stream)?;
        let player_block = Self::parse_uword(stream)?;
        let player_line = Self::parse_uword(stream)?;
        let player_sequence_num = Self::parse_uword(stream)?;

        let actual_play_line = Self::parse_word(stream)?;
        let counter = Self::parse_ubyte(stream)?;
        let extra_songs = Self::parse_ubyte(stream)?;
        let header = OctamedMMD0Header {
            id,
            module_length: length,
            song_ptr,
            player_seconds_num,
            player_sequence,
            block_array_ptr,
            flags,
            reserved,
            sample_array_ptr,
            reserved2,
            expansion_data_ptr,
            reserved3,
            player_state,
            player_block,
            player_line,
            player_sequence_num,
            actual_play_line,
            counter,
            extra_songs,
        };
        return Ok(header);
    }
    fn parse_song_mmd0<R: Read + Seek>(
        song_offset: Offset,
        stream: &mut R
    ) -> Result<OctamedMMD0Song> {
        stream.seek(song_offset.into())?;
        let mut samples = [OctamedMMD0Sample::new(); 63];
        Self::parse_samples(&mut samples, stream)?;
        let block_count = Self::parse_uword(stream)?;
        let song_length = Self::parse_uword(stream)?;
        let mut player_sequence_list_bytes = [0 as u8; 256];
        stream.read_exact(&mut player_sequence_list_bytes)?;
        let mut player_sequence_list = [UByte(0); 256];
        for i in 0..player_sequence_list_bytes.len() {
            player_sequence_list[i] = UByte(player_sequence_list_bytes[i]);
        }

        let default_song_tempo = Self::parse_uword(stream)?;
        let global_transpose: Byte = Self::parse_ubyte(stream)?.into();
        let flags_byte = Self::parse_ubyte(stream)?;
        let flags2_byte = Self::parse_ubyte(stream)?;
        let flags = OctamedMMD0SongFlags::from_bytes(flags_byte, flags2_byte);
        let pulses_per_line = Self::parse_ubyte(stream)?;
        let mut track_volumes = [UByte(0); 16];
        for i in 0..track_volumes.len() {
            let vol = Self::parse_ubyte(stream)?;
            track_volumes[i] = vol;
        }
        let master_volume = Self::parse_ubyte(stream)?;
        let sample_count = Self::parse_ubyte(stream)?;
        return Ok(OctamedMMD0Song {
            samples,
            block_count,
            song_length,
            player_sequence_list,
            primary_tempo: default_song_tempo,
            global_transpose,
            flags,
            secondary_tempo: pulses_per_line,
            track_volumes,
            master_volume,
            sample_count,
        });
    }
    fn parse_samples<R: Read + Seek>(
        buf: &mut [OctamedMMD0Sample; 63],
        stream: &mut R
    ) -> Result<()> {
        for i in 0..63 {
            let mut buffer = [0 as u8; 8];
            stream.read_exact(&mut buffer)?;
            let repeat = buffer[0..2].try_into().unwrap();
            let repeat = UWord(u16::from_be_bytes(repeat));

            let repeat_length = buffer[2..4].try_into().unwrap();
            let repeat_length = UWord(u16::from_be_bytes(repeat_length));

            let midi_channel = UByte(buffer[4]);
            let midi_preset = UByte(buffer[5]);
            let sample_volume = UByte(buffer[6]);
            let sample_transpose = Byte(buffer[7].cast_signed());
            buf[i] = OctamedMMD0Sample {
                repeat,
                repeat_length,
                midi_channel,
                midi_preset,
                sample_volume,
                sample_transpose,
            };
        }
        return Ok(());
    }
    fn parse_sample_table<R: Read + Seek>(
        sample_offset: Offset,
        song: &OctamedMMD0Song,
        stream: &mut R
    ) -> Result<OctamedMMD0SampleTable> {
        stream.seek(sample_offset.into())?;
        let mut instrument_ptrs = vec![];
        for _ in 0..song.sample_count.0 {
            let offset = Self::parse_offset(stream)?;
            instrument_ptrs.push(offset);
        }
        let mut sample_table = OctamedMMD0SampleTable { headers: vec![], samples: vec![] };
        for offset in instrument_ptrs {
            stream.seek(offset.into())?;
            let sample_length = Self::parse_ulong(stream)?;
            let sample_type: Word = Self::parse_word(stream)?;
            let is_stereo = bit_flag(sample_type, 0x20);
            let is_16_bit = bit_flag(sample_type, 0x10);
            let is_sample = sample_type.0 >= 0;
            let sample_length = if !is_stereo { sample_length } else { ULong(sample_length.0 * 2) };
            if sample_length.0 == 0 && is_sample {
                //null instrument
                sample_table.headers.push(None);
                sample_table.samples.push(None);
                continue;
            }
            let sample_type = OctamedMMD0InstrumentType::from_word(sample_type);
            let samples = Self::parse_exact(stream, sample_length.0 as usize)?
                .iter()
                .map(|s| Byte(*s as i8))
                .collect();
            sample_table.headers.push(
                Some(OctamedMMD0SampleHeader { sample_length, sample_type, is_16_bit, is_stereo })
            );
            if is_sample {
                sample_table.samples.push(Some(samples));
            } else {
                //todo where are synths stored
            }
        }
        return Ok(sample_table);
    }
    fn parse_blocks<R: Read + Seek>(
        blocks_offset: Offset,
        stream: &mut R,
        song: &OctamedMMD0Song
    ) -> Result<OctamedMMD0BlockTable> {
        stream.seek(blocks_offset.into())?;
        let mut block_pointers = vec![];
        for _ in 0..song.block_count.0 {
            let offset = Self::parse_offset(stream)?;
            block_pointers.push(offset);
        }
        let mut table = OctamedMMD0BlockTable { blocks: vec![], headers: vec![] };
        for ptr in block_pointers {
            stream.seek(ptr.into())?;
            let track_count = Self::parse_ubyte(stream)?;
            let line_count = Self::parse_ubyte(stream)?;
            let header = OctamedMMD0BlockHeader { track_count, line_count };
            let mut block = OctamedMMD0Block { lines: vec![] };
            for i in 0..line_count.0 + 1 {
                let mut block_line = OctamedMMD0BlockLine { tracks: vec![] };
                for track in 0..track_count.0 {
                    let byte1 = Self::parse_ubyte(stream)?;
                    let byte2 = Self::parse_ubyte(stream)?;
                    let byte3 = Self::parse_ubyte(stream)?;
                    let track_line = OctamedMMD0TrackLine::from_bytes(byte1, byte2, byte3);
                    block_line.tracks.push(track_line);
                }
                block.lines.push(block_line);
            }
            table.headers.push(header);
            table.blocks.push(block);
            //todo
        }
        return Ok(table);
    }
    fn parse_expansion_data<R: Read + Seek>(
        expansion_offset: Offset,
        stream: &mut R
    ) -> Result<Option<OctamedMMD0Expansion>> {
        if expansion_offset.is_null() {
            return Ok(None);
        }
        stream.seek(expansion_offset.into())?;
        let header = Self::parse_expansion_data_header(stream)?;

        let annotation = {
            if
                header.annotation_text_char_array_ptr.is_null() ||
                header.annotation_text_length.0 <= 1
            {
                "N/A".into()
            } else {
                stream.seek(header.annotation_text_char_array_ptr.into())?;
                let mut text = String::with_capacity(
                    (header.annotation_text_length.0 - 1) as usize
                );
                for i in 0..header.annotation_text_length.0 - 1 {
                    let c = Self::parse_ubyte(stream)?.as_char();
                    text.push(c);
                }
                text
            }
        };
        let color_pallete = {
            stream.seek(header.rgb_table_ptr.into())?;
            let mut pallete = [UWord(0); 8];
            for i in 0..pallete.len() {
                let color = Self::parse_uword(stream)?;
                pallete[i] = color;
            }
            OctamedMMD0ColorPallete::from_bytes(pallete)
        };
        let external_instruments = vec![];
        let instrument_infos = {
            if header.instrument_info_ptr.is_null() {
                vec![]
            } else {
                stream.seek(header.instrument_info_ptr.into())?;
                let mut infos = Vec::new();
                if header.instrument_info_struct_size.0 != 40 {
                    panic!("instrument_info struct size: {}", header.instrument_info_struct_size);
                }
                for i in 0..header.instrument_info_array_length.0 {
                    let mut info = OctamedMMD0InstrumentInfo { name: String::with_capacity(40) };
                    for j in 0..header.instrument_info_struct_size.0 - 1 {
                        let c = Self::parse_ubyte(stream)?.as_char();
                        info.name.push(c);
                    }
                    Self::parse_byte(stream)?; // \0
                    infos.push(info);
                }
                infos
            }
        };
        let mmd_dump = OctamedMMD0Dump {};
        let mmd_info = OctamedMMD0Info {};
        let mmd_midi_commands = OctamedMMD0MidiCommands {};
        let mmd_rexx = OctamedMMD0Rexx {};
        let notation_info = OctamedMMD0NotationInfo {};
        let song_name = {
            if header.song_name_char_array_ptr.is_null() || header.song_name_length.0 <= 1 {
                "N/A".into()
            } else {
                let mut name = String::with_capacity((header.song_name_length.0 - 1) as usize);
                stream.seek(header.song_name_char_array_ptr.into())?;
                for i in 0..header.song_name_length.0 - 1 {
                    let c = Self::parse_ubyte(stream)?.as_char();
                    name.push(c);
                }
                name
            }
        };

        return Ok(
            Some(OctamedMMD0Expansion {
                header,
                annotation,
                color_pallete,
                external_instruments,
                instrument_infos,
                mmd_dump,
                mmd_info,
                mmd_midi_commands,
                mmd_rexx,
                notation_info,
                song_name,
            })
        );
    }
    fn parse_expansion_data_header<R: Read + Seek>(
        stream: &mut R
    ) -> Result<OctamedMMD0ExpansionHeader> {
        let next_module_ptr = Self::parse_offset(stream)?;
        let expanded_instruments_array_ptr = Self::parse_offset(stream)?;
        let expanded_instruments_array_length = Self::parse_uword(stream)?;
        let extpanded_instruments_struct_size = Self::parse_uword(stream)?;
        let annotation_text_char_array_ptr = Self::parse_offset(stream)?;
        let annotation_text_length = Self::parse_ulong(stream)?;
        let instrument_info_ptr = Self::parse_offset(stream)?;
        let instrument_info_array_length = Self::parse_uword(stream)?;
        let instrument_info_struct_size = Self::parse_uword(stream)?;
        let jump_mask = Self::parse_ulong(stream)?;
        let rgb_table_ptr = Self::parse_offset(stream)?;
        let channel_split: [UByte; 4] = Self::parse_exact(stream, 4)?
            .iter()
            .map(|b| UByte(*b))
            .collect::<Vec<UByte>>()
            .try_into()
            .unwrap();
        let notation_info_ptr = Self::parse_offset(stream)?;
        let song_name_char_array_ptr = Self::parse_offset(stream)?;
        let song_name_length = Self::parse_ulong(stream)?;
        let mmd_dump_ptr = Self::parse_offset(stream)?;
        let mmd_info_ptr = Self::parse_offset(stream)?;
        let mmd_rexx_ptr = Self::parse_offset(stream)?;
        let mmd_midi_commands_ptr = Self::parse_offset(stream)?;
        let mut reserved = [ULong(0); 3];
        for i in 0..reserved.len() {
            reserved[i] = Self::parse_ulong(stream)?;
        }
        let tag_end = Self::parse_ulong(stream)?;

        return Ok(OctamedMMD0ExpansionHeader {
            next_module_ptr,
            expanded_instruments_array_ptr,
            expanded_instruments_array_length,
            extpanded_instruments_struct_size,
            annotation_text_char_array_ptr,
            annotation_text_length,
            instrument_info_ptr,
            instrument_info_array_length,
            instrument_info_struct_size,
            jump_mask,
            rgb_table_ptr,
            channel_split,
            notation_info_ptr,
            song_name_char_array_ptr,
            song_name_length,
            mmd_dump_ptr,
            mmd_info_ptr,
            mmd_rexx_ptr,
            mmd_midi_commands_ptr,
            reserved,
            tag_end,
        });
    }
}
