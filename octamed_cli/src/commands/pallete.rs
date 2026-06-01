
use clap_derive::Args;

use crate::commands::repl::{Command, CommandResult};

#[derive(Args, Debug, Clone)]
#[command(about = "Shows color information")]

pub struct PalleteCommand {}

impl Command for PalleteCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {

        let pallete = mmd.expansion_data.as_ref().unwrap().color_pallete.colors;

        for (i, color) in pallete.iter().enumerate() {

            let (r, g, b) = color.as_rgb_4();

            println!("Color {}: [{}, {}, {}]\n", i, r, g, b)
        }

        return Ok(());
    }
}
