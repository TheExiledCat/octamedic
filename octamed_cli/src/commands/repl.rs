use std::{iter, path::PathBuf};

use clap::{CommandFactory, Parser};
use clap_derive::Subcommand;
use figlet_rs::Toilet;
use inquire::{Autocomplete, Text, autocompletion::Replacement};
use octamed::mmd::{module::OctamedMMD, parser::OctamedMMDParser};

use crate::commands::{
    blocks::{InspectBlockCommand, InspectBlocksCommand},
    clear::ClearCommand,
    exit::ExitCommand,
    inspect::InspectCommand,
    instruments::InstrumentsCommand,
    pallete::PalleteCommand,
    play::PlayCommand,
    rewrite::RewriteCommand,
    sample::SampleCommand,
    sequence::ShowSequenceCommand,
    wavexport::WavExportCommand,
};

pub type CommandResult = Result<(), CommandError>;

pub trait Command {
    fn run(&self, mmd: &mut OctamedMMD) -> CommandResult;
}

#[derive(Debug)]

pub enum CommandError {
    Generic(String),
}

impl CommandError {
    pub fn print(&self) {

        match self {
            CommandError::Generic(e) => println!("{}", e),
        }
    }
}

#[derive(Parser)]
#[command(name = "mmd")]

pub struct MMDRepl {
    module_file: PathBuf,
}

#[derive(Parser)]
#[command(name = "")]

pub struct MMDCommand {
    #[command(subcommand)]
    command: MMDCommandKind,
}

#[derive(Subcommand)]

pub enum MMDCommandKind {
    #[command(name = "inspect")]
    Inspect(InspectCommand),
    #[command(name = "exit")]
    Exit(ExitCommand),
    #[command(name = "wavexport")]
    ExportWav(WavExportCommand),
    #[command(name = "clear")]
    Clear(ClearCommand),
    #[command(name = "block")]
    InspectBlock(InspectBlockCommand),
    #[command(name = "blocks")]
    InspectBlocks(InspectBlocksCommand),
    #[command(name = "instruments")]
    Instruments(InstrumentsCommand),
    #[command(name = "sample")]
    Sample(SampleCommand),
    #[command(name = "sequence")]
    Sequence(ShowSequenceCommand),
    #[command(name = "rewrite")]
    Rewrite(RewriteCommand),
    #[command(name = "pallete")]
    Pallete(PalleteCommand),
    #[command(name = "play")]
    Play(PlayCommand),
}

impl MMDCommandKind {
    pub fn run(&mut self, mmd: &mut OctamedMMD) -> CommandResult {

        match self {
            MMDCommandKind::Inspect(c) => c.run(mmd),
            MMDCommandKind::Exit(c) => c.run(mmd),
            MMDCommandKind::ExportWav(c) => c.run(mmd),
            MMDCommandKind::Clear(c) => c.run(mmd),
            MMDCommandKind::InspectBlock(c) => c.run(mmd),
            MMDCommandKind::InspectBlocks(c) => c.run(mmd),
            MMDCommandKind::Instruments(c) => c.run(mmd),
            MMDCommandKind::Sample(c) => c.run(mmd),
            MMDCommandKind::Sequence(c) => c.run(mmd),
            MMDCommandKind::Rewrite(c) => c.run(mmd),
            MMDCommandKind::Pallete(c) => c.run(mmd),
            MMDCommandKind::Play(c) => c.run(mmd),
        }
    }
}

impl MMDRepl {
    pub fn start(path: PathBuf) {

        println!("Parsing file {}", path.to_string_lossy());

        let mut mmd = OctamedMMDParser::new()
            .parse_file(&path)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        println!("Module parsed\nStarting Repl...");

        let future_font = Toilet::future().unwrap();

        println!("{}", future_font.convert("MMD CLI").unwrap());

        println!("Octamed file loaded");

        println!("Use 'help' to see commands");

        println!("Use 'exit' to leave");

        let autocompleter = MMDReplCompleter::new(
            MMDCommand::command()
                .get_subcommands()
                .map(|c| c.get_name().to_owned())
                .chain(iter::once("help".into()))
                .collect::<Vec<String>>(),
        );

        loop {

            let input = Self::read_line(&autocompleter).trim().to_owned();

            let args = iter::once("").chain(input.split_whitespace());

            let command = MMDCommand::try_parse_from(args);

            match command {
                Ok(mut command) => {

                    let res = command.command.run(&mut mmd);

                    match res {
                        Ok(_) => (),
                        Err(e) => e.print(),
                    }
                }
                Err(e) => {

                    e.print().unwrap();
                }
            }

            // if command == "help" {
            //     self.help();
            // } else if command == "exit" {
            //     break;
            // } else if self.commands.iter().any(|c| c.help().0 == command) {
            //     self.commands
            //         .iter()
            //         .find(|c| c.help().0 == command)
            //         .unwrap()
            //         .run(&mut self.mmd);
            // } else {
            // }
        }
    }

    fn read_line(completer: &MMDReplCompleter) -> String {

        return Text::new("")
            .with_autocomplete(completer.clone())
            .prompt()
            .unwrap();
    }
}

#[derive(Clone)]

struct MMDReplCompleter {
    command_list: Vec<String>,
}

impl MMDReplCompleter {
    pub fn new(commands: Vec<String>) -> Self {

        return Self {
            command_list: commands,
        };
    }
}

impl Autocomplete for MMDReplCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, inquire::CustomUserError> {

        let input = input.split_whitespace().last().unwrap_or("");

        let sug = self
            .command_list
            .iter()
            .filter(|c| c.starts_with(input))
            .take(5)
            .map(|s| s.to_owned());

        return Ok(sug.collect::<Vec<String>>());
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<inquire::autocompletion::Replacement, inquire::CustomUserError> {

        Ok(match highlighted_suggestion {
            Some(s) => {

                let mut tokens = input
                    .split_whitespace()
                    .map(|t| t.to_owned())
                    .collect::<Vec<String>>();

                tokens.pop();

                tokens.push(s);

                Replacement::Some(tokens.join(" "))
            }
            None => Replacement::None,
        })
    }
}
