use std::path::PathBuf;

use clap_derive::Args;
use hound::{ WavSpec, WavWriter };
use octamed::utility::{ frequency::Frequency, period::AmigaPalPeriod };
use progress_bar::{
    Color,
    Style,
    finalize_progress_bar,
    inc_progress_bar,
    init_progress_bar,
    set_progress_bar_action,
    set_progress_bar_width,
};
use crate::commands::repl::{ Command, CommandError, CommandResult };

#[derive(Args, Debug, Clone)]
#[command(about = "Export a sample to wav file. Does not take into account sample transpose.")]
pub struct WavExportCommand {
    #[arg(help = "The sample number to export.  Use 'instruments' to see possible values")]
    sample_number: usize,
    #[arg(help = "The output file to write to. creates a new file if needed")]
    output: PathBuf,
    #[arg(
        short,
        long,
        help = "The sampling frequency in Amiga PAL Periods. (default is about 16.5khz or the C-3 note)",
        default_value = "214"
    )]
    periods: u16,
}

impl Command for WavExportCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {
        let header = mmd.sample_table.headers[self.sample_number]
            .as_ref()
            .ok_or(
                CommandError::Generic(
                    format!("Sample not found. Selectable samples: 0-{}", mmd.song.sample_count)
                )
            )?;
        let sample = mmd.sample_table.samples[self.sample_number].as_ref().unwrap();
        let bits_per_sample = if header.is_16_bit() { 16 } else { 8 };
        let channels = if header.is_stereo() { 2 } else { 1 };
        let sample_rate = Frequency::period(AmigaPalPeriod::new(self.periods))
            .as_hertz()
            .floor() as u32;
        let spec = WavSpec {
            bits_per_sample,
            channels,
            sample_format: hound::SampleFormat::Int,
            sample_rate,
        };
        let mut writer = WavWriter::create(&self.output, spec).unwrap();
        init_progress_bar(sample.len());
        set_progress_bar_action("Writing", Color::Blue, Style::Normal);
        set_progress_bar_width(100);
        for (i, sample) in sample.iter().enumerate() {
            writer.write_sample(sample.0).unwrap();
            inc_progress_bar();
        }
        finalize_progress_bar();
        return Ok(());
    }
}
