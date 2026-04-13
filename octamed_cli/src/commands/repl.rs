use figlet_rs::Toilet;
use inquire::Text;
use octamed::mmd0::module::OctamedMMD0;

use crate::commands::inspect::InspectCommand;

pub trait Command {
    fn help(&self) -> (String, String);
    fn run(&self, mmd: &mut OctamedMMD0);
}
pub struct MMDRepl {
    mmd: OctamedMMD0,
    commands: Vec<Box<dyn Command>>,
}

impl MMDRepl {
    pub fn new(mmd: OctamedMMD0) -> Self {
        return Self { mmd, commands: vec![Box::new(InspectCommand::new())] };
    }

    pub fn start(&mut self) {
        let future_font = Toilet::future().unwrap();
        println!("{}", future_font.convert("MMD CLI").unwrap());
        println!("Octamed file loaded");
        println!("Use 'help' to see commands");
        println!("Use 'exit' to leave");
        loop {
            let command = self.read_line().trim().to_owned();

            if command == "help" {
                self.help();
            } else if command == "exit" {
                break;
            } else if self.commands.iter().any(|c| c.help().0 == command) {
                self.commands
                    .iter()
                    .find(|c| c.help().0 == command)
                    .unwrap()
                    .run(&mut self.mmd);
            } else {
            }
        }
    }
    fn help(&self) {
        println!("commands:");
        for command in &self.commands {
            let help = command.help();
            println!("{} - {}", help.0, help.1);
        }

        println!();
        println!("use 'help [command]' to see command details");
    }
    fn read_line(&mut self) -> String {
        return Text::new("").prompt().unwrap();
    }
}
