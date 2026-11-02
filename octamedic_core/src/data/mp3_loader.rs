use crate::data::{
    audio_decode,
    instrument_loader::{InstrumentLoadError, InstrumentLoader},
    sample_import::DecodedSample,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mp3Loader;

impl InstrumentLoader for Mp3Loader {
    fn recognizes(&self, bytes: &[u8]) -> bool {
        return bytes.starts_with(b"ID3") || is_frame_header(bytes);
    }

    fn decode(&self, bytes: &[u8]) -> Result<DecodedSample, InstrumentLoadError> {
        return audio_decode::decode(bytes, "mp3", "MP3");
    }
}

// strict, since anything unrecognized loads as raw and raw data can start with the sync bits
fn is_frame_header(bytes: &[u8]) -> bool {
    let [sync, flags, rates, ..] = *bytes else {
        return false;
    };
    let synced = sync == 0xFF && flags & 0xE0 == 0xE0;
    let version_known = (flags >> 3) & 0b11 != 0b01;
    let layer_3 = (flags >> 1) & 0b11 == 0b01;
    let bitrate_known = rates >> 4 != 0b1111;
    let rate_known = (rates >> 2) & 0b11 != 0b11;
    return synced && version_known && layer_3 && bitrate_known && rate_known;
}

#[cfg(test)]
mod tests {
    use super::*;

    const SINE: &[u8] = include_bytes!("../../test_data/sine.mp3");

    #[test]
    fn recognized_by_its_header() {
        assert!(Mp3Loader.recognizes(SINE));
        assert!(Mp3Loader.recognizes(b"ID3\x04\0\0\0\0\0\0"));
        // sync bits but layer 1, then a reserved sample rate
        assert!(!Mp3Loader.recognizes(&[0xFF, 0xFF, 0x90]));
        assert!(!Mp3Loader.recognizes(&[0xFF, 0xFB, 0x9C]));
        assert!(!Mp3Loader.recognizes(b"RIFF\0\0\0\0WAVE"));
    }

    // 0.1s of a 440 Hz sine, 22050 Hz mono. MP3 pads the start and end, so the length isn't exact
    #[test]
    fn decodes() {
        let decoded = Mp3Loader.decode(SINE).unwrap();
        assert_eq!(decoded.rate, 22_050);
        assert_eq!(decoded.bits, None);
        assert_eq!(decoded.channels.len(), 1);
        let frames = decoded.channels[0].len();
        assert!((2000..4000).contains(&frames), "{frames}");
        let peak = decoded.channels[0]
            .iter()
            .fold(0.0f32, |peak, s| peak.max(s.abs()));
        assert!(peak > 0.05, "{peak}");
    }
}
