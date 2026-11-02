use crate::data::{
    audio_decode,
    instrument_loader::{Decoded, InstrumentLoadError, InstrumentLoader},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WavLoader;

impl InstrumentLoader for WavLoader {
    // "RIFF", the chunk's size, then "WAVE"
    fn recognizes(&self, bytes: &[u8]) -> bool {
        return bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE";
    }

    fn decode(&self, bytes: &[u8]) -> Result<Decoded, InstrumentLoadError> {
        return audio_decode::decode(bytes, "wav", "WAV").map(Decoded::Sample);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PCM: u16 = 1;
    const FLOAT: u16 = 3;

    fn wav(format: u16, channels: u16, bits: u16, data: &[u8]) -> Vec<u8> {
        let rate: u32 = 44_100;
        let block = channels * bits / 8;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes());
        bytes.extend_from_slice(&format.to_le_bytes());
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&rate.to_le_bytes());
        bytes.extend_from_slice(&(rate * block as u32).to_le_bytes());
        bytes.extend_from_slice(&block.to_le_bytes());
        bytes.extend_from_slice(&bits.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
        bytes.extend_from_slice(data);
        return bytes;
    }

    fn samples_16_bit(samples: &[i16]) -> Vec<u8> {
        return samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect();
    }

    #[test]
    fn recognized_by_its_header() {
        assert!(WavLoader.recognizes(&wav(PCM, 1, 16, &samples_16_bit(&[0]))));
        assert!(!WavLoader.recognizes(b"RIFF\0\0\0\0AVI LIST"));
        assert!(!WavLoader.recognizes(b"FORM\0\0\0\08SVX"));
        assert!(!WavLoader.recognizes(b"RIFF"));
    }

    #[test]
    fn decodes_16_bit_stereo_into_channels() {
        let data = samples_16_bit(&[i16::MAX, 0, 0, i16::MIN]);
        let decoded = WavLoader
            .decode(&wav(PCM, 2, 16, &data))
            .unwrap()
            .into_sample()
            .unwrap();
        assert_eq!((decoded.rate, decoded.bits), (44_100, Some(16)));
        assert_eq!(decoded.channels.len(), 2);
        assert!((decoded.channels[0][0] - 1.0).abs() < 1e-4);
        assert_eq!(decoded.channels[1], [0.0, -1.0]);
    }

    #[test]
    fn decodes_float() {
        let data = 0.5f32.to_le_bytes();
        let decoded = WavLoader
            .decode(&wav(FLOAT, 1, 32, &data))
            .unwrap()
            .into_sample()
            .unwrap();
        assert_eq!(decoded.channels, [[0.5]]);
    }

    // 8-bit WAVs are unsigned, 128 is silence
    #[test]
    fn decodes_8_bit() {
        let decoded = WavLoader
            .decode(&wav(PCM, 1, 8, &[64, 128, 192]))
            .unwrap()
            .into_sample()
            .unwrap();
        assert_eq!(decoded.channels, [[-0.5, 0.0, 0.5]]);
    }

    #[test]
    fn damaged_wav() {
        let result = WavLoader.decode(b"RIFF\x04\0\0\0WAVE");
        assert!(matches!(result, Err(InstrumentLoadError::Malformed(_))));
    }
}
