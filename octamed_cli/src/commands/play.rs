use std::{
    ops::Add,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use crate::commands::repl::{Command, CommandError, CommandResult};
use clap_derive::Args;
use clap_num::number_range;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use octamed::utility::{frequency::Frequency, period::AmigaPalPeriod};
use progress_bar::{
    Color, Style, finalize_progress_bar, inc_progress_bar, init_progress_bar,
    set_progress_bar_action, set_progress_bar_width,
};

#[derive(Args, Debug, Clone)]
#[command(about = "Play a sample. Does not take into account sample transpose.")]
pub struct PlayCommand {
    #[arg(help = "The sample number to play.  Use 'instruments' to see possible values")]
    sample_number: usize,

    #[arg(
        help = "The sampling frequency in Amiga PAL Periods. (default is about 16.5khz or the C-3 note)",
        default_value = "214"
    )]
    periods: u16,
    #[arg(short,long,help = "The volume to play the sample at (0-100)", default_value="50", value_parser=zero_to_100)]
    volume: u8,
    #[arg(
        short,
        long,
        help = "The interpolation mode to use for resampling. Defaults to nearest neighbor sampling, set to true to enable linear smoothing"
    )]
    fill: bool,
}

fn zero_to_100(text: &str) -> Result<u8, String> {
    number_range(text, 0, 100)
}
impl Command for PlayCommand {
    fn run(&self, mmd: &mut octamed::mmd0::module::OctamedMMD0) -> CommandResult {
        let header = mmd.sample_table.headers[self.sample_number]
            .as_ref()
            .ok_or(CommandError::Generic(format!(
                "Sample not found. Selectable samples: 0-{}",
                mmd.song.sample_count
            )))?;
        let sample: Vec<i8> = mmd.sample_table.samples[self.sample_number]
            .as_ref()
            .unwrap()
            .iter()
            .map(|b| b.0)
            .collect();
        let bits_per_sample = if header.is_16_bit { 16 } else { 8 };
        let channels = if header.is_stereo { 2 } else { 1 };
        let sample_rate = Frequency::period(AmigaPalPeriod::new(self.periods))
            .as_hertz()
            .floor() as u32;
        let host = cpal::default_host();
        let device = host.default_output_device().expect("No output device");
        let config = device.default_output_config().unwrap();
        let output_sample_rate = config.sample_rate();
        let output_channels = config.channels();

        let mut index = 0;
        let samples = if self.fill {
            resample_linear(sample.as_slice(), sample_rate, output_sample_rate)
        } else {
            resample_nearest(sample.as_slice(), sample_rate, output_sample_rate)
        };
        let sample_length = samples.len();
        let volume = self.volume as f32;
        let stream = device
            .build_output_stream(
                &config.config(),
                move |data: &mut [f32], _| {
                    for frame in data.chunks_mut(output_channels as usize) {
                        let value = *samples.get(index).unwrap_or(&0.0);

                        for channel in frame {
                            *channel = value * (volume / 100.0);
                        }
                        index += 1;
                    }
                },
                |e| panic!("{}", e),
                None,
            )
            .unwrap();

        let duration_secs = (sample_length as f32) / (output_sample_rate as f32);

        stream.play().unwrap();
        thread::sleep(Duration::from_secs_f32(duration_secs));
        return Ok(());
    }
}

fn resample_linear(samples: &[i8], input_sample_rate: u32, output_sample_rate: u32) -> Vec<f32> {
    let ratio = (input_sample_rate as f32) / (output_sample_rate as f32);
    let output_length = ((samples.len() as f32) / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_length);

    for i in 0..output_length {
        let input_pos = (i as f32) * ratio;
        let index = input_pos
            .floor()
            .clamp(0.0, samples.len().saturating_sub(1) as f32) as usize;
        let next_index = (index.add(1)).clamp(0, samples.len().saturating_sub(1)) as usize;
        let t = input_pos - index as f32;

        let sample_8bit_first = samples[index] as f32;
        let sample_8bit_next = samples[next_index] as f32;
        let sample = sample_8bit_first + (sample_8bit_next - sample_8bit_first) * t;
        let sample_f32 = (sample as f32) / 128.0;
        output.push(sample_f32);
    }
    return output;
}
fn resample_nearest(samples: &[i8], input_sample_rate: u32, output_sample_rate: u32) -> Vec<f32> {
    let ratio = (input_sample_rate as f32) / (output_sample_rate as f32);
    let output_length = ((samples.len() as f32) / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_length);

    for i in 0..output_length {
        let input_pos = (i as f32) * ratio;
        let index = input_pos
            .floor()
            .clamp(0.0, samples.len().saturating_sub(1) as f32) as usize;
        let sample_8bit = samples[index];
        let sample_f32 = (sample_8bit as f32) / 128.0;
        output.push(sample_f32);
    }
    return output;
}
