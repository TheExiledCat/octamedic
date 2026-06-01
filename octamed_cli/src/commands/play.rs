use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use clap_derive::Args;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use octamed::mmd::conversion::FromModule;
use octamedic_core::{data::project::OctamedicProject, engine::engine::OctamedicEngine};

use crate::commands::repl::{Command, CommandError, CommandResult};

#[derive(Args, Debug, Clone)]
#[command(about = "Play the module.")]
pub struct PlayCommand {}

fn cpal_err(e: impl std::fmt::Display) -> CommandError {
    CommandError::Generic(e.to_string())
}

impl Command for PlayCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {

        let project = OctamedicProject::from_module(mmd);
        let engine = Arc::new(Mutex::new(OctamedicEngine::new(project)));

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| CommandError::Generic("no output device available".into()))?;

        let config = device.default_output_config().map_err(cpal_err)?;
        let sample_rate = config.sample_rate();
        let channels = config.channels() as usize;

        let finished = Arc::new(Mutex::new(false));
        let finished_cb = Arc::clone(&finished);
        let engine_cb = Arc::clone(&engine);

        let stream = device
            .build_output_stream::<f32, _, _>(
                &config.config(),
                move |data: &mut [f32], _| {
                    let mut engine = engine_cb.lock().unwrap();
                    let frame_count = data.len() / channels;
                    let mut buf = vec![128u8; frame_count];
                    let done = engine.process(&mut buf, sample_rate);
                    for (frame, byte) in data.chunks_mut(channels).zip(buf) {
                        let sample = (byte as f32 - 128.0) / 128.0;
                        for out in frame.iter_mut() {
                            *out = sample;
                        }
                    }
                    if done {
                        *finished_cb.lock().unwrap() = true;
                    }
                },
                |err| eprintln!("audio error: {err}"),
                None,
            )
            .map_err(cpal_err)?;

        stream.play().map_err(cpal_err)?;

        println!("Playing... (press Ctrl+C to stop)");

        while !*finished.lock().unwrap() {
            std::thread::sleep(Duration::from_millis(10));
        }

        return Ok(());
    }
}
