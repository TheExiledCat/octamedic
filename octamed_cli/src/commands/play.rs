use std::time::Duration;

use clap_derive::Args;
use clap_num::number_range;
use octamed::mmd::conversion::FromModule;
use octamedic_core::{
    data::{project::OctamedicProject, song::SongId, volume::MAX_VOLUME},
    engine::{engine::PlayMode, handle::EngineHandle},
};

use crate::commands::repl::{Command, CommandError, CommandResult};

const START_WAIT: Duration = Duration::from_millis(100);

const POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Args, Debug, Clone)]
#[command(about = "Play the module.")]
pub struct PlayCommand {
    #[arg(
        short,
        long,
        help = "The volume to play the song at (0-64)",
        default_value = "32",
        value_parser = zero_to_64
    )]
    volume: u8,
}

impl Command for PlayCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {
        let project = OctamedicProject::from_module(mmd);
        let song = project
            .get_song(SongId(0))
            .ok_or_else(|| CommandError::Generic("module has no song".into()))?;
        let engine = EngineHandle::spawn().map_err(|e| CommandError::Generic(e.to_string()))?;
        engine.set_output_volume(self.volume);
        engine.load_song(song);
        engine.play_song(0, 0, false);
        println!("Playing... (press Ctrl+C to stop)");
        // engine needs a moment before status says it's playing

        std::thread::sleep(START_WAIT);
        let mut last_row = None;
        while engine.status().play_mode() != PlayMode::Paused {
            let status = engine.status();
            let row = (status.sequence_index(), status.pattern(), status.row());
            if last_row != Some(row) {
                println!("seq {:03}  block {:03}  line {:03}", row.0, row.1, row.2);
                last_row = Some(row);
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        return Ok(());
    }
}

pub fn zero_to_64(text: &str) -> Result<u8, String> {
    return number_range(text, 0, MAX_VOLUME);
}
