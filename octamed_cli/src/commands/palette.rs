use clap_derive::Args;
use octamed::mmd::conversion::FromModule;
use octamedic_core::data::{palette::PALETTE_SIZE, project::OctamedicProject};

use crate::commands::repl::{Command, CommandResult};

#[derive(Args, Debug, Clone)]
#[command(about = "Shows color information")]
pub struct PaletteCommand {}

impl Command for PaletteCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {
        // modules without a palette show OctaMED's default one, as the editor does
        let palette = OctamedicProject::from_module(mmd).palette();
        for pen in 0..PALETTE_SIZE {
            let [r, g, b] = palette.rgb(pen);
            println!("Color {pen}: [{r}, {g}, {b}]");
        }
        return Ok(());
    }
}
