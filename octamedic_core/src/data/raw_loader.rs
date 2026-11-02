use crate::data::{
    instrument_loader::{InstrumentLoadError, InstrumentLoader},
    sample_import::DecodedSample,
};

// raw files don't say. C-2 on PAL, and it's only ever shown
const NOMINAL_RATE: u32 = 8287;

/// Any file as signed 8-bit samples. Like OctaMED, whatever no other format recognizes loads
/// this way, so it goes last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawLoader;

impl InstrumentLoader for RawLoader {
    fn recognizes(&self, _bytes: &[u8]) -> bool {
        return true;
    }

    fn decode(&self, bytes: &[u8]) -> Result<DecodedSample, InstrumentLoadError> {
        let samples = bytes
            .iter()
            .map(|&byte| byte as i8 as f32 / 128.0)
            .collect();
        return Ok(DecodedSample {
            rate: NOMINAL_RATE,
            bits: Some(8),
            channels: vec![samples],
            repeat: None,
        });
    }

    fn asks_for_settings(&self) -> bool {
        return false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::sample_import::ImportSettings;

    #[test]
    fn recognizes_anything() {
        assert!(RawLoader.recognizes(b"whatever this is"));
        assert!(RawLoader.recognizes(&[]));
    }

    #[test]
    fn bytes_come_out_as_they_went_in() {
        let bytes = [0x00, 0x7F, 0x80, 0xFF, 0x40];
        let decoded = RawLoader.decode(&bytes).unwrap();
        let instrument = decoded
            .to_instrument("raw", ImportSettings::default())
            .unwrap();
        assert_eq!(instrument.sample.unwrap().data, [0, 127, -128, -1, 64]);
    }

    #[test]
    fn empty_file_has_no_sound() {
        let decoded = RawLoader.decode(&[]).unwrap();
        assert!(
            decoded
                .to_instrument("raw", ImportSettings::default())
                .is_err()
        );
    }
}
