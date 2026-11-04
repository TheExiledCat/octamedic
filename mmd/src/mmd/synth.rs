use std::io::{Cursor, Read, Seek, SeekFrom};

use crate::{
    mmd::error::{MmdError, MmdResult},
    utility::bytes::{Byte, UByte, UWord},
};

const MODULE_TABLE_LENGTH: usize = 128;
const MODULE_WAVEFORMS: usize = 64;
const MSH_ID: &[u8; 4] = b"MSH\0";
const SYNTH: i16 = -1;
const HYBRID: i16 = -2;

/// A synth or hybrid instrument. Tables are only as long as their length says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OctamedMMD0SynthInstrument {
    pub default_decay: UByte,
    /// Hybrids only.
    pub repeat: UWord,
    pub repeat_length: UWord,
    pub volume_speed: UByte,
    pub waveform_speed: UByte,
    pub volume_table: Vec<UByte>,
    pub waveform_table: Vec<UByte>,
    /// A hybrid's first waveform is its sample instead, so it's empty here.
    pub waveforms: Vec<Vec<Byte>>,
    pub hybrid_sample: Option<Vec<Byte>>,
}

impl OctamedMMD0SynthInstrument {
    pub fn is_hybrid(&self) -> bool {
        return self.hybrid_sample.is_some();
    }

    /// One saved on its own: "MSH\0", then the fields with the tables and waveform offsets cut to
    /// their lengths.
    pub fn from_msh(bytes: &[u8]) -> MmdResult<Self> {
        let mut stream = Cursor::new(bytes);
        let mut id = [0; 4];
        stream.read_exact(&mut id)?;
        if &id != MSH_ID {
            return Err(MmdError::UnknownFormat(u32::from_be_bytes(id)));
        }
        return Self::read(&mut stream, 0, false);
    }

    /// One in a module, starting at `start`: the full 128 byte tables and 64 waveform offsets.
    pub fn from_module<R: Read + Seek>(stream: &mut R, start: u64) -> MmdResult<Self> {
        stream.seek(SeekFrom::Start(start + 4))?;
        return Self::read(stream, start, true);
    }

    // waveform offsets count from `start`
    fn read<R: Read + Seek>(stream: &mut R, start: u64, full_tables: bool) -> MmdResult<Self> {
        let hybrid = read_type(stream)?;
        let default_decay = UByte(read_u8(stream)?);
        read_bytes(stream, 3)?;
        let repeat = UWord(read_u16(stream)?);
        let repeat_length = UWord(read_u16(stream)?);
        let volume_length = read_u16(stream)? as usize;
        let waveform_length = read_u16(stream)? as usize;
        let volume_speed = UByte(read_u8(stream)?);
        let waveform_speed = UByte(read_u8(stream)?);
        let count = read_u16(stream)? as usize;

        let (volume_table, waveform_table, offsets) = if full_tables {
            let mut volume_table = read_bytes(stream, MODULE_TABLE_LENGTH)?;
            let mut waveform_table = read_bytes(stream, MODULE_TABLE_LENGTH)?;
            let mut offsets = read_offsets(stream, MODULE_WAVEFORMS)?;
            volume_table.truncate(volume_length);
            waveform_table.truncate(waveform_length);
            offsets.truncate(count);
            (volume_table, waveform_table, offsets)
        } else {
            (
                read_bytes(stream, volume_length)?,
                read_bytes(stream, waveform_length)?,
                read_offsets(stream, count)?,
            )
        };

        let mut waveforms = Vec::new();
        let mut hybrid_sample = None;
        for (index, offset) in offsets.into_iter().enumerate() {
            stream.seek(SeekFrom::Start(start + offset as u64))?;
            if hybrid && index == 0 {
                hybrid_sample = Some(read_hybrid_sample(stream)?);
                waveforms.push(Vec::new());
            } else {
                let words = read_u16(stream)? as usize;
                waveforms.push(signed(read_bytes(stream, words * 2)?));
            }
        }
        return Ok(Self {
            default_decay,
            repeat,
            repeat_length,
            volume_speed,
            waveform_speed,
            volume_table,
            waveform_table,
            waveforms,
            hybrid_sample,
        });
    }
}

