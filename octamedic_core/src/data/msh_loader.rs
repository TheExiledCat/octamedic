use std::sync::Arc;

use octamed::mmd::synth::OctamedMMD0SynthInstrument;

use crate::data::{
    instrument::{InstrumentKind, OctamedicSample, OctamedicSynth, SampleOctaves},
    instrument_loader::{Decoded, InstrumentLoadError, InstrumentLoader},
};

const ID: &[u8] = b"MSH\0";

/// A synth or hybrid sound saved on its own, see docs/FILES.md.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MshLoader;

impl InstrumentLoader for MshLoader {
    fn recognizes(&self, bytes: &[u8]) -> bool {
        return bytes.starts_with(ID);
    }

    fn decode(&self, bytes: &[u8]) -> Result<Decoded, InstrumentLoadError> {
        let msh = OctamedMMD0SynthInstrument::from_msh(bytes)
            .map_err(|_| InstrumentLoadError::Malformed("Synth sound is damaged".to_owned()))?;
        let synth = Arc::new(OctamedicSynth::from_octamed(&msh));
        let kind = match &msh.hybrid_sample {
            Some(bytes) => InstrumentKind::Hybrid(synth, Arc::new(hybrid_sample(&msh, bytes))),
            None => InstrumentKind::Synth(synth),
        };
        return Ok(Decoded::Ready(kind));
    }

    fn asks_for_settings(&self) -> bool {
        return false;
    }
}

// repeat and its length are in words
fn hybrid_sample(
    msh: &OctamedMMD0SynthInstrument,
    bytes: &[octamed::utility::bytes::Byte],
) -> OctamedicSample {
    let data: Vec<i8> = bytes.iter().map(|byte| byte.0).collect();
    let start = msh.repeat.0 as usize * 2;
    let end = (start + msh.repeat_length.0 as usize * 2).min(data.len());
    let repeat = (msh.repeat_length.0 > 1 && start < end).then_some((start, end));
    return OctamedicSample {
        data,
        repeat,
        octaves: SampleOctaves::One,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::sample_import::ImportSettings;

    // volume 64 then end, waveform 0 then end, one 4 byte waveform
    fn synth_msh() -> Vec<u8> {
        let mut bytes = b"MSH\0".to_vec();
        bytes.extend_from_slice(&[0xff, 0xff, 7, 0, 0, 0, 0, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 2, 0, 2, 1, 1, 0, 1]);
        bytes.extend_from_slice(&[0x40, 0xff, 0x00, 0xff]);
        bytes.extend_from_slice(&30u32.to_be_bytes());
        bytes.extend_from_slice(&2u16.to_be_bytes());
        bytes.extend_from_slice(&[0x10, 0x20, 0xf0, 0xe0]);
        return bytes;
    }

    #[test]
    fn recognized_by_its_id() {
        assert!(MshLoader.recognizes(&synth_msh()));
        assert!(!MshLoader.recognizes(b"MSH"));
        assert!(!MshLoader.recognizes(b"RIFF\0\0\0\0WAVE"));
    }

    #[test]
    fn loads_a_synth_ready_to_use() {
        let decoded = MshLoader.decode(&synth_msh()).unwrap();
        let instrument = decoded
            .into_instrument("organ", ImportSettings::default())
            .unwrap();
        assert_eq!(instrument.decay, 7);
        let InstrumentKind::Synth(synth) = instrument.kind else {
            panic!("not a synth");
        };
        assert_eq!(synth.volume_table, [0x40, 0xff]);
        assert_eq!(synth.waveform_table, [0x00, 0xff]);
        assert_eq!(synth.waveforms, [vec![0x10, 0x20, -0x10, -0x20]]);
    }

    #[test]
    fn damaged() {
        let mut bytes = synth_msh();
        bytes.truncate(20);
        let result = MshLoader.decode(&bytes);
        assert!(matches!(result, Err(InstrumentLoadError::Malformed(_))));
    }
}
