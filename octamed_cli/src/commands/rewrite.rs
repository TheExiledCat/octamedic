use std::{ fs::File, io::Write };

use clap_derive::Args;
use octamed::mmd::writer::OctamedMMDWriter;

use crate::commands::repl::{ Command, CommandResult };

#[derive(Args, Debug, Clone)]
#[command(about = "Debug command")]
pub struct RewriteCommand {}

impl Command for RewriteCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {
        let bytes = OctamedMMDWriter::new().write_module(mmd).unwrap();
        let mut file_name = String::from("writer.");
        let extension = match mmd.get_type() {
            octamed::mmd::module::OctamedMMDType::MMD0 => "mmd0",
            octamed::mmd::module::OctamedMMDType::MMD1 => "mmd1",
            octamed::mmd::module::OctamedMMDType::MMD2 => "mmd2",
            octamed::mmd::module::OctamedMMDType::MMD3 => "mmd3",
        };
        file_name.push_str(extension);
        let mut file = File::create(file_name).unwrap();
        file.write_all(&bytes).unwrap();
        return Ok(());
    }
}
