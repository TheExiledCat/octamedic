use std::{ path::PathBuf };
mod commands;
use octamed::{ mmd0::parser::OctamedMMD0Parser };

use crate::commands::repl::MMDRepl;
fn main() {
    let path = PathBuf::from("example_meds/example.mmd0");
    let mmd = OctamedMMD0Parser::parse_file(path.as_path()).unwrap();

    let mut repl = MMDRepl::new(mmd.into_iter().next().unwrap());
    repl.start();
}
