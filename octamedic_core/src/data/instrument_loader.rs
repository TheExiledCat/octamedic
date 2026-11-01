use std::{fmt::Display, fs, io, path::Path};

use crate::data::instrument::OctamedicInstrument;

/// Reads one audio file format into an instrument. Each format gets its own, and
/// [`load_instrument`] picks one by looking at the file's header, since plenty of files,
/// especially old Amiga ones, have no extension.
pub trait InstrumentLoader {
    /// Whether the file's bytes start the way this format does.
    fn recognizes(&self, bytes: &[u8]) -> bool;

    /// `name` is what the instrument gets called, usually the file name.
    fn load(&self, name: &str, bytes: &[u8]) -> Result<OctamedicInstrument, InstrumentLoadError>;
}

#[derive(Debug)]
pub enum InstrumentLoadError {
    Unreadable(io::Error),
    UnknownFormat,
    /// What's wrong with it, for the user. Keep it short.
    Malformed(String),
}

// short, it's shown in a notification or a dialog
impl Display for InstrumentLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            InstrumentLoadError::Unreadable(_) => write!(f, "Can't read file"),
            InstrumentLoadError::UnknownFormat => write!(f, "Not a sample"),
            InstrumentLoadError::Malformed(problem) => write!(f, "{problem}"),
        };
    }
}

impl std::error::Error for InstrumentLoadError {}

/// Loads `path` with the first of `loaders` that recognizes it, named after the file.
pub fn load_instrument(
    path: &Path,
    loaders: &[&dyn InstrumentLoader],
) -> Result<OctamedicInstrument, InstrumentLoadError> {
    let bytes = fs::read(path).map_err(InstrumentLoadError::Unreadable)?;
    let loader = loaders
        .iter()
        .find(|loader| loader.recognizes(&bytes))
        .ok_or(InstrumentLoadError::UnknownFormat)?;
    // the whole name: a dot in an Amiga file name doesn't mean an extension
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    return loader.load(&name, &bytes);
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use super::*;
    use crate::data::instrument::OctamedicSample;

    const RAW_HEADER: &[u8] = b"RAW!";

    // made up for the tests: "RAW!" then signed 8-bit samples
    struct RawLoader;

    impl InstrumentLoader for RawLoader {
        fn recognizes(&self, bytes: &[u8]) -> bool {
            return bytes.starts_with(RAW_HEADER);
        }

        fn load(
            &self,
            name: &str,
            bytes: &[u8],
        ) -> Result<OctamedicInstrument, InstrumentLoadError> {
            let samples = &bytes[RAW_HEADER.len()..];
            if samples.is_empty() {
                return Err(InstrumentLoadError::Malformed("No sound in it".to_owned()));
            }
            let mut instrument = OctamedicInstrument::new(name);
            instrument.sample = Some(Arc::new(OctamedicSample {
                data: samples.iter().map(|&byte| byte as i8).collect(),
                repeat: None,
            }));
            return Ok(instrument);
        }
    }

    fn temp_file(name: &str, bytes: &[u8]) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("octamedic-loader-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join(name);
        fs::write(&path, bytes).unwrap();
        return path;
    }

    fn raw(samples: &[u8]) -> Vec<u8> {
        return [RAW_HEADER, samples].concat();
    }

    #[test]
    fn picked_by_header_not_name() {
        let path = temp_file("BassDrum", &raw(&[0, 127, 128]));
        let instrument = load_instrument(&path, &[&RawLoader]);
        let _ = fs::remove_file(&path);
        let instrument = instrument.unwrap();
        assert_eq!(instrument.name, "BassDrum");
        assert_eq!(instrument.sample.unwrap().data, [0, 127, -128]);
    }

    #[test]
    fn keeps_the_whole_name() {
        let path = temp_file("st-01.bass", &raw(&[1]));
        let instrument = load_instrument(&path, &[&RawLoader]);
        let _ = fs::remove_file(&path);
        assert_eq!(instrument.unwrap().name, "st-01.bass");
    }

    #[test]
    fn misleading_extension_doesnt_matter() {
        let path = temp_file("looks-like.raw", b"but isn't");
        let result = load_instrument(&path, &[&RawLoader]);
        let _ = fs::remove_file(&path);
        assert!(matches!(result, Err(InstrumentLoadError::UnknownFormat)));
    }

    #[test]
    fn missing_file() {
        let path = std::env::temp_dir().join("octamedic-no-such-sample");
        let result = load_instrument(&path, &[&RawLoader]);
        assert!(matches!(result, Err(InstrumentLoadError::Unreadable(_))));
    }

    #[test]
    fn loader_errors_come_through() {
        let path = temp_file("empty", RAW_HEADER);
        let result = load_instrument(&path, &[&RawLoader]);
        let _ = fs::remove_file(&path);
        assert!(matches!(result, Err(InstrumentLoadError::Malformed(_))));
    }
}
