use std::{
    fmt::Display,
    sync::{Arc, mpsc},
    time::Duration,
};

use super::{
    engine::{OctamedicEngine, PlayMode},
    handle::{EngineCommand, EngineStatus},
};
use crate::data::{instrument::MAX_INSTRUMENT_VOLUME, pattern::PatternId, song::OctamedicSong};

// frames rendered per step; also the most silence a recording can end with
const CHUNK_FRAMES: usize = 256;

const DEFAULT_SAMPLE_RATE: u32 = 44_100;

/// The longest a recording runs by default, so a song that loops forever still ends.
const DEFAULT_MAX_DURATION: Duration = Duration::from_secs(30 * 60);

// a wav file: the RIFF header, then the format chunk and the data chunk
const WAV_HEADER_SIZE: usize = 44;
/// What the RIFF size leaves out: the "RIFF" tag and the size itself.
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

// renders songs and blocks offline with its own engine, without the audio device or the live engine
pub struct Recorder {
    pub sample_rate: u32,
    pub output_volume: u8,
    // a song that never ends, like one that jumps back, is cut off here
    pub max_duration: Duration,
}

impl Default for Recorder {
    fn default() -> Self {
        return Self {
            sample_rate: DEFAULT_SAMPLE_RATE,
            output_volume: MAX_INSTRUMENT_VOLUME,
            max_duration: DEFAULT_MAX_DURATION,
        };
    }
}

impl Recorder {
    // the song from `sequence_start` to its end, played once
    pub fn record_song(
        &self,
        song: &OctamedicSong,
        sequence_start: usize,
    ) -> Result<Recording, RecordError> {
        if song.sequence().get(sequence_start).is_none() {
            return Err(RecordError::NoSuchSequenceIndex(sequence_start));
        }
        return Ok(self.record(
            song,
            EngineCommand::PlaySong {
                sequence_start,
                row: 0,
                looping: false,
            },
        ));
    }

    // one block, played once
    pub fn record_pattern(
        &self,
        song: &OctamedicSong,
        pattern: PatternId,
    ) -> Result<Recording, RecordError> {
        if song.get_pattern(&pattern).is_none() {
            return Err(RecordError::NoSuchPattern(pattern));
        }
        return Ok(self.record(
            song,
            EngineCommand::PlayPattern {
                pattern,
                row: 0,
                looping: false,
            },
        ));
    }

    fn record(&self, song: &OctamedicSong, play: EngineCommand) -> Recording {
        let (commands, receiver) = mpsc::channel();
        let status = Arc::new(EngineStatus::default());
        let mut engine = OctamedicEngine::new(receiver, status.clone(), self.sample_rate);
        let setup = EngineCommand::load_song(song)
            .into_iter()
            .chain([EngineCommand::SetOutputVolume(self.output_volume), play]);
        for command in setup {
            // the engine owns the receiver and is still alive, so this can't fail
            let _ = commands.send(command);
        }
        let max_frames = (self.max_duration.as_secs_f64() * self.sample_rate as f64) as usize;
        let mut samples = Vec::new();
        let mut chunk = [0.0; CHUNK_FRAMES];
        while samples.len() < max_frames {
            engine.process(&mut chunk, 1);
            samples.extend_from_slice(&chunk);
            if status.play_mode() == PlayMode::Paused {
                break;
            }
        }
        samples.truncate(max_frames);
        return Recording {
            sample_rate: self.sample_rate,
            samples,
        };
    }
}

// mono audio in -1.0..1.0
pub struct Recording {
    pub sample_rate: u32,
    pub samples: Vec<f32>,
}

impl Recording {
    pub fn duration(&self) -> Duration {
        return Duration::from_secs_f64(self.samples.len() as f64 / self.sample_rate as f64);
    }

    // 16-bit signed little-endian PCM, the sample data of a wav file
    pub fn to_pcm16_bytes(&self) -> Vec<u8> {
        return self
            .samples
            .iter()
            .flat_map(|sample| ((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes())
            .collect();
    }

    // a complete mono 16-bit wav file
    pub fn to_wav_bytes(&self) -> Vec<u8> {
        let data = self.to_pcm16_bytes();
        let channels: u16 = 1;
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
        instrument::{InstrumentId, OctamedicInstrument, OctamedicSample},
        note::OctamedicNote,
        tempo::OctamedicTempo,
    };

    const SAMPLE_RATE: u32 = 1000;

    // 4 lines of 1 tick at 10 ticks a second: 100 frames per line at `SAMPLE_RATE`
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
        instrument.sample = Some(Arc::new(OctamedicSample {
            data: vec![100, -100],
            repeat: Some((0, 2)),
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
    fn a_block_records_its_lines_and_stops() {
        let recording = recorder().record_pattern(&song(), PatternId(0)).unwrap();
        // 4 lines of 100 frames, plus at most one chunk of silence after the end
        assert!(recording.samples.len() >= 400);
        assert!(recording.samples.len() < 400 + CHUNK_FRAMES);
        assert!(recording.samples[..400].iter().any(|&sample| sample != 0.0));
        assert!(recording.samples[400..].iter().all(|&sample| sample == 0.0));
    }

    #[test]
    fn a_song_records_from_the_sequence_start() {
        let recording = recorder().record_song(&song(), 0).unwrap();
        assert!(recording.samples.len() >= 400);
    }

    #[test]
    fn missing_positions_and_blocks_are_errors() {
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
    fn a_song_that_never_ends_is_cut_at_the_max_duration() {
        let mut song = song();
        // a block too long to finish within the limit
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
    fn the_output_volume_scales_the_recording() {
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
    fn the_wav_header_matches_the_data() {
        let recording = Recording {
            sample_rate: 22050,
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
    fn a_bpm_mode_block_plays_at_its_bpm() {
        let mut song = song();
        // 96 BPM, 16 lines per beat, 6 ticks per line: a 16-line block is exactly one beat
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
        // 60 / 96 seconds, plus at most one chunk of silence after the end
        assert!(recording.samples.len() >= 30000);
        assert!(recording.samples.len() < 30000 + CHUNK_FRAMES);
    }
}
