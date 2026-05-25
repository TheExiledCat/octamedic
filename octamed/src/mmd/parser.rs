use core::panic;
use std::{ fs::File, io::{ Read, Seek }, path::Path };

use crate::{
    mmd::module::{
        OctamedMMD,
        OctamedMMD0Block,
        OctamedMMD0BlockHeader,
        OctamedMMD0BlockLine,
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
        OctamedMMDTrackLine,
        OctamedMMD1Block,
        OctamedMMD2BlockCommandPageTable,
        OctamedMMD1BlockHeader,
        OctamedMMD1BlockInfo,
        OctamedMMD1BlockInfoHeader,
        OctamedMMD1HighlightMask,
        OctamedMMDBlockTable,
    },
    utility::bytes::{ Byte, Offset, UByte, ULong, UWord, Word, bit_flag },
};

type Result<T> = std::io::Result<T>;
enum OctamedModuleKind {
    MMD0,
    MMD1,
    MMD2,
    MMD3,
}
impl OctamedModuleKind {
    const MMD0_ID: u32 = 0x4d4d4430;
    const MMD1_ID: u32 = 0x4d4d4431;
    const MMD2_ID: u32 = 0x4d4d4432;
    const MMD3_ID: u32 = 0x4d4d4433;
}
impl From<ULong> for OctamedModuleKind {
    fn from(value: ULong) -> Self {
        let value = value.0;
        if value == Self::MMD0_ID {
            return Self::MMD0;
        }
        if value == Self::MMD1_ID {
            return Self::MMD1;
        }
        if value == Self::MMD2_ID {
            return Self::MMD2;
        }
        if value == Self::MMD3_ID {
            return Self::MMD3;
        }
        panic!("Unknown mmd format: {}", value);
    }
}
pub struct OctamedMMDParser {
    mode: OctamedModuleKind,
}

impl OctamedMMDParser {
    pub fn new() -> Self {
        return Self { mode: OctamedModuleKind::MMD0 };
    }
    pub fn parse_module<R: Read + Seek>(
        &mut self,
        stream: &mut R,
        offset: Offset
    ) -> Result<OctamedMMD> {
        stream.seek(offset.into())?;
        let header = Self::parse_header(stream)?;
        self.mode = header.id.into();
        if
            header.song_ptr.is_null() ||
            header.block_array_ptr.is_null() ||
            header.sample_array_ptr.is_null() ||
            header.expansion_data_ptr.is_null()
        {
            panic!("Null pointer in file");
        }
        let song = self.parse_song_mmd0(header.song_ptr, stream)?;

        let block_table = self.parse_blocks(header.block_array_ptr, stream, &song)?;
        let sample_table = self.parse_sample_table(header.sample_array_ptr, &song, stream)?;
        let expansion_data = self.parse_expansion_data(header.expansion_data_ptr, stream)?;
        return Ok(OctamedMMD { song, block_table, header, sample_table, expansion_data });
    }

