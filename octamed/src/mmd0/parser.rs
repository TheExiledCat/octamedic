use std::{ fs::File, io::{ Read, Seek }, path::Path };

use crate::utility::{ bytes::{ Offset, UByte, ULong, UWord, Word }, logger::Logger };

type Result<T> = std::io::Result<T>;
pub struct OctamedMMD0Parser;

impl OctamedMMD0Parser {
    pub fn parse_module<R: Read + Seek, L: Logger>(stream: &mut R, logger: L) -> Result<()> {
        let id = Self::parse_ulong(stream)?;
        let length = Self::parse_ulong(stream)?;

        let id_bytes = id.0.to_be_bytes();
        let id_text = str::from_utf8(&id_bytes).unwrap();
        logger.log(&format!("ID: {}", id_text));
        logger.log(&format!("File Length: {} Bytes", length.0));
        logger.log("Parsing File");
        let song_ptr: Offset = Self::parse_offset(stream)?;
        let player_seconds_num = Self::parse_uword(stream)?;
        let player_sequence = Self::parse_uword(stream)?;

        let block_array_ptr: Offset = Self::parse_offset(stream)?;
        let mmdflags = Self::parse_ubyte(stream)?;
        let reserved1 = Self::parse_exact(stream, 3)?;
        let sample_array_ptr = Self::parse_offset(stream)?;
        let reserved2 = Self::parse_ulong(stream)?;
        let expansion_data_ptr = Self::parse_offset(stream)?;
        let reserved3 = Self::parse_ulong(stream)?;

        let player_state = Self::parse_uword(stream)?;
        let player_block = Self::parse_uword(stream)?;
        let player_line = Self::parse_uword(stream)?;
        let player_sequence_number = Self::parse_uword(stream)?;

        let actual_play_line = Self::parse_word(stream)?;
        let counter = Self::parse_ubyte(stream)?;
        let extra_songs = Self::parse_ubyte(stream)?;

        logger.log(&format!("Block array offset: {}", block_array_ptr));
        logger.log(&format!("Flags: {:08b}", mmdflags.0));
        logger.log(&format!("Sample array offset: {}", sample_array_ptr));
        logger.log(&format!("Extra songs in module: {}", extra_songs.0));
        logger.log(
            &format!(
                "Advanced {} bytes into the stream (should be 52)",
                stream.stream_position().unwrap()
            )
        );
        return Ok(());
        todo!()
    }
    fn parse_file<L: Logger>(path: &Path, logger: L) -> Result<()> {
        let mut file = File::open(path)?;
        return Self::parse_module(&mut file, logger);
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
}
