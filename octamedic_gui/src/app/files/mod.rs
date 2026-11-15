// the keybinds and file manager parts are for screens that don't exist yet
#![allow(dead_code)]

pub mod browser;
pub mod load_instrument;
pub mod shortcuts;

use std::{
    fmt::Display,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};

use mmd::mmd::{conversion::FromModule, error::MmdError, parser::OctamedMMDParser};
use octamedic_core::data::project::OctamedicProject;
pub use shortcuts::{Shortcut, get_shortcuts};

const FOLDER_ROOT_NAME: &str = "octamedic";
const CONFIG_FILE_NAME: &str = "config.toml";
const SHORTCUTS_FILE_NAME: &str = "shortcuts.toml";

pub const KEYBINDS_FILE_NAME: &str = "keybinds.toml";

pub fn initialize() -> io::Result<()> {
    fs::create_dir_all(get_config_directory_root().ok_or_else(|| missing("config"))?)?;
    fs::create_dir_all(get_projects_directory_root().ok_or_else(|| missing("music"))?)?;
    return Ok(());
}

// XDG user dirs aren't always configured on unix, so audio_dir() can come back empty
fn music_dir() -> Option<PathBuf> {
    return dirs::audio_dir().or_else(|| dirs::home_dir().map(|home| home.join("Music")));
}

pub fn get_projects_directory_root() -> Option<PathBuf> {
    return music_dir().map(|dir| dir.join(FOLDER_ROOT_NAME));
}

pub fn get_config_directory_root() -> Option<PathBuf> {
    return dirs::config_local_dir().map(|dir| dir.join(FOLDER_ROOT_NAME));
}

pub fn get_config_file_path() -> io::Result<PathBuf> {
    return config_file(CONFIG_FILE_NAME);
}

pub fn get_keybinds_file_path() -> io::Result<PathBuf> {
    return config_file(KEYBINDS_FILE_NAME);
}
pub fn get_shortcuts_file_path() -> io::Result<PathBuf> {
    return config_file(SHORTCUTS_FILE_NAME);
}

pub fn create_if_missing(path: &Path, default: impl FnOnce() -> String) -> io::Result<()> {
    if !path.exists() {
        fs::write(path, default())?;
    }
    return Ok(());
}

pub fn open_in_file_manager(folder: &Path) -> io::Result<()> {
    let mut command = file_manager_command(folder)?;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    // reap it so it doesn't stay a zombie
    thread::spawn(move || child.wait());
    return Ok(());
}

#[cfg(windows)]
fn file_manager_command(folder: &Path) -> io::Result<Command> {
    let mut command = Command::new("explorer");
    command.arg(folder);
    return Ok(command);
}

#[cfg(target_os = "macos")]
fn file_manager_command(folder: &Path) -> io::Result<Command> {
    let mut command = Command::new("open");
    command.arg(folder);
    return Ok(command);
}

// xdg-open usually can't open folders on WSL
#[cfg(not(any(windows, target_os = "macos")))]
fn file_manager_command(folder: &Path) -> io::Result<Command> {
    if std::env::var_os("WSL_DISTRO_NAME").is_none() {
        let mut command = Command::new("xdg-open");
        command.arg(folder);
        return Ok(command);
    }
    let output = Command::new("wslpath").arg("-w").arg(folder).output()?;
    let windows_path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let mut command = Command::new("explorer.exe");
    command.arg(windows_path);
    return Ok(command);
}

fn config_file(name: &str) -> io::Result<PathBuf> {
    let root = get_config_directory_root().ok_or_else(|| missing("config"))?;
    return Ok(root.join(name));
}

#[derive(Debug)]
pub enum ModuleLoadError {
    Unreadable(MmdError),
    NoModule,
}

// short, it's shown in a notification or a dialog
impl Display for ModuleLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            // too short to be a module
            ModuleLoadError::Unreadable(MmdError::Io(error))
                if error.kind() == io::ErrorKind::UnexpectedEof =>
            {
                "Not a song"
            }
            ModuleLoadError::Unreadable(MmdError::Io(_)) => "Can't read file",
            ModuleLoadError::Unreadable(MmdError::UnknownFormat(_)) | ModuleLoadError::NoModule => {
                "Not a song"
            }
            ModuleLoadError::Unreadable(MmdError::Unsupported(_)) => "Song type not supported",
            ModuleLoadError::Unreadable(
                MmdError::MissingPointer(_) | MmdError::UnknownInstrumentType(_),
            ) => "Song is damaged",
        };
        return write!(f, "{message}");
    }
}

// a file can hold several modules, only the first is used
pub fn load_module(path: &Path) -> Result<OctamedicProject, ModuleLoadError> {
    let modules = OctamedMMDParser::new()
        .parse_file(path)
        .map_err(ModuleLoadError::Unreadable)?;
    let module = modules.first().ok_or(ModuleLoadError::NoModule)?;
    return Ok(OctamedicProject::from_module(module));
}

fn missing(folder: &str) -> io::Error {
    return io::Error::new(
        io::ErrorKind::NotFound,
        format!("couldn't find the {folder} folder"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, bytes: &[u8]) -> PathBuf {
        let path = std::env::temp_dir().join(format!("octamedic-{name}-{}", std::process::id()));
        fs::write(&path, bytes).unwrap();
        return path;
    }

    #[test]
    fn missing_module_file() {
        let path = std::env::temp_dir().join("octamedic-no-such-module.mmd0");
        assert!(matches!(
            load_module(&path),
            Err(ModuleLoadError::Unreadable(_))
        ));
    }

    #[test]
    fn not_a_module() {
        let path = temp_file("not-a-module", b"just some text, not a module at all");
        let result = load_module(&path);
        let _ = fs::remove_file(&path);
        assert_eq!(result.err().unwrap().to_string(), "Not a song");
    }

    #[test]
    fn missing_file_message() {
        let path = std::env::temp_dir().join("octamedic-no-such-module.mmd0");
        assert_eq!(
            load_module(&path).err().unwrap().to_string(),
            "Can't read file"
        );
    }

    #[test]
    fn shortcuts_are_folders() {
        let shortcuts = get_shortcuts();
        assert!(!shortcuts.is_empty());
        for shortcut in shortcuts
            .iter()
            .filter(|shortcut| shortcut.kind == shortcuts::ShortcutKind::System)
        {
            assert!(!shortcut.name.is_empty());
            assert!(shortcut.path.is_dir(), "{shortcut:?}");
        }
    }

    #[test]
    fn projects_first() {
        if let Some(projects) = get_projects_directory_root().filter(|path| path.is_dir()) {
            assert_eq!(get_shortcuts()[0].path, projects);
        }
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    #[test]
    fn root_and_mounts() {
        let shortcuts = get_shortcuts();
        assert!(
            shortcuts
                .iter()
                .any(|shortcut| shortcut.path == Path::new("/"))
        );
        assert!(
            !shortcuts
                .iter()
                .any(|shortcut| shortcut.name.starts_with("wsl"))
        );
    }
}
