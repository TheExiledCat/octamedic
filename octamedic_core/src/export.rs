//! Samples written out as files, the other way round from `import`.

use crate::{
    data::instrument::{
        MAX_INSTRUMENT_VOLUME, OctamedicInstrument, OctamedicSample, SampleOctaves,
    },
    import::iff::{BODY, FORM, FULL_VOLUME, HEADER, SAMPLE_FORM},
};

/// The rate a sample plays at for C-2 on a PAL Amiga, which is what the raw loader assumes.
pub const NOMINAL_RATE: u32 = 8287;

pub fn raw(sample: &OctamedicSample) -> Vec<u8> {
    return sample.data.iter().map(|&value| value as u8).collect();
}

/// IFF 8SVX: a copy per octave, each cut short at its loop's end as the format has the loop
/// running to the end of the copy, see docs/FILES.md.
pub fn iff(name: &str, instrument: &OctamedicInstrument, sample: &OctamedicSample) -> Vec<u8> {
    let copies = sample.octaves.copies();
    let shortest = sample.octave_copy(0);
    // the shortest copy starts the data, so its loop is where VHDR wants it
    let (one_shot, repeat) = match &shortest.repeat {
        Some(repeat) => (repeat.start, repeat.len()),
        None => (shortest.data.len(), 0),
    };
    let mut body = Vec::new();
    for copy in 0..copies {
        let copy = sample.octave_copy(copy);
        let end = copy.repeat.map_or(copy.data.end, |repeat| repeat.end);
        body.extend(
            sample.data[copy.data.start..end]
                .iter()
                .map(|&value| value as u8),
        );
    }
    // ExtraLow has no IFF octave count of its own, it's saved as a plain sample
    let octaves = match sample.octaves {
        SampleOctaves::ExtraLow => 1,
        _ => copies as u8,
    };
    let volume = instrument.volume as u32 * FULL_VOLUME / MAX_INSTRUMENT_VOLUME as u32;
    let mut header = Vec::new();
    header.extend_from_slice(&(one_shot as u32).to_be_bytes());
    header.extend_from_slice(&(repeat as u32).to_be_bytes());
    // samples per cycle, only for synthesized waveforms
    header.extend_from_slice(&0u32.to_be_bytes());
    header.extend_from_slice(&(NOMINAL_RATE as u16).to_be_bytes());
    header.extend_from_slice(&[octaves, UNCOMPRESSED]);
    header.extend_from_slice(&volume.to_be_bytes());
    let chunks = [
        chunk(HEADER, &header),
        chunk(NAME, name.as_bytes()),
        chunk(BODY, &body),
    ]
    .concat();
    let mut bytes = FORM.to_vec();
    bytes.extend_from_slice(&(SAMPLE_FORM.len() as u32 + chunks.len() as u32).to_be_bytes());
    bytes.extend_from_slice(SAMPLE_FORM);
    bytes.extend_from_slice(&chunks);
    return bytes;
}

