use clap_derive::Args;
use octamed::mmd::module::OctamedMMDBlockTable;

use crate::commands::repl::{Command, CommandError, CommandResult};

#[derive(Args, Debug, Clone)]
#[command(about = "Inspect the blocks in the module")]

pub struct InspectBlocksCommand {}

impl Command for InspectBlocksCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {

        match &mmd.block_table {
            OctamedMMDBlockTable::MMD0BlockTable {
                headers,
                blocks,
            } => {
                for (i, block) in headers.iter().enumerate() {

                    println!("Block: {}", i);

                    println!("Line count: {}", block.line_count.0 + 1);

                    println!("Track count: {}", block.track_count);

                    println!();
                }
            }
            OctamedMMDBlockTable::MMD1BlockTable {
                headers,
                blocks,
            } => {
                for (i, block) in headers.iter().enumerate() {

                    let name = if let Some(i) = &blocks[i].info {

                        if i.block_name.len() > 0 {

                            i.block_name.clone()
                        } else {

                            "No Name".into()
                        }
                    } else {

                        "No Name".into()
                    };

                    println!("Block: {} ({})", i, name);

                    println!("Line count: {}", block.line_count.0 + 1);

                    println!("Track count: {}", block.track_count);

                    println!();
                }
            }
        }

        return Ok(());
    }
}

#[derive(Args, Debug, Clone)]
#[command(about = "Inspect an entire block in detail")]

pub struct InspectBlockCommand {
    #[arg(help = "The block number to inspect. Use 'blocks' to see available blocks")]
    block_number: usize,
    #[arg(
        short,
        long,
        help = "The maximum number of block lines to render, defaults to all of them",
        default_value_t = usize::MAX
    )]
    max_lines: usize,
}

impl Command for InspectBlockCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> CommandResult {

        clearscreen::clear().unwrap();

        match &mmd.block_table {
            OctamedMMDBlockTable::MMD0BlockTable {
                headers,
                blocks,
            } => {

                let block = blocks
                    .get(self.block_number)
                    .ok_or(CommandError::Generic(format!(
                        "Failed to find block: {}",
                        self.block_number
                    )))?;

                let header = headers.get(self.block_number).unwrap();

                let tempo = mmd.song.get_tempo();

                println!(
                    "Tempo ({}): {} \n",
                    if tempo.is_bpm_mode() { "BPM" } else { "SPD" },
                    tempo
                );

                println!(
                    "Line count: {} (0-{})",
                    header.line_count.0 + 1,
                    header.line_count.0
                );

                println!(
                    "    {}",
                    (0..8)
                        .map(|i| format!("Track {:<3}", i))
                        .collect::<Vec<String>>()
                        .join("|")
                );

                for (i, line) in block.lines.iter().enumerate() {

                    print!("{:03} ", i);

                    println!(
                        "{}",
                        line.tracks
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<String>>()
                            .join("|")
                    );
                }

                return Ok(());
            }
            OctamedMMDBlockTable::MMD1BlockTable {
                headers,
                blocks,
            } => {

                let block = blocks
                    .get(self.block_number)
                    .ok_or(CommandError::Generic(format!(
                        "Failed to find block: {}",
                        self.block_number
                    )))?;

                let header = headers.get(self.block_number).unwrap();

                let tempo = mmd.song.get_tempo();

                println!(
                    "Tempo ({}): {} \n",
                    if tempo.is_bpm_mode() { "BPM" } else { "SPD" },
                    tempo
                );

                println!(
                    "Line count: {} (0-{})",
                    header.line_count.0 + 1,
                    header.line_count.0
                );

                println!(
                    "    {}",
                    (0..8)
                        .map(|i| format!("Track {:<3}", i))
                        .collect::<Vec<String>>()
                        .join("|")
                );

                for (i, line) in block.lines.iter().enumerate() {

                    print!("{:03} ", i);

                    println!(
                        "{}",
                        line.tracks
                            .iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<String>>()
                            .join("|")
                    );
                }

                return Ok(());
            }
        }
    }
}
