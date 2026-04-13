use std::{ env::{ self, ArgsOs }, path::PathBuf };

use octamed::{ mmd0::parser::OctamedMMD0Parser, utility::logger::ConsoleLogger };
fn main() {
    let args: Vec<String> = env::args().collect();
    let path = PathBuf::from(args.get(1).unwrap());
    OctamedMMD0Parser::parse_file(path.as_path(), ConsoleLogger::new()).unwrap()
}