fn read_type<R: Read>(stream: &mut R) -> MmdResult<bool> {
    let kind = read_u16(stream)? as i16;
    return match kind {
        SYNTH => Ok(false),
        HYBRID => Ok(true),
        _ => Err(MmdError::UnknownInstrumentType(kind)),
    };
}

// a normal sample header, then the data
fn read_hybrid_sample<R: Read>(stream: &mut R) -> MmdResult<Vec<Byte>> {
    let length = read_u32(stream)? as usize;
    read_u16(stream)?;
    return Ok(signed(read_bytes(stream, length)?));
}

fn read_offsets<R: Read>(stream: &mut R, count: usize) -> MmdResult<Vec<u32>> {
    return (0..count).map(|_| read_u32(stream)).collect();
}

fn read_bytes<R: Read>(stream: &mut R, count: usize) -> MmdResult<Vec<UByte>> {
    let mut bytes = vec![0; count];
    stream.read_exact(&mut bytes)?;
    return Ok(bytes.into_iter().map(UByte).collect());
}

fn signed(bytes: Vec<UByte>) -> Vec<Byte> {
    return bytes.into_iter().map(|byte| Byte(byte.0 as i8)).collect();
}

fn read_u8<R: Read>(stream: &mut R) -> MmdResult<u8> {
    let mut bytes = [0; 1];
    stream.read_exact(&mut bytes)?;
    return Ok(bytes[0]);
}

fn read_u16<R: Read>(stream: &mut R) -> MmdResult<u16> {
    let mut bytes = [0; 2];
    stream.read_exact(&mut bytes)?;
    return Ok(u16::from_be_bytes(bytes));
}

fn read_u32<R: Read>(stream: &mut R) -> MmdResult<u32> {
    let mut bytes = [0; 4];
    stream.read_exact(&mut bytes)?;
    return Ok(u32::from_be_bytes(bytes));
}

#[cfg(test)]
mod tests {
    use super::*;

    // an organ, one 128 byte waveform
    const ORGAN_WAVE: [u8; 64] = [
        0x0f, 0x9b, 0x24, 0xac, 0x31, 0xb5, 0x36, 0xb6, 0x33, 0xb2, 0x31, 0xb3, 0x36, 0xbc, 0x43,
        0xcd, 0x17, 0xa3, 0x2c, 0xb4, 0x39, 0xbd, 0x3e, 0xbe, 0x3b, 0xba, 0x39, 0xbb, 0x3e, 0xc4,
        0x4b, 0xd5, 0x1f, 0xab, 0x34, 0xbc, 0x41, 0xc5, 0x46, 0xc6, 0x43, 0xc2, 0x41, 0xc3, 0x46,
        0xcc, 0x53, 0xdd, 0x27, 0xb3, 0x3c, 0xc4, 0x49, 0xcd, 0x4e, 0xce, 0x4b, 0xca, 0x49, 0xcb,
        0x4e, 0xd4, 0x5b, 0xe5,
    ];

    fn organ_msh() -> Vec<u8> {
        let mut bytes = b"MSH\0".to_vec();
        bytes.extend_from_slice(&[0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 4, 0, 6, 1, 3, 0, 1]);
        bytes.extend_from_slice(&[0x40, 0xf2, 0x03, 0xff]);
        bytes.extend_from_slice(&[0xf4, 0x06, 0xf5, 0x50, 0x00, 0xff]);
        bytes.extend_from_slice(&0x24u32.to_be_bytes());
        bytes.extend_from_slice(&0x40u16.to_be_bytes());
        bytes.extend_from_slice(&ORGAN_WAVE);
        bytes.extend_from_slice(&ORGAN_WAVE);
        return bytes;
    }

