use std::{env, path::PathBuf};

mod commands;

use octamed::mmd::parser::OctamedMMDParser;

use crate::commands::{
    repl::{Command, MMDRepl},
    rewrite::RewriteCommand,
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = PathBuf::from(&args[1]);
    // --rewrite: parse, write, exit — no TTY needed
    if args.get(2).map(|s| s.as_str()) == Some("--rewrite") {
        println!("Parsing file {}", path.to_string_lossy());
        let mut mmd = match OctamedMMDParser::new().parse_file(&path) {
            Ok(modules) => modules
                .into_iter()
                .next()
                .expect("a parsed file has a module"),
            Err(error) => {
                eprintln!("Couldn't open {}: {error}", path.to_string_lossy());
                std::process::exit(1);
            }
        };
        if let Err(error) = (RewriteCommand {}).run(&mut mmd) {
            eprintln!("Couldn't rewrite it: {error:?}");
            std::process::exit(1);
        }
        println!("Done.");
        return;
    }
    println!("Opening module: {}", path.to_string_lossy());
    MMDRepl::start(path);
}
