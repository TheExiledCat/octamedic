use std::path::{Path, PathBuf};

use octamed::mmd::parser::OctamedMMDParser;

fn example(name: &str) -> PathBuf {
    return Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../octamed_cli/example_meds")
        .join(name);
}

#[test]
fn every_example_module_parses() {
    for name in ["example.mmd0", "example.mmd1"] {
        let result = OctamedMMDParser::new().parse_file(&example(name));
        assert!(result.is_ok(), "{name}: {:?}", result.err());
    }
}
