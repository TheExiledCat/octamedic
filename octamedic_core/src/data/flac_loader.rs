use crate::data::{
    audio_decode,
    instrument_loader::{Decoded, InstrumentLoadError, InstrumentLoader},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FlacLoader;

impl InstrumentLoader for FlacLoader {
    fn recognizes(&self, bytes: &[u8]) -> bool {
        return bytes.starts_with(b"fLaC");
    }

    fn decode(&self, bytes: &[u8]) -> Result<Decoded, InstrumentLoadError> {
        return audio_decode::decode(bytes, "flac", "FLAC").map(Decoded::Sample);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINE: &[u8] = include_bytes!("../../test_data/sine.flac");

    #[test]
    fn recognized_by_its_header() {
        assert!(FlacLoader.recognizes(SINE));
        assert!(!FlacLoader.recognizes(b"RIFF\0\0\0\0WAVE"));
        assert!(!FlacLoader.recognizes(b"fLa"));
    }

    // 0.1s of a 440 Hz sine, 22050 Hz, 16 bit stereo
    #[test]
    fn decodes() {
        let decoded = FlacLoader.decode(SINE).unwrap().into_sample().unwrap();
        assert_eq!((decoded.rate, decoded.bits), (22_050, Some(16)));
        assert_eq!(decoded.channels.len(), 2);
        assert_eq!(decoded.channels[0].len(), 2205);
        let peak = decoded.channels[0]
            .iter()
            .fold(0.0f32, |peak, s| peak.max(s.abs()));
        assert!(peak > 0.05, "{peak}");
    }

    #[test]
    fn damaged() {
        let result = FlacLoader.decode(b"fLaC and then nothing useful");
        assert!(matches!(result, Err(InstrumentLoadError::Malformed(_))));
    }
}
