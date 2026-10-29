use std::{fs::File, io::Write};

use clap_derive::Args;
use octamed::mmd::{module::OctamedMMDType, writer::OctamedMMDWriter};

use crate::commands::repl::{Command, CommandError, CommandResult};

#[derive(Args, Debug, Clone)]
#[command(about = "Debug command")]
pub struct RewriteCommand {}

impl Command for RewriteCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {
        let error = |e: &dyn std::fmt::Display| CommandError::Generic(e.to_string());
        let bytes = OctamedMMDWriter::new()
            .write_module(mmd)
            .map_err(|e| error(&e))?;
        let extension = match mmd.get_type() {
            Some(OctamedMMDType::MMD0) => "mmd0",
            Some(OctamedMMDType::MMD1) => "mmd1",
            Some(OctamedMMDType::MMD2) => "mmd2",
            Some(OctamedMMDType::MMD3) => "mmd3",
            None => "mmd",
        };
        let mut file = File::create(format!("writer.{extension}")).map_err(|e| error(&e))?;
        file.write_all(&bytes).map_err(|e| error(&e))?;
        return Ok(());
    }
}
