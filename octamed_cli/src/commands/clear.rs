use clap_derive::Args;

use crate::commands::repl::{ Command, CommandResult };

#[derive(Args, Debug, Clone)]
#[command(name = "clear", about = "Clears the terminal for readability")]
pub struct ClearCommand {}

impl Command for ClearCommand {
    fn run(&self, mmd: &mut octamed::mmd0::module::OctamedMMD0) -> CommandResult {
        clearscreen::clear().unwrap();
        return Ok(());
    }
}
