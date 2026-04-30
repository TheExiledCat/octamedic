use clap_derive::Args;

use crate::commands::repl::Command;
#[derive(Args, Debug, Clone)]
#[command(about = "Show the sequence block order.")]
pub struct ShowSequenceCommand;

impl Command for ShowSequenceCommand {
    fn run(&self, mmd: &mut octamed::mmd::module::OctamedMMD) -> super::repl::CommandResult {
        let sequence_length = mmd.song.song_length;
        println!("Sequence length: {} Blocks", sequence_length);
        for seq in 0..sequence_length.0 as usize {
            println!("{}: {:02}", seq, mmd.song.player_sequence_list[seq]);
        }

        return Ok(());
    }
}
