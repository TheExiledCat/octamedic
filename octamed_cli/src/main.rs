use std::{ path::PathBuf };
mod commands;
use octamed::{ mmd0::parser::OctamedMMD0Parser };

use crate::commands::repl::MMDRepl;
fn main() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = project_root.join("example_meds/example.mmd0");
    println!("Parsing file {}", path.to_string_lossy());
    let mut mmd = OctamedMMD0Parser::parse_file(path.as_path()).unwrap();
    println!("Module parsed\nStarting Repl...");
    MMDRepl::start(&mut mmd[0]);
}
