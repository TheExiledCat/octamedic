// the config, keybinds and file manager parts are for screens that don't exist yet
#![allow(dead_code)]

use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};

const FOLDER_ROOT_NAME: &str = "octamedic";
const CONFIG_FILE_NAME: &str = "config.toml";
pub const KEYBINDS_FILE_NAME: &str = "keybinds.toml";

/// Creates the config and projects folders when they don't exist yet.
pub fn initialize() -> io::Result<()> {
    fs::create_dir_all(get_config_directory_root().ok_or_else(|| missing("config"))?)?;
    fs::create_dir_all(get_projects_directory_root().ok_or_else(|| missing("music"))?)?;
    return Ok(());
}

/// Where modules are saved and opened from by default: the music folder's `octamedic`.
pub fn get_projects_directory_root() -> Option<PathBuf> {
    return dirs::audio_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join("Music")))
        .map(|dir| dir.join(FOLDER_ROOT_NAME));
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

/// Writes `default()` to `path` when there's no file there yet.
pub fn create_if_missing(path: &Path, default: impl FnOnce() -> String) -> io::Result<()> {
    if !path.exists() {
        fs::write(path, default())?;
    }
    return Ok(());
}

/// Opens `folder` in the OS file manager.
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

/// Uses Explorer on WSL, where xdg-open usually can't open folders.
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

/// The error for a folder the OS doesn't say where to put.
fn missing(folder: &str) -> io::Error {
    return io::Error::new(
        io::ErrorKind::NotFound,
        format!("couldn't find the {folder} folder"),
    );
}
