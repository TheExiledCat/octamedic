use clap_derive::Args;

use crate::commands::repl::{Command, CommandResult};

#[derive(Args, Debug, Clone)]
#[command(about = "Inspect the loaded mmd module.")]

pub struct InspectCommand {}

impl Command for InspectCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {

        println!("{}\n{}", mmd.header, mmd.song);

        match &mmd.expansion_data {
            Some(e) => {

                println!("Song name: {}", e.song_name);

                println!("Comment: {}", e.annotation);
            }
            None => (),
        }

        return Ok(());
    }
}
