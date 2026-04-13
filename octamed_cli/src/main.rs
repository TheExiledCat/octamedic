use std::{ env::{ self, ArgsOs }, path::PathBuf };

use octamed::{ mmd0::parser::OctamedMMD0Parser, utility::logger::ConsoleLogger };
fn main() {
    let path = PathBuf::from("example_meds/daft_punk.med");
    let res = OctamedMMD0Parser::parse_file(path.as_path(), ConsoleLogger::new()).unwrap();
}