/// 8-bit WAV samples are unsigned. A loop goes in a `smpl` chunk, only for a single octave
/// sample as a multi-octave one's loop is in its shortest copy.
pub fn wav(sample: &OctamedicSample) -> Vec<u8> {
    let data: Vec<u8> = sample
        .data
        .iter()
        .map(|&value| (value as u8) ^ 0x80)
        .collect();
    let mut format = Vec::new();
    format.extend_from_slice(&PCM.to_le_bytes());
    // mono
    format.extend_from_slice(&1u16.to_le_bytes());
    format.extend_from_slice(&NOMINAL_RATE.to_le_bytes());
    // bytes a second, then a frame's bytes
    format.extend_from_slice(&NOMINAL_RATE.to_le_bytes());
    format.extend_from_slice(&1u16.to_le_bytes());
    format.extend_from_slice(&8u16.to_le_bytes());
    let mut chunks = [riff_chunk(b"fmt ", &format), riff_chunk(b"data", &data)].concat();
    if let (Some((start, end)), 1) = (sample.repeat, sample.octaves.copies()) {
        chunks.extend(riff_chunk(b"smpl", &sampler_loop(start, end)));
    }
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&(4 + chunks.len() as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(&chunks);
    return bytes;
}

// the sampler chunk's fields, all 32-bit, with one forward loop that plays forever
fn sampler_loop(start: usize, end: usize) -> Vec<u8> {
    let nanoseconds_a_sample = 1_000_000_000 / NOMINAL_RATE;
    let fields = [
        // manufacturer, product
        0,
        0,
        nanoseconds_a_sample,
        MIDI_C_2,
        // pitch fraction, SMPTE format and offset
        0,
        0,
        0,
        // loops, then bytes of sampler data after them
        1,
        0,
        // the loop: cue id, forward, start, end (its last sample), fraction, play count
        0,
        0,
        start as u32,
        end.saturating_sub(1) as u32,
        0,
        0,
    ];
    return fields
        .iter()
        .flat_map(|field: &u32| field.to_le_bytes())
        .collect();
}

// the MIDI note the sample plays at its own rate, which is OctaMED's C-2
const MIDI_C_2: u32 = 60;
const NAME: &[u8] = b"NAME";
const UNCOMPRESSED: u8 = 0;
const PCM: u16 = 1;

// an id, a big-endian length and the data, padded to an even length
fn chunk(id: &[u8], data: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
    bytes.extend_from_slice(data);
    if data.len() % 2 == 1 {
        bytes.push(0);
    }
    return bytes;
}

// the same, little-endian
fn riff_chunk(id: &[u8], data: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(data);
    if data.len() % 2 == 1 {
        bytes.push(0);
    }
    return bytes;
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::{
        data::instrument::InstrumentKind,
        import::{Decoded, InstrumentLoader, iff::IffLoader, raw::RawLoader, wav::WavLoader},
    };

    fn sample(
        data: Vec<i8>,
        repeat: Option<(usize, usize)>,
        octaves: SampleOctaves,
    ) -> OctamedicSample {
        return OctamedicSample {
            data,
            repeat,
            octaves,
        };
    }

    fn instrument(volume: u8) -> OctamedicInstrument {
        let mut instrument = OctamedicInstrument::new("test");
        instrument.volume = volume;
        return instrument;
    }

    fn loaded(decoded: Decoded) -> Arc<OctamedicSample> {
        let Decoded::Ready(instrument) = decoded else {
            panic!("an IFF loads ready");
        };
        let InstrumentKind::Sample(sample) = instrument.kind else {
            panic!("a sample");
        };
        return sample;
    }

    #[test]
    fn raw_is_just_the_bytes() {
        let bytes = raw(&sample(vec![0, 1, -1, -128], None, SampleOctaves::One));
        assert_eq!(bytes, [0, 1, 0xFF, 0x80]);
        let decoded = RawLoader.decode(&bytes).unwrap().into_sample().unwrap();
        assert_eq!(decoded.rate, NOMINAL_RATE);
    }

    #[test]
    fn iff_loads_back_the_same() {
        let looped = sample((0..40).collect(), Some((10, 30)), SampleOctaves::One);
        let bytes = iff("name", &instrument(32), &looped);
        assert!(IffLoader.recognizes(&bytes));
        let decoded = IffLoader.decode(&bytes).unwrap();
        let Decoded::Ready(loaded_instrument) = &decoded else {
            panic!("an IFF loads ready");
        };
        assert_eq!(loaded_instrument.volume, 32);
        // nothing after the loop's end ever plays, so it's left off
        let back = loaded(decoded);
        assert_eq!(back.data, (0..30).collect::<Vec<i8>>());
        assert_eq!(back.repeat, Some((10, 30)));
        let plain = sample(vec![5; 9], None, SampleOctaves::One);
        let back = loaded(IffLoader.decode(&iff("", &instrument(64), &plain)).unwrap());
        assert_eq!(*back, plain);
    }

    #[test]
    fn iff_keeps_every_octave_copy() {
        // 3 copies: 4, 8 and 16 bytes, looping the second half of each
        let data: Vec<i8> = (0..28).collect();
        let octaves = sample(data.clone(), Some((2, 4)), SampleOctaves::Three);
        let back = loaded(
            IffLoader
                .decode(&iff("", &instrument(64), &octaves))
                .unwrap(),
        );
        assert_eq!(*back, octaves);
        // a loop that ends early cuts every copy short at its own loop end
        let early = sample(data, Some((1, 2)), SampleOctaves::Three);
        let back = loaded(IffLoader.decode(&iff("", &instrument(64), &early)).unwrap());
        assert_eq!(back.octaves, SampleOctaves::Three);
        assert_eq!(back.repeat, Some((1, 2)));
        assert_eq!(
            back.data,
            [0, 1, 4, 5, 6, 7, 12, 13, 14, 15, 16, 17, 18, 19]
        );
    }

    #[test]
    fn wav_loads_back_the_same() {
        let data = vec![0, 64, -64, 127, -128];
        let bytes = wav(&sample(data.clone(), None, SampleOctaves::One));
        assert!(WavLoader.recognizes(&bytes));
        let decoded = WavLoader.decode(&bytes).unwrap().into_sample().unwrap();
        assert_eq!(decoded.rate, NOMINAL_RATE);
        let back: Vec<i8> = decoded.channels[0]
            .iter()
            .map(|&value| (value * 128.0).round() as i8)
            .collect();
        assert_eq!(back, data);
    }

    #[test]
    fn wav_loop_goes_in_a_sampler_chunk() {
        let looped = wav(&sample(vec![0; 40], Some((10, 30)), SampleOctaves::One));
        let at = looped.windows(4).position(|id| id == b"smpl").unwrap();
        let field = |index: usize| {
            let start = at + 8 + index * 4;
            u32::from_le_bytes(looped[start..start + 4].try_into().unwrap())
        };
        // one loop, from 10 up to and including 29
        assert_eq!((field(7), field(11), field(12)), (1, 10, 29));
        assert!(WavLoader.decode(&looped).is_ok());
        let plain = wav(&sample(vec![0; 40], None, SampleOctaves::One));
        assert!(!plain.windows(4).any(|id| id == b"smpl"));
        let octaves = wav(&sample(vec![0; 28], Some((2, 4)), SampleOctaves::Three));
        assert!(!octaves.windows(4).any(|id| id == b"smpl"));
    }
}
