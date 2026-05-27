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

        let mut mmd = OctamedMMDParser::new()
            .parse_file(&path)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        RewriteCommand {}.run(&mut mmd).unwrap();

        println!("Done.");

        return;
    }

    println!("Opening module: {}", path.to_string_lossy());

    MMDRepl::start(path);
}
