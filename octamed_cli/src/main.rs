use std::{ path::PathBuf };
mod commands;

use crate::commands::repl::MMDRepl;
fn main() {
    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = project_root.join("example_meds/example.mmd0");

    MMDRepl::start(path);
}
