use std::{fs, io, path::Path};

use octamedic_core::amiga::clock::VideoStandard;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::app::{DEFAULT_VIDEO, files};

/// Persisted settings, saved in `config.toml` between runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    #[serde(
        default = "default_video",
        serialize_with = "serialize_video",
        deserialize_with = "deserialize_video"
    )]
    pub video: VideoStandard,
    #[serde(default, alias = "audio")]
    pub output: OutputSettings,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputSettings {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "name",
        alias = "output_device"
    )]
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
        "ntsc" => Ok(VideoStandard::Ntsc),
        _ => Ok(VideoStandard::Pal),
    };
}

impl Default for Settings {
    fn default() -> Self {
        return Self {
            video: DEFAULT_VIDEO,
            output: OutputSettings::default(),
        };
    }
}

impl Settings {
    /// Loads settings from `config.toml`, or creates it with defaults if it doesn't exist yet.
    /// Any error from reading or parsing is returned so startup can report it.
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
            let settings = Self::default();
            let _ = settings.save_to_path(path);
            return (settings, None);
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
            Ok(settings) => return (settings, None),
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
    fn default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.video, VideoStandard::Pal);
        assert_eq!(settings.output.device, None);
        assert!(!settings.output.mono);
    }

    #[test]
    fn round_trips_toml() {
        let folder =
            std::env::temp_dir().join(format!("octamedic-settings-{}", std::process::id()));
        let path = folder.join("config.toml");
        let settings = Settings {
            video: VideoStandard::Ntsc,
            output: OutputSettings {
                device: Some("Headphones".to_owned()),
                mono: true,
            },
        };
        settings.save_to_path(&path).unwrap();
        let (loaded, error) = Settings::load_from_path(&path);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(error, None);
        assert_eq!(loaded, settings);
    }

    #[test]
    fn parses_video_case_insensitive() {
        let ntsc: Settings = toml::from_str("video = \"NTSC\"").unwrap();
        assert_eq!(ntsc.video, VideoStandard::Ntsc);
        let pal: Settings = toml::from_str("video = \"pal\"").unwrap();
        assert_eq!(pal.video, VideoStandard::Pal);
        let unknown: Settings = toml::from_str("video = \"secam\"").unwrap();
        assert_eq!(unknown.video, VideoStandard::Pal);
    }

    #[test]
    fn parses_missing_fields_as_defaults() {
        let settings: Settings = toml::from_str("").unwrap();
        assert_eq!(settings, Settings::default());
    }

    #[test]
    fn output_table_alias() {
        let toml = r#"
            video = "pal"

            [audio]
            device = "USB DAC"
            mono = true
        "#;
        let settings: Settings = toml::from_str(toml).unwrap();
        assert_eq!(settings.output.device.as_deref(), Some("USB DAC"));
        assert!(settings.output.mono);
    }

    #[test]
    fn missing_file_creates_default() {
        let folder = std::env::temp_dir().join(format!("octamedic-missing-{}", std::process::id()));
        let path = folder.join("config.toml");
        let (loaded, error) = Settings::load_from_path(&path);
        let written = fs::read_to_string(&path).unwrap();
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(error, None);
        assert_eq!(loaded, Settings::default());
        assert!(written.contains("video = 'pal'") || written.contains("video = \"pal\""));
    }

    #[test]
    fn damaged_file_returns_error() {
        let folder = std::env::temp_dir().join(format!("octamedic-bad-{}", std::process::id()));
        let path = folder.join("config.toml");
        fs::create_dir_all(&folder).unwrap();
        fs::write(&path, "invalid [[[ toml").unwrap();
        let (loaded, error) = Settings::load_from_path(&path);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(loaded, Settings::default());
        assert!(error.is_some());
        assert!(error.unwrap().contains("Couldn't parse"));
    }
}
