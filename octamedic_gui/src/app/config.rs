use std::{fs, io, path::Path};

use octamedic_core::amiga::clock::VideoStandard;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::app::{DEFAULT_VIDEO, files};

/// Persisted configuration, saved in `config.toml` between runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    #[serde(
        default = "default_video",
        serialize_with = "serialize_video",
        deserialize_with = "deserialize_video"
    )]
    pub video: VideoStandard,
    #[serde(default)]
    pub output: OutputConfig,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(default)]
    pub mono: bool,
}

fn default_video() -> VideoStandard {
    return DEFAULT_VIDEO;
}

fn serialize_video<S>(video: &VideoStandard, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let name = match video {
        VideoStandard::Pal => "pal",
        VideoStandard::Ntsc => "ntsc",
    };
    return serializer.serialize_str(name);
}

fn deserialize_video<'de, D>(deserializer: D) -> Result<VideoStandard, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    return match s.to_ascii_lowercase().as_str() {
        "pal" => Ok(VideoStandard::Pal),
        "ntsc" => Ok(VideoStandard::Ntsc),
        other => Err(serde::de::Error::custom(format!(
            "unknown video standard \"{other}\", expected \"pal\" or \"ntsc\""
        ))),
    };
}

impl Default for Config {
    fn default() -> Self {
        return Self {
            video: DEFAULT_VIDEO,
            output: OutputConfig::default(),
        };
    }
}

impl Config {
    /// Loads configuration from `config.toml`, or creates it with defaults if it doesn't exist
    /// yet. Any error from reading or parsing is returned so startup can report it.
    pub fn load() -> (Self, Option<String>) {
        let path = match files::get_config_file_path() {
            Ok(path) => path,
            Err(error) => {
                return (
                    Self::default(),
                    Some(format!("Config folder missing: {error}")),
                );
            }
        };
        return Self::load_from_path(&path);
    }

    pub fn load_from_path(path: &Path) -> (Self, Option<String>) {
        if !path.exists() {
            let config = Self::default();
            let _ = config.save_to_path(path);
            return (config, None);
        }
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) => {
                return (
                    Self::default(),
                    Some(format!("Couldn't read {}: {error}", path.display())),
                );
            }
        };
        match toml::from_str(&content) {
            Ok(config) => return (config, None),
            Err(error) => {
                return (
                    Self::default(),
                    Some(format!("Couldn't parse {}: {error}", path.display())),
                );
            }
        }
    }

    pub fn save_to_path(&self, path: &Path) -> io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }
        let toml_string = toml::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        return fs::write(path, toml_string);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = Config::default();
        assert_eq!(config.video, VideoStandard::Pal);
        assert_eq!(config.output.device, None);
        assert!(!config.output.mono);
    }

    #[test]
    fn round_trips_toml() {
        let folder = std::env::temp_dir().join(format!("octamedic-config-{}", std::process::id()));
        let path = folder.join("config.toml");
        let config = Config {
            video: VideoStandard::Ntsc,
            output: OutputConfig {
                device: Some("Headphones".to_owned()),
                mono: true,
            },
        };
        config.save_to_path(&path).unwrap();
        let (loaded, error) = Config::load_from_path(&path);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(error, None);
        assert_eq!(loaded, config);
    }

    #[test]
    fn parses_video_case_insensitive() {
        let ntsc: Config = toml::from_str("video = \"NTSC\"").unwrap();
        assert_eq!(ntsc.video, VideoStandard::Ntsc);
        let pal: Config = toml::from_str("video = \"pal\"").unwrap();
        assert_eq!(pal.video, VideoStandard::Pal);
    }

    #[test]
    fn unknown_video_standard_fails_parsing() {
        let result: Result<Config, _> = toml::from_str("video = \"ntcs\"");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unknown video standard")
        );
    }

    #[test]
    fn parses_missing_fields_as_defaults() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn missing_file_creates_default() {
        let folder =
            std::env::temp_dir().join(format!("octamedic-missing-config-{}", std::process::id()));
        let path = folder.join("config.toml");
        let (loaded, error) = Config::load_from_path(&path);
        let written = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(error, None);
        assert_eq!(loaded, Config::default());
        assert!(written.contains("video = 'pal'") || written.contains("video = \"pal\""));
    }

    #[test]
    fn damaged_file_returns_error() {
        let folder =
            std::env::temp_dir().join(format!("octamedic-bad-config-{}", std::process::id()));
        let path = folder.join("config.toml");
        fs::create_dir_all(&folder).unwrap();
        fs::write(&path, "invalid [[[ toml").unwrap();
        let (loaded, error) = Config::load_from_path(&path);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(loaded, Config::default());
        assert!(error.is_some());
        assert!(error.unwrap().contains("Couldn't parse"));
    }
}