    fn bytes(values: &[u8]) -> Vec<UByte> {
        return values.iter().copied().map(UByte).collect();
    }

    #[test]
    fn reads_an_msh_synth() {
        let synth = OctamedMMD0SynthInstrument::from_msh(&organ_msh()).unwrap();
        assert!(!synth.is_hybrid());
        assert_eq!(
            (synth.volume_speed, synth.waveform_speed),
            (UByte(1), UByte(3))
        );
        assert_eq!(synth.volume_table, bytes(&[0x40, 0xf2, 0x03, 0xff]));
        assert_eq!(
            synth.waveform_table,
            bytes(&[0xf4, 0x06, 0xf5, 0x50, 0x00, 0xff])
        );
        assert_eq!(synth.waveforms.len(), 1);
        assert_eq!(synth.waveforms[0].len(), 128);
        assert_eq!(synth.waveforms[0][1], Byte(0x9b_u8 as i8));
    }

    #[test]
    fn msh_needs_its_id() {
        let mut bytes = organ_msh();
        bytes[0] = b'X';
        assert!(matches!(
            OctamedMMD0SynthInstrument::from_msh(&bytes),
            Err(MmdError::UnknownFormat(_))
        ));
    }

    // a module's layout, `start` bytes into the stream: full tables and 64 offsets, then a
    // hybrid's sample and one waveform
    fn module_hybrid(start: usize) -> Vec<u8> {
        let header_size = 22 + 2 * MODULE_TABLE_LENGTH + 4 * MODULE_WAVEFORMS;
        let sample_at = header_size as u32;
        let waveform_at = sample_at + 6 + 4;
        let mut synth = Vec::new();
        synth.extend_from_slice(&0u32.to_be_bytes());
        synth.extend_from_slice(&(HYBRID as u16).to_be_bytes());
        synth.extend_from_slice(&[5, 0, 0, 0, 0, 1, 0, 2]);
        synth.extend_from_slice(&[0, 2, 0, 1, 4, 5, 0, 2]);
        let mut volume_table = vec![0xaa; MODULE_TABLE_LENGTH];
        volume_table[..2].copy_from_slice(&[0x20, 0xff]);
        synth.extend_from_slice(&volume_table);
        let mut waveform_table = vec![0xaa; MODULE_TABLE_LENGTH];
        waveform_table[0] = 0xff;
        synth.extend_from_slice(&waveform_table);
        let mut offsets = vec![0u32; MODULE_WAVEFORMS];
        offsets[0] = sample_at;
        offsets[1] = waveform_at;
        offsets
            .iter()
            .for_each(|offset| synth.extend_from_slice(&offset.to_be_bytes()));
        synth.extend_from_slice(&4u32.to_be_bytes());
        synth.extend_from_slice(&0u16.to_be_bytes());
        synth.extend_from_slice(&[1, 2, 3, 4]);
        synth.extend_from_slice(&1u16.to_be_bytes());
        synth.extend_from_slice(&[0x7f, 0x80]);
        return [vec![0xee; start], synth].concat();
    }

    #[test]
    fn reads_a_module_hybrid() {
        let mut stream = Cursor::new(module_hybrid(10));
        let synth = OctamedMMD0SynthInstrument::from_module(&mut stream, 10).unwrap();
        assert!(synth.is_hybrid());
        assert_eq!(synth.default_decay, UByte(5));
        assert_eq!((synth.repeat, synth.repeat_length), (UWord(1), UWord(2)));
        assert_eq!(synth.volume_table, bytes(&[0x20, 0xff]));
        assert_eq!(synth.waveform_table, bytes(&[0xff]));
        assert_eq!(
            synth.hybrid_sample,
            Some(vec![Byte(1), Byte(2), Byte(3), Byte(4)])
        );
        assert_eq!(synth.waveforms, [vec![], vec![Byte(127), Byte(-128)]]);
    }
}
