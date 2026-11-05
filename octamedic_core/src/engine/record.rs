use std::{
    fmt::Display,
    ops::Range,
    sync::{Arc, mpsc},
    time::Duration,
};

use super::{
    engine::{OctamedicEngine, PlayMode},
    handle::{EngineCommand, EngineStatus},
};
use crate::{
    amiga::clock::AmigaClock,
    data::{instrument::MAX_INSTRUMENT_VOLUME, pattern::PatternId, song::OctamedicSong},
};

// also the most trailing silence a recording can have
const CHUNK_FRAMES: usize = 256;

const DEFAULT_SAMPLE_RATE: u32 = 44_100;

// songs that loop forever still have to stop somewhere
const DEFAULT_MAX_DURATION: Duration = Duration::from_secs(30 * 60);

const WAV_HEADER_SIZE: usize = 44;
// "RIFF" + the size field, not counted in the size
const RIFF_HEADER_SIZE: usize = 8;
const FORMAT_CHUNK_SIZE: u32 = 16;
const PCM_FORMAT: u16 = 1;
const BITS_PER_SAMPLE: u16 = 16;

#[derive(Debug, PartialEq, Eq)]
pub enum RecordError {
    NoSuchSequenceIndex(usize),
    NoSuchPattern(PatternId),
}

impl Display for RecordError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            RecordError::NoSuchSequenceIndex(index) => {
                write!(f, "the song has no sequence position {index}")
            }
            RecordError::NoSuchPattern(id) => write!(f, "the song has no block {}", id.0),
        };
    }
}

impl std::error::Error for RecordError {}

// offline render, own engine, no audio device
pub struct Recorder {
    pub sample_rate: u32,
    pub machine: AmigaClock,
    pub output_volume: u8,
    pub max_duration: Duration,
    pub channels: u16,
}

impl Default for Recorder {
    fn default() -> Self {
        return Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            machine: AmigaClock::PAL,
            output_volume: MAX_INSTRUMENT_VOLUME,
            max_duration: DEFAULT_MAX_DURATION,
            channels: 1,
        };
    }
}

impl Recorder {
    pub fn record_song(
        &self,
        song: &OctamedicSong,
        sequence_start: usize,
    ) -> Result<Recording, RecordError> {
        return self.record_range(song, sequence_start..song.sequence().len());
    }

    /// Renders only `sequence_range` of the song's play sequence, stopping once it's played
    /// through. The end clamps to the song's length; it doesn't have to land on a real position.
    pub fn record_range(
        &self,
        song: &OctamedicSong,
        sequence_range: Range<usize>,
    ) -> Result<Recording, RecordError> {
        if song.sequence().get(sequence_range.start).is_none() {
            return Err(RecordError::NoSuchSequenceIndex(sequence_range.start));
        }
        let end = sequence_range
            .end
            .clamp(sequence_range.start, song.sequence().len());
        let mut commands = EngineCommand::load_song(song);
        if let Some(EngineCommand::SetSequence { sequence }) = commands.last_mut() {
            sequence.truncate(end);
            sequence.drain(..sequence_range.start);
        }
        commands.push(EngineCommand::SetOutputVolume(self.output_volume));
        commands.push(EngineCommand::PlaySong {
            sequence_start: 0,
            row: 0,
            looping: false,
        });
        return Ok(self.record(commands));
    }

    pub fn record_pattern(
        &self,
        song: &OctamedicSong,
        pattern: PatternId,
    ) -> Result<Recording, RecordError> {
        if song.get_pattern(&pattern).is_none() {
            return Err(RecordError::NoSuchPattern(pattern));
        }
        let mut commands = EngineCommand::load_song(song);
        commands.push(EngineCommand::SetOutputVolume(self.output_volume));
        commands.push(EngineCommand::PlayPattern {
            pattern,
            row: 0,
            looping: false,
        });
        return Ok(self.record(commands));
    }

