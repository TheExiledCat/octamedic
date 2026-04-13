use std::{ fs::File, io::{ Read, Seek }, path::Path };

use crate::{
    mmd0::module::{
        OctamedMMD0,
        OctamedMMD0BlockTable,
        OctamedMMD0Header,
        OctamedMMD0Sample,
        OctamedMMD0SampleTable,
        OctamedMMD0Song,
        OctamedMMD0SongFlags,
    },
    utility::{ bytes::{ Byte, Offset, UByte, ULong, UWord, Word } },
};

type Result<T> = std::io::Result<T>;
pub struct OctamedMMD0Parser;

impl OctamedMMD0Parser {
    pub fn parse_module<R: Read + Seek>(stream: &mut R) -> Result<OctamedMMD0> {
        let header = Self::parse_header_mmd0(stream)?;

        let song = Self::parse_song_mmd0(header.song_ptr, stream)?;

        let block_table = Self::parse_blocks(header.block_array_ptr, stream)?;
        let sample_table = Self::parse_sample_table(header.sample_array_ptr, stream)?;

        return Ok(OctamedMMD0 { song, block_table, header, sample_table });
    }

    pub fn parse_file(path: &Path) -> Result<Vec<OctamedMMD0>> {
        let mut file = File::open(path)?;
        let mut modules = vec![];
        let module = Self::parse_module(&mut file)?;
        loop {
            modules.push(module);

            if &module.header.extra_songs.0 > 0 {
                //todo parse others
                // module = Self:: etc.
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
            default_song_tempo,
            global_transpose,
            flags,
            pulses_per_line,
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
        stream: &mut R
    ) -> Result<OctamedMMD0SampleTable> {
        return Ok(OctamedMMD0SampleTable {});
        todo!()
    }
    fn parse_blocks<R: Read + Seek>(
        blocks_offset: Offset,
        stream: &mut R
    ) -> Result<OctamedMMD0BlockTable> {
        return Ok(OctamedMMD0BlockTable {});
        todo!()
    }
}
