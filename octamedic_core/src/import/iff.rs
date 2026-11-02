use std::sync::Arc;

use crate::{
    data::instrument::{
        InstrumentKind, MAX_INSTRUMENT_VOLUME, OctamedicInstrument, OctamedicSample, SampleOctaves,
    },
    import::{Decoded, InstrumentLoadError, InstrumentLoader},
};

const FORM: &[u8] = b"FORM";
const SAMPLE_FORM: &[u8] = b"8SVX";
const HEADER: &[u8] = b"VHDR";
const BODY: &[u8] = b"BODY";
const HEADER_LENGTH: usize = 20;
// the volume is fixed point, this much is full
const FULL_VOLUME: u32 = 0x10000;
const UNCOMPRESSED: u8 = 0;
const FIBONACCI_DELTA: u8 = 1;
const DELTAS: [i8; 16] = [-34, -21, -13, -8, -5, -3, -2, -1, 0, 1, 2, 3, 5, 8, 13, 21];

/// An IFF 8SVX sample, the Amiga's own format: signed 8-bit, possibly with a copy per octave and
/// a loop, see docs/FILES.md. Loads as it is, there's nothing to ask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IffLoader;

impl InstrumentLoader for IffLoader {
    fn recognizes(&self, bytes: &[u8]) -> bool {
        return bytes.len() >= 12 && &bytes[0..4] == FORM && &bytes[8..12] == SAMPLE_FORM;
    }

    fn decode(&self, bytes: &[u8]) -> Result<Decoded, InstrumentLoadError> {
        let damaged = || InstrumentLoadError::Malformed("IFF sample is damaged".to_owned());
        let mut header = None;
        let mut body = None;
        for (id, data) in chunks(&bytes[12..]) {
            match id {
                HEADER => header = Some(Header::read(data).ok_or_else(damaged)?),
                BODY => body = Some(data),
                _ => {}
            }
        }
        let (header, body) = header.zip(body).ok_or_else(damaged)?;
        let data = match header.compression {
            UNCOMPRESSED => body.iter().map(|&byte| byte as i8).collect(),
            FIBONACCI_DELTA => unpack_fibonacci_delta(body),
            _ => {
                return Err(InstrumentLoadError::Malformed(
                    "Unknown IFF packing".to_owned(),
                ));
            }
        };
        return Ok(Decoded::Ready(Box::new(header.instrument(data))));
    }

    fn asks_for_settings(&self) -> bool {
        return false;
    }
}

struct Header {
    // of the shortest copy
    one_shot: usize,
    repeat: usize,
    octaves: u8,
    compression: u8,
    volume: u32,
}

impl Header {
    fn read(data: &[u8]) -> Option<Self> {
        let data = data.get(..HEADER_LENGTH)?;
        let long = |at: usize| u32::from_be_bytes(data[at..at + 4].try_into().unwrap());
        return Some(Self {
            one_shot: long(0) as usize,
            repeat: long(4) as usize,
            octaves: data[14],
            compression: data[15],
            volume: long(16),
        });
    }

    fn instrument(&self, mut data: Vec<i8>) -> OctamedicInstrument {
        let octaves = match self.octaves {
            2 => SampleOctaves::Two,
            3 => SampleOctaves::Three,
            4 => SampleOctaves::Four,
            5 => SampleOctaves::Five,
            6 => SampleOctaves::Six,
            7 => SampleOctaves::Seven,
            _ => SampleOctaves::One,
        };
        // every octave's copy, each twice as long as the one before
        let total = (1usize << octaves.copies()) - 1;
        let shortest = self.one_shot + self.repeat;
        if shortest > 0 {
            data.truncate(shortest * total);
        }
        let end = (self.one_shot + self.repeat).min(data.len());
        let repeat = (self.repeat > 0 && self.one_shot < end).then_some((self.one_shot, end));
        let mut instrument = OctamedicInstrument::new("");
        instrument.volume = self.volume_out_of_64();
        instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data,
            repeat,
            octaves,
        }));
        return instrument;
    }

    fn volume_out_of_64(&self) -> u8 {
        let volume = self.volume as u64 * MAX_INSTRUMENT_VOLUME as u64 / FULL_VOLUME as u64;
        return volume.min(MAX_INSTRUMENT_VOLUME as u64) as u8;
    }
}

// each an id, a big-endian length and the data, padded to an even length
fn chunks(mut bytes: &[u8]) -> impl Iterator<Item = (&[u8], &[u8])> {
    return std::iter::from_fn(move || {
        let id = bytes.get(0..4)?;
        let length = u32::from_be_bytes(bytes.get(4..8)?.try_into().unwrap()) as usize;
        let data = &bytes[8..];
        let data = &data[..length.min(data.len())];
        let padded = 8 + length + length % 2;
        bytes = bytes.get(padded..).unwrap_or(&[]);
        return Some((id, data));
    });
}

