use std::fmt::format;

use clap_derive::Args;

use crate::commands::repl::{ Command, CommandError, CommandResult };

#[derive(Args, Debug, Clone)]
#[command(about = "Inspect the blocks in the module")]
pub struct InspectBlocksCommand {}

impl Command for InspectBlocksCommand {
    fn run(&self, mmd: &mut octamed::mmd0::module::OctamedMMD0) -> CommandResult {
        for (i, block) in mmd.block_table.headers.iter().enumerate() {
            println!("Block: {}", i);
            println!("Line count: {}", block.line_count.0 + 1);
            println!("Track count: {}", block.track_count);
            println!();
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
    fn run(&self, mmd: &mut octamed::mmd0::module::OctamedMMD0) -> CommandResult {
        clearscreen::clear().unwrap();

        let block = mmd.block_table.blocks
            .get(self.block_number)
            .ok_or(CommandError::Generic(format!("Failed to find block: {}", self.block_number)))?;
        let header = mmd.block_table.headers.get(self.block_number).unwrap();
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
