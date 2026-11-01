use std::{fmt::Display, fs, io, path::Path};

use crate::data::instrument::OctamedicInstrument;

/// Reads one audio file format into an instrument. Each format gets its own, and
/// [`load_instrument`] picks one by the file's extension.
pub trait InstrumentLoader {
    /// Lowercase, without the dot.
    fn extensions(&self) -> &[&str];

    /// `name` is what the instrument gets called, usually the file name.
    fn load(&self, name: &str, bytes: &[u8]) -> Result<OctamedicInstrument, InstrumentLoadError>;

    fn can_load(&self, path: &Path) -> bool {
        let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
            return false;
        };
        let extension = extension.to_lowercase();
        return self.extensions().contains(&extension.as_str());
    }
}

#[derive(Debug)]
pub enum InstrumentLoadError {
    Unreadable(io::Error),
    UnknownFormat,
    /// What's wrong with it, for the user.
    Malformed(String),
}

impl Display for InstrumentLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return match self {
            InstrumentLoadError::Unreadable(error) => write!(f, "{error}"),
            InstrumentLoadError::UnknownFormat => write!(f, "not a format that can be loaded"),
            InstrumentLoadError::Malformed(problem) => write!(f, "{problem}"),
        };
    }
}

impl std::error::Error for InstrumentLoadError {}

/// Loads `path` with the first of `loaders` that reads its extension, named after the file.
pub fn load_instrument(
    path: &Path,
    loaders: &[&dyn InstrumentLoader],
) -> Result<OctamedicInstrument, InstrumentLoadError> {
    let loader = loaders
        .iter()
        .find(|loader| loader.can_load(path))
        .ok_or(InstrumentLoadError::UnknownFormat)?;
    let bytes = fs::read(path).map_err(InstrumentLoadError::Unreadable)?;
    let name = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    return loader.load(&name, &bytes);
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};

    use super::*;
    use crate::data::instrument::OctamedicSample;

    // signed 8-bit samples with nothing around them
    struct RawLoader;

    impl InstrumentLoader for RawLoader {
        fn extensions(&self) -> &[&str] {
            return &["raw", "smp"];
        }

        fn load(
            &self,
            name: &str,
            bytes: &[u8],
        ) -> Result<OctamedicInstrument, InstrumentLoadError> {
            if bytes.is_empty() {
                return Err(InstrumentLoadError::Malformed("it's empty".to_owned()));
            }
            let mut instrument = OctamedicInstrument::new(name);
            instrument.sample = Some(Arc::new(OctamedicSample {
                data: bytes.iter().map(|&byte| byte as i8).collect(),
                repeat: None,
            }));
            return Ok(instrument);
        }
    }

    fn temp_file(name: &str, bytes: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("octamedic-{}-{name}", std::process::id()));
        fs::write(&path, bytes).unwrap();
        return path;
    }

    #[test]
    fn reads_by_extension() {
        assert!(RawLoader.can_load(Path::new("kick.raw")));
        assert!(RawLoader.can_load(Path::new("KICK.SMP")));
        assert!(!RawLoader.can_load(Path::new("kick.wav")));
        assert!(!RawLoader.can_load(Path::new("kick")));
    }

    #[test]
    fn loads_with_the_matching_loader() {
        let path = temp_file("bass.raw", &[0, 127, 128]);
        let instrument = load_instrument(&path, &[&RawLoader]);
        let _ = fs::remove_file(&path);
        let instrument = instrument.unwrap();
        assert!(instrument.name.ends_with("bass"), "{}", instrument.name);
        assert_eq!(instrument.sample.unwrap().data, [0, 127, -128]);
    }

    #[test]
    fn unknown_extension() {
        let result = load_instrument(Path::new("song.mp3"), &[&RawLoader]);
        assert!(matches!(result, Err(InstrumentLoadError::UnknownFormat)));
    }

    #[test]
    fn missing_file() {
        let path = std::env::temp_dir().join("octamedic-no-such-sample.raw");
        let result = load_instrument(&path, &[&RawLoader]);
        assert!(matches!(result, Err(InstrumentLoadError::Unreadable(_))));
    }

    #[test]
    fn loader_errors_come_through() {
        let path = temp_file("empty.raw", &[]);
        let result = load_instrument(&path, &[&RawLoader]);
        let _ = fs::remove_file(&path);
        assert!(matches!(result, Err(InstrumentLoadError::Malformed(_))));
    }
}