    fn record(&self, setup: Vec<EngineCommand>) -> Recording {
        let (commands, receiver) = mpsc::channel();
        let status = Arc::new(EngineStatus::default());
        let mut engine =
            OctamedicEngine::new(receiver, status.clone(), self.sample_rate, self.machine);
        for command in setup {
            let _ = commands.send(command);
        }
        let channels = self.channels.max(1) as usize;
        let max_samples =
            (self.max_duration.as_secs_f64() * self.sample_rate as f64) as usize * channels;
        let mut samples = Vec::new();
        let mut chunk = vec![0.0; CHUNK_FRAMES * channels];
        while samples.len() < max_samples {
            engine.process(&mut chunk, channels);
            samples.extend_from_slice(&chunk);
            if status.play_mode() == PlayMode::Paused {
                break;
            }
        }
        samples.truncate(max_samples);
        return Recording {
            sample_rate: self.sample_rate,
            channels: channels as u16,
            samples,
        };
    }
}

// interleaved per `channels`, -1..1
pub struct Recording {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl Recording {
    pub fn duration(&self) -> Duration {
        let frames = self.samples.len() / self.channels.max(1) as usize;
        return Duration::from_secs_f64(frames as f64 / self.sample_rate as f64);
    }

    pub fn to_pcm16_bytes(&self) -> Vec<u8> {
        return self
            .samples
            .iter()
            .flat_map(|sample| ((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes())
            .collect();
    }

    pub fn to_wav_bytes(&self) -> Vec<u8> {
        let data = self.to_pcm16_bytes();
        let channels = self.channels.max(1);
        let block_align = channels * BITS_PER_SAMPLE / u8::BITS as u16;
        let mut wav = Vec::with_capacity(WAV_HEADER_SIZE + data.len());
        wav.extend_from_slice(b"RIFF");
        let riff_size = WAV_HEADER_SIZE - RIFF_HEADER_SIZE + data.len();
        wav.extend_from_slice(&(riff_size as u32).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&FORMAT_CHUNK_SIZE.to_le_bytes());
        wav.extend_from_slice(&PCM_FORMAT.to_le_bytes());
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&self.sample_rate.to_le_bytes());
        wav.extend_from_slice(&(self.sample_rate * block_align as u32).to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&BITS_PER_SAMPLE.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(&data);
        return wav;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::data::{
        instrument::{
            InstrumentId, InstrumentKind, OctamedicInstrument, OctamedicSample, SampleOctaves,
        },
        note::OctamedicNote,
        tempo::OctamedicTempo,
    };

    const SAMPLE_RATE: u32 = 1000;

    // 4 lines, 1 tick each, 100 frames per line
    fn song() -> OctamedicSong {
        let mut song = OctamedicSong::new("test");
        let mut tempo = OctamedicTempo::new();
        tempo.primary_tempo = 25;
        tempo.ticks_per_line = 1;
        *song.tempo_mut() = tempo;
        let pattern = song.ensure_pattern(PatternId(0));
        pattern.lines.truncate(4);
        let cell = pattern.track_mut(0, 0).unwrap();
        cell.note = OctamedicNote { value: 13 };
        cell.instrument_id = InstrumentId(1);
        let mut instrument = OctamedicInstrument::new("square");
        instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data: vec![100, -100],
            repeat: Some((0, 2)),
            octaves: SampleOctaves::One,
        }));
        song.set_instrument(InstrumentId(1), Some(instrument));
        return song;
    }

    fn recorder() -> Recorder {
        return Recorder {
            sample_rate: SAMPLE_RATE,
            ..Recorder::default()
        };
    }

    #[test]
    fn record_block() {
        let recording = recorder().record_pattern(&song(), PatternId(0)).unwrap();
        // plus up to a chunk of silence
        assert!(recording.samples.len() >= 400);
        assert!(recording.samples.len() < 400 + CHUNK_FRAMES);
        assert!(recording.samples[..400].iter().any(|&sample| sample != 0.0));
        assert!(recording.samples[400..].iter().all(|&sample| sample == 0.0));
    }

    #[test]
    fn record_song() {
        let recording = recorder().record_song(&song(), 0).unwrap();
        assert!(recording.samples.len() >= 400);
    }

    // 3 positions, each the same block as `song()`'s
    fn multi_sequence_song() -> OctamedicSong {
        let mut song = song();
        song.sequence_mut().extend([PatternId(0), PatternId(0)]);
        return song;
    }

    #[test]
    fn record_range_stops_before_the_rest_of_the_song() {
        let song = multi_sequence_song();
        let full = recorder().record_song(&song, 0).unwrap();
        let ranged = recorder().record_range(&song, 0..2).unwrap();
        assert!(ranged.samples.len() >= 800);
        assert!(ranged.samples.len() < 800 + CHUNK_FRAMES);
        assert!(ranged.samples.len() < full.samples.len());
    }

