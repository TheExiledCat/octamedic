use std::{ env, path::PathBuf };
mod commands;

use crate::commands::repl::MMDRepl;
fn main() {
    let args: Vec<String> = env::args().collect();

    let path = PathBuf::from(&args[0]);

    MMDRepl::start(path);
}
