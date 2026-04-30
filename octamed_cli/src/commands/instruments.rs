use clap_derive::Args;

use crate::commands::repl::{ Command, CommandResult };

#[derive(Args, Debug, Clone)]
#[command(about = "Show details about all the instruments in the module")]
pub struct InstrumentsCommand {}

impl Command for InstrumentsCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {
        println!("Instruments");
        println!();
        for inst in 0..mmd.song.sample_count.0 as usize {
            println!("Instrument {:02}", inst);
            let header = mmd.sample_table.headers.get(inst).unwrap().as_ref().unwrap();
            println!("Size (Bytes): {}", header.sample_length.0);
            let name = {
                if mmd.expansion_data.is_none() {
                    "N/A".into()
                } else {
                    let infos = &mmd.expansion_data.as_ref().unwrap().instrument_infos;
                    if infos.len() == 0 {
                        "No Name".into()
                    } else {
                        infos.get(inst).as_ref().unwrap().name.clone()
                    }
                }
            };
            println!("Name: {}", name);
            let bit_count = if header.is_16_bit { 16 } else { 8 };
            let channels = if header.is_stereo { "Stereo" } else { "Mono" };
            println!("Type: {} ({} bit, {})", header.sample_type, bit_count, channels);
            println!();
        }
        return Ok(());
    }
}
