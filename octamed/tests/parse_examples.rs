use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

use octamed::{
    mmd::{module::OctamedMMD, parser::OctamedMMDParser, writer::OctamedMMDWriter},
    utility::bytes::Offset,
};

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

fn instrument_extensions(module: &OctamedMMD) -> Vec<(u8, u8, i8, usize)> {
    return module
        .expansion_data
        .as_ref()
        .unwrap()
        .external_instruments
        .iter()
        .map(|i| (i.hold.0, i.decay.0, i.fine_tune.0, i.rest.len()))
        .collect();
}

#[test]
fn instrument_extensions_survive_a_rewrite() {
    for name in ["example.mmd0", "example.mmd1"] {
        let module = OctamedMMDParser::new()
            .parse_file(&example(name))
            .unwrap()
            .remove(0);
        let written = OctamedMMDWriter::new().write_module(&module).unwrap();
        let reread = OctamedMMDParser::new()
            .parse_module(&mut Cursor::new(written), Offset(0))
            .unwrap();
        assert_eq!(
            instrument_extensions(&reread),
            instrument_extensions(&module),
            "{name}"
        );
    }
}

#[test]
fn finetunes_are_read_from_the_instrument_extensions() {
    let module = OctamedMMDParser::new()
        .parse_file(&example("example.mmd1"))
        .unwrap()
        .remove(0);
    let finetunes: Vec<i8> = instrument_extensions(&module)
        .iter()
        .map(|&(_, _, finetune, _)| finetune)
        .collect();
    assert_eq!(&finetunes[..9], &[-1, 0, 0, 0, 0, 0, 0, 0, 1]);
}