    pub fn parse_file(&mut self, path: &Path) -> Result<Vec<OctamedMMD>> {
        let mut file = File::open(path)?;
        let mut modules = vec![];
        let mut module = self.parse_module(&mut file, Offset(0))?;
        loop {
            let extra_songs = module.header.extra_songs.0;
            modules.push(module);

            if extra_songs > 0 {
                //todo parse others
                // module = Self:: etc.
                module = self.parse_module(&mut file, Offset(1))?;
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
    pub fn parse_header<R: Read + Seek>(stream: &mut R) -> Result<OctamedMMD0Header> {
        let id = Self::parse_ulong(stream)?;
        let length = Self::parse_ulong(stream)?;

        let song_ptr: Offset = Self::parse_offset(stream)?;
        let player_seconds_num = Self::parse_uword(stream)?;
        let player_sequence = Self::parse_uword(stream)?;

        let block_array_ptr: Offset = Self::parse_offset(stream)?;
        let flags = super::module::OctamedMMD0HeaderFlags::from_byte(Self::parse_ubyte(stream)?);
        let reserved = Self::parse_exact(stream, 3)?;
        let mut reserved_buffer = [UByte(0); 3];
        for (i, byte) in reserved.iter().enumerate() {
            reserved_buffer[i] = UByte(*byte);
        }
        let reserved = reserved_buffer;
        let sample_array_ptr = Self::parse_offset(stream)?;
        if sample_array_ptr.is_null() {
            panic!("MMD1+NoInstr not supported at this time");
        }
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
            active_play_line: actual_play_line,
            counter,
            extra_songs,
        };
        return Ok(header);
    }
    fn parse_song_mmd0<R: Read + Seek>(
        &self,
        song_offset: Offset,
        stream: &mut R
    ) -> Result<OctamedMMD0Song> {
        stream.seek(song_offset.into())?;
        let mut samples = [OctamedMMD0Sample::new(); 63];
        self.parse_samples(&mut samples, stream)?;
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
        &self,
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
        &self,
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
            let _is_stereo = ((sample_type.0 as i16) & OctamedMMD0SampleHeader::STEREO_SAMPLE) != 0;

            let is_sample = sample_type.0 >= 0;
            // sample_length is the total byte count of the sample data as stored on disk;
            // do NOT double for stereo — the field already encodes the full length
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
            sample_table.headers.push(Some(OctamedMMD0SampleHeader { sample_length, sample_type }));
            if is_sample {
                sample_table.samples.push(Some(samples));
            } else {
                // synth/hybrid instrument: keep headers and samples vecs in sync
                sample_table.samples.push(None);
            }
        }
        return Ok(sample_table);
    }
    fn parse_blocks<R: Read + Seek>(
        &self,
        blocks_offset: Offset,
        stream: &mut R,
        song: &OctamedMMD0Song
    ) -> Result<OctamedMMDBlockTable> {
        stream.seek(blocks_offset.into())?;
        let mut block_pointers = vec![];
        for _ in 0..song.block_count.0 {
            let offset = Self::parse_offset(stream)?;
            block_pointers.push(offset);
        }
        match self.mode {
            OctamedModuleKind::MMD0 => {
                let mut blocks = vec![];
                let mut headers = vec![];
                for ptr in block_pointers {
                    stream.seek(ptr.into())?;
                    let track_count = Self::parse_ubyte(stream)?;
                    let line_count = Self::parse_ubyte(stream)?;
                    let header = OctamedMMD0BlockHeader { track_count, line_count };
                    let mut block = OctamedMMD0Block { lines: vec![] };
                    // line_count is zero-indexed: 0 = 1 line. Cast to u32 before +1 to avoid u8 overflow.
                    for _i in 0..(line_count.0 as u32) + 1 {
                        let mut block_line = OctamedMMD0BlockLine { tracks: vec![] };
                        for _track in 0..track_count.0 {
                            let byte1 = Self::parse_ubyte(stream)?;
                            let byte2 = Self::parse_ubyte(stream)?;
                            let byte3 = Self::parse_ubyte(stream)?;
                            let track_line = OctamedMMDTrackLine::from_bytes_mmd0(
                                byte1,
                                byte2,
                                byte3
                            );
                            block_line.tracks.push(track_line);
                        }
                        block.lines.push(block_line);
                    }
                    headers.push(header);
                    blocks.push(block);
                    //todo
                }
                return Ok(OctamedMMDBlockTable::MMD0BlockTable { headers, blocks });
            }
            OctamedModuleKind::MMD1 => {
                let mut blocks = vec![];
                let mut headers = vec![];
                for ptr in block_pointers {
                    stream.seek(ptr.into())?;
                    let track_count = Self::parse_uword(stream)?;
                    let line_count = Self::parse_uword(stream)?;
                    let info_ptr = Self::parse_offset(stream)?;
                    let header = OctamedMMD1BlockHeader { track_count, line_count, info_ptr };
                    let info = self.parse_block_info(stream, &header)?;
                    let mut block = OctamedMMD1Block { lines: vec![], info };
                    // line_count is zero-indexed: 0 = 1 line. Cast to u32 before +1 to avoid u16 overflow.
                    for _i in 0..(line_count.0 as u32) + 1 {
                        let mut block_line = OctamedMMD0BlockLine { tracks: vec![] };
                        for _track in 0..track_count.0 {
                            let byte1 = Self::parse_ubyte(stream)?;
                            let byte2 = Self::parse_ubyte(stream)?;
                            let byte3 = Self::parse_ubyte(stream)?;
                            let byte4 = Self::parse_ubyte(stream)?;
                            let track_line = OctamedMMDTrackLine::from_bytes_mmd1(
                                byte1,
                                byte2,
                                byte3,
                                byte4
                            );
                            block_line.tracks.push(track_line);
                        }
                        block.lines.push(block_line);
                    }
                    headers.push(header);
                    blocks.push(block);
                    //todo
                }
                return Ok(OctamedMMDBlockTable::MMD1BlockTable { headers, blocks });
            }
            _ => panic!("No support for mmd2+ yet"),
        }
    }
    fn parse_block_info<R: Read + Seek>(
        &self,
        stream: &mut R,
        header: &OctamedMMD1BlockHeader
    ) -> Result<Option<OctamedMMD1BlockInfo>> {
        let start_pos = stream.stream_position()?;
        let offset = header.info_ptr;
        if offset.is_null() {
            return Ok(None);
        }
        stream.seek(offset.into())?;
        let highlight_mask_array_ptr = Self::parse_offset(stream)?;
        let block_name_string_ptr = Self::parse_offset(stream)?;
        let block_name_length = Self::parse_ulong(stream)?;
        let page_table_ptr = Self::parse_offset(stream)?;
        let mut reserved = [ULong(0); 5];
        for i in 0..reserved.len() {
            reserved[i] = Self::parse_ulong(stream)?;
        }
        let header = OctamedMMD1BlockInfoHeader {
            block_name_string_ptr,
            block_name_length,
            highlight_mask_array_ptr,
            page_table_ptr,
            reserved,
        };
        // block_name is at the pointed address, not sequential after the header struct
        stream.seek(header.block_name_string_ptr.into())?;
        let mut block_name = String::with_capacity(
            header.block_name_length.0.saturating_sub(1) as usize
        );
        for _ in 0..header.block_name_length.0.saturating_sub(1) {
            block_name.push(Self::parse_ubyte(stream)?.as_char());
        }
        let highlight_mask = {
            stream.seek(header.highlight_mask_array_ptr.into())?;
            //todo. not that important
            OctamedMMD1HighlightMask {}
        };
        let page_table: OctamedMMD2BlockCommandPageTable = {
            // only for mmd2
            OctamedMMD2BlockCommandPageTable {}
        };

        let info = OctamedMMD1BlockInfo { header, block_name, highlight_mask, page_table };
        stream.seek(std::io::SeekFrom::Start(start_pos))?;
        return Ok(Some(info));
    }
    fn parse_expansion_data<R: Read + Seek>(
        &self,
        expansion_offset: Offset,
        stream: &mut R
    ) -> Result<Option<OctamedMMD0Expansion>> {
        if expansion_offset.is_null() {
            return Ok(None);
        }
        stream.seek(expansion_offset.into())?;
        let header = self.parse_expansion_data_header(stream)?;

        let annotation = {
            if
                header.annotation_text_char_array_ptr.is_null() ||
                header.annotation_text_length.0 <= 1
            {
                // null/empty annotation: use empty string so writer allocates exactly 1 byte (null terminator)
                String::new()
            } else {
                stream.seek(header.annotation_text_char_array_ptr.into())?;
                let mut text = String::with_capacity(
                    (header.annotation_text_length.0 - 1) as usize
                );
                for _ in 0..header.annotation_text_length.0 - 1 {
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
        let notation_info = OctamedMMD0NotationInfo::new();
        let song_name = {
            if header.song_name_char_array_ptr.is_null() || header.song_name_length.0 <= 1 {
                // null/empty song name: use empty string so writer allocates exactly 1 byte (null terminator)
                String::new()
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
        &self,
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
