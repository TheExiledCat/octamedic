use std::path::PathBuf;

use clap_derive::{Args, Subcommand};
use octamed::mmd::conversion::FromModule;
use octamedic_core::{
    data::{pattern::PatternId, project::OctamedicProject, song::SongId},
    engine::record::Recorder,
};

use crate::commands::{
    play::zero_to_64,
    repl::{Command, CommandError, CommandResult},
};

#[derive(Args, Debug, Clone)]
#[command(about = "Record the song or a block to a wav file.")]

pub struct RecordCommand {
    #[command(subcommand)]
    kind: RecordKind,
}

#[derive(Subcommand, Debug, Clone)]

enum RecordKind {
    #[command(about = "Record one block, played once.")]
    Pattern {
        #[arg(help = "The block number to record. Use 'blocks' to see possible values")]
        block: u8,
        #[arg(help = "The wav file to write. Creates a new file if needed")]
        output: PathBuf,
        #[command(flatten)]
        options: RecordOptions,
    },
    #[command(about = "Record the song from a sequence position to its end.")]
    Song {
        #[arg(help = "The wav file to write. Creates a new file if needed")]
        output: PathBuf,
        #[arg(
            short,
            long,
            help = "The sequence position to start from. Use 'sequence' to see possible values",
            default_value = "0"
        )]
        start: usize,
        #[command(flatten)]
        options: RecordOptions,
    },
}

#[derive(Args, Debug, Clone)]

struct RecordOptions {
    #[arg(
        short,
        long,
        help = "The volume to record at (0-64)",
        default_value = "64",
        value_parser = zero_to_64
    )]
    volume: u8,
    #[arg(
        short = 'r',
        long,
        help = "The sample rate of the wav file in Hz",
        default_value = "44100"
    )]
    sample_rate: u32,
}

impl RecordOptions {
    fn recorder(&self) -> Recorder {

        return Recorder {
            sample_rate: self.sample_rate,
            output_volume: self.volume,
            ..Recorder::default()
        };
    }
}

impl Command for RecordCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {

        let project = OctamedicProject::from_module(mmd);

        let song = project
            .get_song(SongId(0))
            .ok_or_else(|| CommandError::Generic("module has no song".into()))?;

        let error = |e: &dyn std::fmt::Display| CommandError::Generic(e.to_string());

        let (recording, output) = match &self.kind {
            RecordKind::Pattern {
                block,
                output,
                options,
            } => (
                options
                    .recorder()
                    .record_pattern(song, PatternId(*block))
                    .map_err(|e| error(&e))?,
                output,
            ),
            RecordKind::Song {
                output,
                start,
                options,
            } => (
                options
                    .recorder()
                    .record_song(song, *start)
                    .map_err(|e| error(&e))?,
                output,
            ),
        };

        std::fs::write(output, recording.to_wav_bytes()).map_err(|e| error(&e))?;

        println!(
            "Recorded {:.1}s to {}",
            recording.duration().as_secs_f32(),
            output.to_string_lossy()
        );

        return Ok(());
    }
}

#[cfg(test)]

mod tests {

    use std::path::Path;

    use clap::Parser;
    use octamed::mmd::{module::OctamedMMD, parser::OctamedMMDParser};

    use super::*;

    #[derive(Parser)]

    struct Cli {
        #[command(flatten)]
        record: RecordCommand,
    }

    fn module() -> OctamedMMD {

        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("example_meds/example.mmd0");

        return OctamedMMDParser::new().parse_file(&path).unwrap().remove(0);
    }

    fn run(args: &[&str]) -> CommandResult {

        let cli =
            Cli::try_parse_from(std::iter::once("record").chain(args.iter().copied())).unwrap();

        return cli.record.run(&mut module());
    }

    fn output(name: &str) -> PathBuf {

        return std::env::temp_dir()
            .join(format!("octamedic-record-{}-{name}", std::process::id()));
    }

    #[test]

    fn record_pattern_writes_a_wav_file() {

        let file = output("block.wav");

        run(&["pattern", "0", file.to_str().unwrap(), "-r", "8000"]).unwrap();

        let wav = std::fs::read(&file).unwrap();

        std::fs::remove_file(&file).unwrap();

        assert_eq!(&wav[0..4], b"RIFF");

        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 8000);

        assert!(wav.len() > 44);
    }

    #[test]

    fn record_song_starts_from_the_given_position() {

        let file = output("song.wav");

        run(&[
            "song",
            file.to_str().unwrap(),
            "--start",
            "0",
            "-r",
            "8000",
            "-v",
            "32",
        ])
        .unwrap();

        assert!(std::fs::metadata(&file).unwrap().len() > 44);

        std::fs::remove_file(&file).unwrap();
    }

    #[test]

    fn a_block_that_doesnt_exist_is_an_error_and_writes_nothing() {

        let file = output("missing.wav");

        assert!(run(&["pattern", "200", file.to_str().unwrap()]).is_err());

        assert!(!file.exists());
    }

    #[test]

    fn volume_above_64_is_rejected() {

        assert!(Cli::try_parse_from(["record", "song", "out.wav", "-v", "65"]).is_err());
    }

    #[test]

    fn a_bpm_mode_block_records_at_its_bpm() {

        // hiphop2.med: BPM mode, 96 BPM, 16 lines per beat, 6 ticks per line
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("example_meds/hiphop2.med");

        let mut mmd = OctamedMMDParser::new().parse_file(&path).unwrap().remove(0);

        let lines = OctamedicProject::from_module(&mmd)
            .get_song(SongId(0))
            .unwrap()
            .get_pattern(&PatternId(0))
            .unwrap()
            .line_count();

        let file = output("hiphop.wav");

        let cli = Cli::try_parse_from([
            "record",
            "pattern",
            "0",
            file.to_str().unwrap(),
            "-r",
            "48000",
        ])
        .unwrap();

        cli.record.run(&mut mmd).unwrap();

        let wav = std::fs::read(&file).unwrap();

        std::fs::remove_file(&file).unwrap();

        let frames = (wav.len() - 44) / 2;

        // a line lasts 6 ticks at 96 * 16 / 10 = 153.6 Hz: 1875 frames at 48 kHz
        let expected = lines * 1875;

        assert!(frames >= expected, "{frames} < {expected}");

        assert!(frames < expected + 256, "{frames} vs {expected}");
    }
}
