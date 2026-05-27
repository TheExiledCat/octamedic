use std::process::exit;

use clap_derive::Args;

use crate::commands::repl::{Command, CommandResult};

#[derive(Args, Debug, Clone)]
#[command(about = "Exit the command line interface")]

pub struct ExitCommand {}

impl Command for ExitCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {

        exit(0);
    }
}
