use std::io::Cursor;

use hound::{SampleFormat, WavReader};

use crate::data::{
    instrument_loader::{InstrumentLoadError, InstrumentLoader},
    sample_import::DecodedSample,
};

/// Reads any PCM or float WAV, with any number of channels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavLoader;

impl InstrumentLoader for WavLoader {
    // "RIFF", the chunk's size, then "WAVE"
    fn recognizes(&self, bytes: &[u8]) -> bool {
        return bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE";
    }

    fn decode(&self, bytes: &[u8]) -> Result<DecodedSample, InstrumentLoadError> {
        let mut reader = WavReader::new(Cursor::new(bytes)).map_err(|_| damaged())?;
        let spec = reader.spec();
        let interleaved: Result<Vec<f32>, hound::Error> = match spec.sample_format {
            SampleFormat::Float => reader.samples::<f32>().collect(),
            SampleFormat::Int => {
                let full_scale = (1u64 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .samples::<i32>()
                    .map(|sample| sample.map(|sample| sample as f32 / full_scale))
                    .collect()
            }
        };
        let interleaved = interleaved.map_err(|_| damaged())?;
        return Ok(DecodedSample {
            rate: spec.sample_rate,
            bits: spec.bits_per_sample,
            channels: deinterleave(&interleaved, spec.channels as usize),
            repeat: None,
        });
    }
}

fn damaged() -> InstrumentLoadError {
    return InstrumentLoadError::Malformed("WAV file is damaged".to_owned());
}

fn deinterleave(samples: &[f32], channels: usize) -> Vec<Vec<f32>> {
    let channels = channels.max(1);
    return (0..channels)
        .map(|channel| {
            samples
                .chunks_exact(channels)
                .map(|frame| frame[channel])
                .collect()
        })
        .collect();
}

#[cfg(test)]
mod tests {
    use hound::{WavSpec, WavWriter};

    use super::*;

    fn wav(spec: WavSpec, write: impl FnOnce(&mut WavWriter<&mut Cursor<Vec<u8>>>)) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        let mut writer = WavWriter::new(&mut bytes, spec).unwrap();
        write(&mut writer);
        writer.finalize().unwrap();
        return bytes.into_inner();
    }

    fn spec(channels: u16, bits: u16, format: SampleFormat) -> WavSpec {
        return WavSpec {
            channels,
            sample_rate: 44_100,
            bits_per_sample: bits,
            sample_format: format,
        };
    }

    #[test]
    fn recognized_by_its_header() {
        let bytes = wav(spec(1, 16, SampleFormat::Int), |writer| {
            writer.write_sample(0i16).unwrap()
        });
        assert!(WavLoader.recognizes(&bytes));
        assert!(!WavLoader.recognizes(b"RIFF\0\0\0\0AVI LIST"));
        assert!(!WavLoader.recognizes(b"FORM\0\0\0\08SVX"));
        assert!(!WavLoader.recognizes(b"RIFF"));
    }

    #[test]
    fn decodes_16_bit_stereo_into_channels() {
        let bytes = wav(spec(2, 16, SampleFormat::Int), |writer| {
            for sample in [i16::MAX, 0, 0, i16::MIN] {
                writer.write_sample(sample).unwrap();
            }
        });
        let decoded = WavLoader.decode(&bytes).unwrap();
        assert_eq!((decoded.rate, decoded.bits), (44_100, 16));
        assert_eq!(decoded.channels.len(), 2);
        assert!((decoded.channels[0][0] - 1.0).abs() < 1e-4);
        assert_eq!(decoded.channels[1], [0.0, -1.0]);
    }

    #[test]
    fn decodes_float_and_8_bit() {
        let float = wav(spec(1, 32, SampleFormat::Float), |writer| {
            writer.write_sample(0.5f32).unwrap()
        });
        assert_eq!(WavLoader.decode(&float).unwrap().channels, [[0.5]]);
        let eight_bit = wav(spec(1, 8, SampleFormat::Int), |writer| {
            writer.write_sample(-64i8).unwrap()
        });
        assert_eq!(WavLoader.decode(&eight_bit).unwrap().channels, [[-0.5]]);
    }

    #[test]
    fn damaged_wav() {
        let result = WavLoader.decode(b"RIFF\x04\0\0\0WAVE");
        assert!(matches!(result, Err(InstrumentLoadError::Malformed(_))));
    }
}
