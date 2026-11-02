use std::io::Cursor;

use symphonia::core::{
    codecs::audio::AudioDecoderOptions,
    errors::Error,
    formats::{FormatOptions, TrackType, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};

use crate::import::{InstrumentLoadError, sample::DecodedSample};

/// Decodes a whole FLAC, MP3 or WAV file. `format_name` is for the error message.
pub fn decode(
    bytes: &[u8],
    extension: &str,
    format_name: &str,
) -> Result<DecodedSample, InstrumentLoadError> {
    let damaged = || InstrumentLoadError::Malformed(format!("{format_name} file is damaged"));
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes.to_vec())), Default::default());
    let mut hint = Hint::new();
    hint.with_extension(extension);
    let options = (FormatOptions::default(), MetadataOptions::default());
    let mut format = symphonia::default::get_probe()
        .probe(&hint, source, options.0, options.1)
        .map_err(|_| damaged())?;
    let track = format.default_track(TrackType::Audio).ok_or_else(damaged)?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|params| params.audio())
        .ok_or_else(damaged)?
        .clone();
    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())
        .map_err(|_| damaged())?;
    let mut interleaved: Vec<f32> = Vec::new();
    let mut rate = params.sample_rate;
    let mut channels = params.channels.as_ref().map(|channels| channels.count());
    // broken frames are skipped, like players do
    while let Ok(Some(packet)) = format.next_packet() {
        if packet.track_id != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(buffer) => {
                rate = Some(buffer.spec().rate());
                channels = Some(buffer.spec().channels().count());
                let start = interleaved.len();
                interleaved.resize(start + buffer.samples_interleaved(), 0.0);
                buffer.copy_to_slice_interleaved(&mut interleaved[start..]);
            }
            Err(Error::DecodeError(_)) => continue,
            Err(_) => break,
        }
    }
    let (Some(rate), Some(channels)) = (rate, channels) else {
        return Err(damaged());
    };
    return Ok(DecodedSample {
        rate,
        bits: params.bits_per_sample.map(|bits| bits as u16),
        channels: deinterleave(&interleaved, channels),
        repeat: None,
    });
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
