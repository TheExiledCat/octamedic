use std::{fmt::Display, fs, io, path::Path};

pub mod decode;
pub mod flac;
pub mod mp3;
pub mod msh;
pub mod raw;
pub mod sample;
pub mod wav;

use crate::{
    data::instrument::{InstrumentKind, OctamedicInstrument},
    import::sample::{DecodedSample, ImportSettings},
};

pub enum Decoded {
    /// Still needs making into what Paula plays, see [`DecodedSample::to_instrument`].
    Sample(DecodedSample),
    Ready(InstrumentKind),
}

impl Decoded {
    pub fn into_sample(self) -> Option<DecodedSample> {
        return match self {
            Decoded::Sample(sample) => Some(sample),
            Decoded::Ready(_) => None,
        };
    }

    pub fn into_instrument(
        self,
        name: &str,
        settings: ImportSettings,
    ) -> Result<OctamedicInstrument, InstrumentLoadError> {
        return match self {
            Decoded::Sample(sample) => sample.to_instrument(name, settings),
            Decoded::Ready(kind) => {
                let mut instrument = OctamedicInstrument::new(name);
                if let InstrumentKind::Synth(synth) | InstrumentKind::Hybrid(synth, _) = &kind {
                    instrument.decay = synth.default_decay;
                }
                instrument.kind = kind;
                Ok(instrument)
            }
        };
    }
}

/// One audio file format. Picked by the file's header, not its extension, since old Amiga files
/// often have none.
pub trait InstrumentLoader {
    fn recognizes(&self, bytes: &[u8]) -> bool;

    fn decode(&self, bytes: &[u8]) -> Result<Decoded, InstrumentLoadError>;

    /// Whether to ask about channel and downsampling first, for samples.
    fn asks_for_settings(&self) -> bool {
        return true;
    }
}

#[derive(Debug)]
pub enum InstrumentLoadError {
    Unreadable(io::Error),
    UnknownFormat,
    /// What's wrong with it, for the user. Keep it short.
    Malformed(String),
}

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

pub fn find_loader<'a>(
    bytes: &[u8],
    loaders: &[&'a dyn InstrumentLoader],
) -> Option<&'a dyn InstrumentLoader> {
    return loaders
        .iter()
        .copied()
        .find(|loader| loader.recognizes(bytes));
}

/// The whole name: a dot in an Amiga file name doesn't mean an extension.
pub fn instrument_name(path: &Path) -> String {
    return path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
}

pub fn load_instrument(
    path: &Path,
    loaders: &[&dyn InstrumentLoader],
    settings: ImportSettings,
) -> Result<OctamedicInstrument, InstrumentLoadError> {
    let bytes = fs::read(path).map_err(InstrumentLoadError::Unreadable)?;
    let loader = find_loader(&bytes, loaders).ok_or(InstrumentLoadError::UnknownFormat)?;
    return loader
        .decode(&bytes)?
        .into_instrument(&instrument_name(path), settings);
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    const RAW_HEADER: &[u8] = b"RAW!";

    // made up for the tests: "RAW!" then signed 8-bit samples at 8000 Hz
    struct RawLoader;

    impl InstrumentLoader for RawLoader {
        fn recognizes(&self, bytes: &[u8]) -> bool {
            return bytes.starts_with(RAW_HEADER);
        }

        fn decode(&self, bytes: &[u8]) -> Result<Decoded, InstrumentLoadError> {
            let samples = &bytes[RAW_HEADER.len()..];
            return Ok(Decoded::Sample(DecodedSample {
                rate: 8000,
                bits: Some(8),
                channels: vec![
                    samples
                        .iter()
                        .map(|&byte| byte as i8 as f32 / 128.0)
                        .collect(),
                ],
                repeat: None,
            }));
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
        let instrument = load_instrument(&path, &[&RawLoader], ImportSettings::default());
        let _ = fs::remove_file(&path);
        let instrument = instrument.unwrap();
        assert_eq!(instrument.name, "BassDrum");
        assert_eq!(instrument.sample().unwrap().data, [0, 127, -128]);
    }

    #[test]
    fn keeps_the_whole_name() {
        let path = temp_file("st-01.bass", &raw(&[1]));
        let instrument = load_instrument(&path, &[&RawLoader], ImportSettings::default());
        let _ = fs::remove_file(&path);
        assert_eq!(instrument.unwrap().name, "st-01.bass");
    }

    #[test]
    fn misleading_extension_doesnt_matter() {
        let path = temp_file("looks-like.raw", b"but isn't");
        let result = load_instrument(&path, &[&RawLoader], ImportSettings::default());
        let _ = fs::remove_file(&path);
        assert!(matches!(result, Err(InstrumentLoadError::UnknownFormat)));
    }

    #[test]
    fn finds_the_loader_by_header() {
        assert!(find_loader(&raw(&[1]), &[&RawLoader]).is_some());
        assert!(find_loader(b"something else", &[&RawLoader]).is_none());
    }

    #[test]
    fn missing_file() {
        let path = std::env::temp_dir().join("octamedic-no-such-sample");
        let result = load_instrument(&path, &[&RawLoader], ImportSettings::default());
        assert!(matches!(result, Err(InstrumentLoadError::Unreadable(_))));
    }

    #[test]
    fn loader_errors_come_through() {
        let path = temp_file("empty", RAW_HEADER);
        let result = load_instrument(&path, &[&RawLoader], ImportSettings::default());
        let _ = fs::remove_file(&path);
        assert!(matches!(result, Err(InstrumentLoadError::Malformed(_))));
    }
}
