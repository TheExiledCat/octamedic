use clap_derive::Args;

use crate::commands::repl::{ Command, CommandResult };

#[derive(Args, Debug, Clone)]
#[command(about = "Inspect the loaded mmd module.")]
pub struct InspectCommand {}

impl Command for InspectCommand {
    fn run(&self, mmd: &mut octamed::mmd0::module::OctamedMMD0) -> CommandResult {
        println!("{}\n{}", mmd.header, mmd.song);
        return Ok(());
    }
}
