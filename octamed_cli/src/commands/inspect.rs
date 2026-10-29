use clap_derive::Args;

use crate::commands::repl::{Command, CommandResult};

#[derive(Args, Debug, Clone)]
#[command(about = "Inspect the loaded mmd module.")]
pub struct InspectCommand {}

impl Command for InspectCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {
        println!("{}\n{}", mmd.header, mmd.song);
        if let Some(e) = &mmd.expansion_data {
            println!("Song name: {}", e.song_name);
            println!("Comment: {}", e.annotation);
        }
        return Ok(());
    }
}
