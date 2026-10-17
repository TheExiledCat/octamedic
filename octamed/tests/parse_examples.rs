use std::path::{Path, PathBuf};

use octamed::mmd::parser::OctamedMMDParser;

fn example(name: &str) -> PathBuf {

    return Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../octamed_cli/example_meds")
        .join(name);
}

#[test]

fn every_example_module_parses() {

    for name in ["example.mmd0", "example.mmd1", "Bellsv2 mono.med"] {

        let result = OctamedMMDParser::new().parse_file(&example(name));

        assert!(result.is_ok(), "{name}: {:?}", result.err());
    }
}

#[test]

fn empty_instrument_slots_stay_empty_and_in_place() {

    let module = OctamedMMDParser::new()
        .parse_file(&example("Bellsv2 mono.med"))
        .unwrap()
        .remove(0);

    let table = &module.sample_table;

    assert_eq!(table.headers.len(), 36);

    assert_eq!(table.samples.len(), 36);

    for slot in 0..36 {

        let empty = [22, 27, 35].contains(&slot);

        assert_eq!(table.headers[slot].is_none(), empty, "slot {slot}");

        assert_eq!(table.samples[slot].is_none(), empty, "slot {slot}");
    }

    // the slot after an empty one is still read from its own pointer
    assert_eq!(table.headers[23].as_ref().unwrap().sample_length.0, 8335);
}