    #[test]
    fn record_range_can_start_partway_through() {
        let ranged = recorder()
            .record_range(&multi_sequence_song(), 1..3)
            .unwrap();
        assert!(ranged.samples.len() >= 800);
        assert!(ranged.samples.len() < 800 + CHUNK_FRAMES);
    }

    #[test]
    fn record_range_end_clamps_to_the_song_length() {
        let song = multi_sequence_song();
        let full = recorder().record_song(&song, 0).unwrap();
        let ranged = recorder().record_range(&song, 0..100).unwrap();
        assert_eq!(ranged.samples.len(), full.samples.len());
    }

    #[test]
    fn record_range_needs_a_real_start() {
        assert_eq!(
            recorder().record_range(&multi_sequence_song(), 5..6).err(),
            Some(RecordError::NoSuchSequenceIndex(5))
        );
    }

    #[test]
    fn stereo_interleaves_both_channels() {
        let mono = recorder().record_pattern(&song(), PatternId(0)).unwrap();
        let stereo = Recorder {
            channels: 2,
            ..recorder()
        }
        .record_pattern(&song(), PatternId(0))
        .unwrap();
        assert_eq!(stereo.channels, 2);
        assert_eq!(stereo.samples.len(), mono.samples.len() * 2);
        assert_eq!(stereo.duration(), mono.duration());
        let wav = stereo.to_wav_bytes();
        assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 2);
    }

    #[test]
    fn missing() {
        assert_eq!(
            recorder().record_song(&song(), 5).err(),
            Some(RecordError::NoSuchSequenceIndex(5))
        );
        assert_eq!(
            recorder().record_pattern(&song(), PatternId(9)).err(),
            Some(RecordError::NoSuchPattern(PatternId(9)))
        );
    }

    #[test]
    fn max_duration() {
        let mut song = song();
        let line = song.get_pattern(&PatternId(0)).unwrap().lines[0].clone();
        song.ensure_pattern(PatternId(0)).lines.resize(10_000, line);
        let recording = Recorder {
            max_duration: Duration::from_secs(2),
            ..recorder()
        }
        .record_pattern(&song, PatternId(0))
        .unwrap();
        assert_eq!(recording.samples.len(), 2 * SAMPLE_RATE as usize);
    }

    #[test]
    fn output_volume() {
        let full = recorder().record_pattern(&song(), PatternId(0)).unwrap();
        let half = Recorder {
            output_volume: 32,
            ..recorder()
        }
        .record_pattern(&song(), PatternId(0))
        .unwrap();
        let peak = |recording: &Recording| {
            recording
                .samples
                .iter()
                .fold(0.0f32, |peak, sample| peak.max(sample.abs()))
        };
        assert!((peak(&half) - peak(&full) / 2.0).abs() < 1e-6);
    }

    #[test]
    fn wav_header() {
        let recording = Recording {
            sample_rate: 22050,
            channels: 1,
            samples: vec![0.0, 1.0, -1.0],
        };
        let wav = recording.to_wav_bytes();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 22050);
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 6);
        assert_eq!(wav.len(), 44 + 6);
        assert_eq!(&wav[44..], &[0, 0, 0xFF, 0x7F, 0x01, 0x80]);
    }

    #[test]
    fn bpm_block() {
        let mut song = song();
        // 96 bpm, lpb 16, 6 ticks/line, so 16 lines = 1 beat
        let mut tempo = OctamedicTempo::new();
        tempo.is_bpm_mode = true;
        tempo.primary_tempo = 96;
        tempo.lines_per_beat = 16;
        tempo.ticks_per_line = 6;
        *song.tempo_mut() = tempo;
        let line = song.get_pattern(&PatternId(0)).unwrap().lines[0].clone();
        song.ensure_pattern(PatternId(0)).lines.resize(16, line);
        let recording = Recorder {
            sample_rate: 48000,
            ..Recorder::default()
        }
        .record_pattern(&song, PatternId(0))
        .unwrap();
        // 60 / 96 s plus up to a chunk

        assert!(recording.samples.len() >= 30000);
        assert!(recording.samples.len() < 30000 + CHUNK_FRAMES);
    }
}
