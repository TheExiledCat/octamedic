use std::io::Cursor;

use octamed::{
    mmd::{error::MmdError, parser::OctamedMMDParser},
    utility::bytes::Offset,
};

fn parse(bytes: Vec<u8>) -> Result<(), MmdError> {
    return OctamedMMDParser::new()
        .parse_module(&mut Cursor::new(bytes), Offset(0))
        .map(|_| ());
}

fn header(id: &[u8; 4]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.resize(52, 0);
    return bytes;
}

#[test]
fn unknown_id() {
    assert!(matches!(
        parse(header(b"RIFF")),
        Err(MmdError::UnknownFormat(_))
    ));
}

#[test]
fn no_song_pointer() {
    assert!(matches!(
        parse(header(b"MMD0")),
        Err(MmdError::MissingPointer("song"))
    ));
}

#[test]
fn truncated_file() {
    assert!(matches!(parse(b"MMD0".to_vec()), Err(MmdError::Io(_))));
}
