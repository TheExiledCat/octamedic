use std::{
    collections::BTreeMap,
    fmt, fs, io,
    path::{Path, PathBuf},
    str::FromStr,
};

use octamedic_core::amiga::clock::VideoStandard;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::app::{DEFAULT_VIDEO, files};

/// Persisted configuration, saved in `config.toml` between runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    #[serde(
        default = "default_video",
        serialize_with = "serialize_video",
        deserialize_with = "deserialize_video"
    )]
    pub video: VideoStandard,
    // before the tables, toml can't write a plain value after one
    #[serde(default)]
    pub graphics: GraphicsBackend,
    #[serde(default)]
    pub input: InputConfig,
    #[serde(default)]
    pub output: OutputConfig,
    /// The screen filters in the order they run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filters: Vec<FilterConfig>,
}

/// One screen filter: which it is, whether it's on, and its settings by name.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FilterConfig {
    pub kind: String,
    #[serde(default = "switched_on")]
    pub enabled: bool,
    #[serde(flatten)]
    pub settings: BTreeMap<String, f32>,
}

fn switched_on() -> bool {
    return true;
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    #[serde(default)]
    pub mono: bool,
}

/// What wgpu draws with. Auto tries Vulkan, Metal and DX12, then GL. wgpu 0.16's DX11 backend is
/// unfinished (its render pipelines are `todo!()`), so it isn't offered.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum GraphicsBackend {
    #[default]
    Auto,
    Vulkan,
    Metal,
    Dx12,
    Gl,
}

impl GraphicsBackend {
    const ALL: [Self; 5] = [Self::Auto, Self::Vulkan, Self::Metal, Self::Dx12, Self::Gl];

    pub fn name(self) -> &'static str {
        return match self {
            Self::Auto => "auto",
            Self::Vulkan => "vulkan",
            Self::Metal => "metal",
            Self::Dx12 => "dx12",
            Self::Gl => "gl",
        };
    }
}

impl FromStr for GraphicsBackend {
    type Err = String;

    fn from_str(name: &str) -> Result<Self, String> {
        let name = name.trim().to_ascii_lowercase();
        return Self::ALL
            .into_iter()
            .find(|backend| backend.name() == name)
            .ok_or_else(|| {
                format!(
                    "unknown graphics backend \"{name}\", expected auto, vulkan, metal, dx12 or gl"
                )
            });
    }
}

impl TryFrom<String> for GraphicsBackend {
    type Error = String;

    fn try_from(name: String) -> Result<Self, String> {
        return name.parse();
    }
}

impl From<GraphicsBackend> for String {
    fn from(backend: GraphicsBackend) -> Self {
        return backend.name().to_owned();
    }
}

impl fmt::Display for GraphicsBackend {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        return f.write_str(self.name());
    }
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
            graphics: GraphicsBackend::Auto,
            input: InputConfig::default(),
            output: OutputConfig::default(),
            filters: Vec::new(),
        };
    }
}

/// The outcome of loading configuration at startup.
#[derive(Clone, Debug, PartialEq)]
pub struct LoadedConfig {
    pub config: Config,
    pub path: Option<PathBuf>,
    pub error: Option<String>,
}

impl Config {
    /// Loads configuration from `config.toml`, or creates it with defaults if it doesn't exist
    /// yet. Returns the config, its path if found, and any error from reading or parsing so
    /// startup can report it.
    pub fn load() -> LoadedConfig {
        let path = match files::get_config_file_path() {
            Ok(path) => path,
            Err(error) => {
                return LoadedConfig {
                    config: Self::default(),
                    path: None,
                    error: Some(format!("Config folder missing: {error}")),
                };
            }
        };
        let (config, error) = Self::load_from_path(&path);
        return LoadedConfig {
            config,
            path: Some(path),
            error,
        };
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
                // preserve the damaged file so the user can inspect or recover it
                let bak = path.with_extension("toml.bak");
                let _ = fs::rename(path, &bak);
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
        assert_eq!(config.input.device, None);
        assert_eq!(config.output.device, None);
        assert!(!config.output.mono);
    }

    #[test]
    fn round_trips_toml() {
        let folder = std::env::temp_dir().join(format!("octamedic-config-{}", std::process::id()));
        let path = folder.join("config.toml");
        let config = Config {
            video: VideoStandard::Ntsc,
            graphics: GraphicsBackend::Dx12,
            input: InputConfig {
                device: Some("Microphone".to_owned()),
            },
            output: OutputConfig {
                device: Some("Headphones".to_owned()),
                mono: true,
            },
            filters: vec![FilterConfig {
                kind: "vignette".to_owned(),
                enabled: false,
                settings: BTreeMap::from([("strength".to_owned(), 0.5), ("size".to_owned(), 0.25)]),
            }],
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
    fn parses_graphics_backend_case_insensitive() {
        let config: Config = toml::from_str("graphics = \"DX12\"").unwrap();
        assert_eq!(config.graphics, GraphicsBackend::Dx12);
        assert_eq!(" Vulkan ".parse(), Ok(GraphicsBackend::Vulkan));
    }

    #[test]
    fn unknown_graphics_backend_fails_parsing() {
        let result: Result<Config, _> = toml::from_str("graphics = \"directx\"");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unknown graphics backend")
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
        assert!(written.contains("video = 'pal'"));
        assert!(written.contains("graphics = 'auto'"));
    }

    #[test]
    fn damaged_file_returns_error() {
        let folder =
            std::env::temp_dir().join(format!("octamedic-bad-config-{}", std::process::id()));
        let path = folder.join("config.toml");
        let bak = folder.join("config.toml.bak");
        fs::create_dir_all(&folder).unwrap();
        fs::write(&path, "invalid [[[ toml").unwrap();
        let (loaded, error) = Config::load_from_path(&path);
        assert_eq!(loaded, Config::default());
        assert!(error.is_some());
        assert!(error.unwrap().contains("Couldn't parse"));
        assert!(bak.exists());
        assert_eq!(fs::read_to_string(&bak).unwrap(), "invalid [[[ toml");
        let _ = fs::remove_dir_all(&folder);
    }
}
