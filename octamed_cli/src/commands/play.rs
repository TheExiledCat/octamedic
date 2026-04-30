use std::{ path::PathBuf, thread, time::{ Duration, Instant } };

use clap_derive::Args;
use cpal::traits::{ DeviceTrait, HostTrait, StreamTrait };
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
#[command(about = "Play a sample. Does not take into account sample transpose.")]
pub struct PlayCommand {
    #[arg(help = "The sample number to play.  Use 'instruments' to see possible values")]
    sample_number: usize,

    #[arg(
        help = "The sampling frequency in Amiga PAL Periods. (default is about 16.5khz or the C-3 note)",
        default_value = "214"
    )]
    periods: u16,
}

impl Command for PlayCommand {
    fn run(&self, mmd: &mut octamed::mmd0::module::OctamedMMD0) -> CommandResult {
        let header = mmd.sample_table.headers[self.sample_number]
            .as_ref()
            .ok_or(
                CommandError::Generic(
                    format!("Sample not found. Selectable samples: 0-{}", mmd.song.sample_count)
                )
            )?;
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
        let samples = resample_nearest(sample.as_slice(), sample_rate, output_sample_rate);
        let sample_length = samples.len();
        let stream = device
            .build_output_stream(
                &config.config(),
                move |data: &mut [f32], _| {
                    for frame in data.chunks_mut(output_channels as usize) {
                        let value = *samples.get(index).unwrap_or(&0.0);

                        for channel in frame {
                            *channel = value;
                        }
                        index += 1;
                    }
                },
                |e| { panic!("{}", e) },
                None
            )
            .unwrap();

        let duration_secs = (sample_length as f32) / (output_sample_rate as f32);

        stream.play().unwrap();
        thread::sleep(Duration::from_secs_f32(duration_secs));
        return Ok(());
    }
}

fn resample_nearest(samples: &[i8], input_sample_rate: u32, output_sample_rate: u32) -> Vec<f32> {
    let ratio = (input_sample_rate as f32) / (output_sample_rate as f32);
    let output_length = ((samples.len() as f32) / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(output_length);

    for i in 0..output_length {
        let input_pos = (i as f32) * ratio;
        let index = input_pos.round().clamp(0.0, samples.len().saturating_sub(1) as f32) as usize;
        let sample_8bit = samples[index];
        let sample_f32 = ((sample_8bit as f32) - 128.0) / 128.0;
        output.push(sample_f32);
    }
    return output;
}