// a pad byte, the first sample, then a 4-bit index into DELTAS per sample, high half first
fn unpack_fibonacci_delta(body: &[u8]) -> Vec<i8> {
    let Some(&start) = body.get(1) else {
        return Vec::new();
    };
    let mut sample = start as i8;
    let mut data = Vec::with_capacity(body.len().saturating_sub(2) * 2);
    for &byte in body.iter().skip(2) {
        for index in [byte >> 4, byte & 0x0F] {
            sample = sample.wrapping_add(DELTAS[index as usize]);
            data.push(sample);
        }
    }
    return data;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::sample::ImportSettings;

    fn chunk(id: &[u8], data: &[u8]) -> Vec<u8> {
        let mut bytes = id.to_vec();
        bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(data);
        if data.len() % 2 == 1 {
            bytes.push(0);
        }
        return bytes;
    }

    fn header(one_shot: u32, repeat: u32, octaves: u8, compression: u8, volume: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&one_shot.to_be_bytes());
        data.extend_from_slice(&repeat.to_be_bytes());
        data.extend_from_slice(&32u32.to_be_bytes());
        data.extend_from_slice(&8363u16.to_be_bytes());
        data.extend_from_slice(&[octaves, compression]);
        data.extend_from_slice(&volume.to_be_bytes());
        return chunk(HEADER, &data);
    }

    fn file(chunks: &[Vec<u8>]) -> Vec<u8> {
        let body: Vec<u8> = chunks.concat();
        let mut bytes = FORM.to_vec();
        bytes.extend_from_slice(&(4 + body.len() as u32).to_be_bytes());
        bytes.extend_from_slice(SAMPLE_FORM);
        bytes.extend(body);
        return bytes;
    }

    fn instrument(bytes: &[u8]) -> OctamedicInstrument {
        return IffLoader
            .decode(bytes)
            .unwrap()
            .into_instrument("snare", ImportSettings::default())
            .unwrap();
    }

    fn sample(instrument: &OctamedicInstrument) -> &OctamedicSample {
        return instrument.sample().unwrap();
    }

    #[test]
    fn recognized_by_its_header() {
        assert!(IffLoader.recognizes(&file(&[])));
        assert!(!IffLoader.recognizes(b"FORM\0\0\0\x04AIFF"));
        assert!(!IffLoader.recognizes(b"RIFF\0\0\0\x04WAVE"));
    }

    #[test]
    fn loop_and_volume() {
        let bytes = file(&[
            chunk(b"NAME", b"Snare"),
            header(4, 2, 1, UNCOMPRESSED, FULL_VOLUME / 2),
            chunk(BODY, &[1, 2, 3, 4, 0xFF, 0x80]),
        ]);
        let instrument = instrument(&bytes);
        assert_eq!(instrument.name, "snare");
        assert_eq!(instrument.volume, 32);
        assert_eq!(sample(&instrument).data, [1, 2, 3, 4, -1, -128]);
        assert_eq!(sample(&instrument).repeat, Some((4, 6)));
    }

    #[test]
    fn a_copy_per_octave() {
        // 3 octaves of a 2 byte sound: 2, 4 and 8 bytes, shortest first
        let body: Vec<u8> = (0..14).collect();
        let bytes = file(&[
            header(2, 0, 3, UNCOMPRESSED, FULL_VOLUME),
            chunk(BODY, &body),
        ]);
        let instrument = instrument(&bytes);
        assert_eq!(sample(&instrument).octaves, SampleOctaves::Three);
        assert_eq!(sample(&instrument).data.len(), 14);
        assert_eq!(sample(&instrument).repeat, None);
    }

    #[test]
    fn too_many_octaves_is_one() {
        let bytes = file(&[
            header(2, 0, 9, UNCOMPRESSED, FULL_VOLUME),
            chunk(BODY, &[1; 6]),
        ]);
        let instrument = instrument(&bytes);
        assert_eq!(sample(&instrument).octaves, SampleOctaves::One);
        assert_eq!(sample(&instrument).data.len(), 2);
    }

    #[test]
    fn fibonacci_delta() {
        // starts at 10, then +1, -1, +21, -34
        let packed = [0, 10, 0x97, 0xF0];
        let bytes = file(&[
            header(0, 0, 1, FIBONACCI_DELTA, FULL_VOLUME),
            chunk(BODY, &packed),
        ]);
        assert_eq!(sample(&instrument(&bytes)).data, [11, 10, 31, -3]);
    }

    #[test]
    fn damaged() {
        let no_body = file(&[header(2, 0, 1, UNCOMPRESSED, FULL_VOLUME)]);
        let short_header = file(&[chunk(HEADER, &[0; 4]), chunk(BODY, &[0])]);
        for bytes in [no_body, short_header] {
            let result = IffLoader.decode(&bytes);
            assert!(matches!(result, Err(InstrumentLoadError::Malformed(_))));
        }
    }
}
