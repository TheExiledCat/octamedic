use std::time::Duration;

use clap_derive::Args;
use octamed::mmd::conversion::FromModule;
use octamedic_core::{
    data::{project::OctamedicProject, song::SongId},
    engine::{engine::PlayMode, handle::EngineHandle},
};

use crate::commands::repl::{Command, CommandError, CommandResult};

#[derive(Args, Debug, Clone)]
#[command(about = "Play the module.")]

pub struct PlayCommand {}

impl Command for PlayCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {

        let project = OctamedicProject::from_module(mmd);

        let song = project
            .get_song(SongId(0))
            .ok_or_else(|| CommandError::Generic("module has no song".into()))?;

        let engine = EngineHandle::spawn().map_err(|e| CommandError::Generic(e.to_string()))?;

        engine.load_song(song);

        engine.play_song(0, false);

        println!("Playing... (press Ctrl+C to stop)");

        // Give the engine a moment to pick up the play command before polling its status.
        std::thread::sleep(Duration::from_millis(100));

        let mut last_row = None;

        while engine.status().play_mode() != PlayMode::Paused {

            let status = engine.status();

            let row = (status.sequence_index(), status.pattern(), status.row());

            if last_row != Some(row) {

                println!("seq {:03}  block {:03}  line {:03}", row.0, row.1, row.2);

                last_row = Some(row);
            }

            std::thread::sleep(Duration::from_millis(5));
        }

        return Ok(());
    }
}
