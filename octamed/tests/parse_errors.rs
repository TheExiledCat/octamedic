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
fn an_unknown_id_is_not_a_module() {
    assert!(matches!(
        parse(header(b"RIFF")),
        Err(MmdError::UnknownFormat(_))
    ));
}

#[test]
fn a_header_without_a_song_is_missing_a_pointer() {
    assert!(matches!(
        parse(header(b"MMD0")),
        Err(MmdError::MissingPointer("song"))
    ));
}

#[test]
fn a_truncated_file_is_an_io_error() {
    assert!(matches!(parse(b"MMD0".to_vec()), Err(MmdError::Io(_))));
}
