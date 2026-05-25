use clap_derive::Args;

use crate::commands::repl::{Command, CommandResult};

#[derive(Args, Debug, Clone)]
#[command(about = "Shows color information")]
pub struct PalleteCommand {}

impl Command for PalleteCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {
        if let Some(exp) = &mmd.expansion_data {
            for (i, color) in exp.color_pallete.colors.iter().enumerate() {
                let (r, g, b) = color.as_rgb_4();
                println!("colour[{i:2}] = 0x{:04X}  rgb({}, {}, {})", color.value.0, r.0, g.0, b.0);
            }
        } else {
            println!("no expansion data / no colour palette");
        }
        return Ok(());
    }
}
