use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::app::files::{self, get_projects_directory_root, music_dir};

/// Where the file browser can jump to, like the Amiga's assigns and devices list.
#[derive(Clone, Debug, PartialEq)]
pub struct Shortcut {
    pub name: String,
    pub path: PathBuf,
    pub kind: ShortcutKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShortcutKind {
    System,
    User { priority: usize },
}

/// The shortcuts the user added, saved in `shortcuts.toml` between runs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ShortcutsConfig {
    #[serde(default, rename = "shortcut", skip_serializing_if = "Vec::is_empty")]
    pub shortcuts: Vec<UserShortcut>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserShortcut {
    pub name: String,
    pub path: PathBuf,
    // lower comes first
    #[serde(default)]
    pub priority: usize,
}

impl ShortcutsConfig {
    /// Loads `shortcuts.toml`, or creates it empty if it doesn't exist yet. Returns any error
    /// from reading or parsing so it can be reported.
    pub fn load() -> (Self, Option<String>) {
        return match files::get_shortcuts_file_path() {
            Ok(path) => Self::load_from_path(&path),
            Err(error) => (
                Self::default(),
                Some(format!("Config folder missing: {error}")),
            ),
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

    /// Inserts the saved shortcuts whose folder still exists into `list`, each at the index its
    /// priority names, counting the system shortcuts. A shortcut past the end goes last, and
    /// ones sharing an index keep their order from the file.
    fn insert_into(self, list: &mut Vec<Shortcut>) {
        let mut saved: Vec<UserShortcut> = self
            .shortcuts
            .into_iter()
            .filter(|saved| saved.path.is_dir())
            .collect();
        saved.sort_by_key(|saved| saved.priority);
        let mut next_free = 0;
        for saved in saved {
            let index = saved.priority.max(next_free).min(list.len());
            list.insert(
                index,
                Shortcut {
                    name: saved.name,
                    path: saved.path,
                    kind: ShortcutKind::User {
                        priority: saved.priority,
                    },
                },
            );
            next_free = index + 1;
        }
    }
}

pub fn get_shortcuts() -> Vec<Shortcut> {
    let places = [
        ("Projects", get_projects_directory_root()),
        ("Home", dirs::home_dir()),
        ("Desktop", dirs::desktop_dir()),
        ("Documents", dirs::document_dir()),
        ("Downloads", dirs::download_dir()),
        ("Music", music_dir()),
        ("Pictures", dirs::picture_dir()),
    ];
    let places = places.into_iter().filter_map(|(name, path)| {
        let path = path.filter(|path| path.is_dir())?;
        return Some(Shortcut {
            name: name.to_owned(),
            path,
            kind: ShortcutKind::System,
        });
    });
    let mut list: Vec<Shortcut> = places.chain(drives()).collect();
    let (saved, _) = ShortcutsConfig::load();
    saved.insert_into(&mut list);
    return list;
}

#[cfg(windows)]
fn drives() -> Vec<Shortcut> {
    return ('A'..='Z')
        .map(|letter| Shortcut {
            name: format!("{letter}:"),
            path: PathBuf::from(format!("{letter}:\\")),
            kind: ShortcutKind::System,
        })
        .filter(|drive| drive.path.is_dir())
        .collect();
}

#[cfg(target_os = "macos")]
fn drives() -> Vec<Shortcut> {
    return subfolders(Path::new("/Volumes"));
}

// where removable drives and WSL's windows drives get mounted
#[cfg(not(any(windows, target_os = "macos")))]
fn drives() -> Vec<Shortcut> {
    let root = Shortcut {
        name: "/".to_owned(),
        path: PathBuf::from("/"),
        kind: ShortcutKind::System,
    };
    let user = std::env::var("USER").unwrap_or_default();
    let mounts = [
        PathBuf::from("/mnt"),
        Path::new("/media").join(&user),
        Path::new("/run/media").join(&user),
    ];
    let mounted = mounts
        .iter()
        .flat_map(|folder| subfolders(folder))
        // WSL's own mounts, not drives
        .filter(|drive| !drive.name.starts_with("wsl"));
    return [root].into_iter().chain(mounted).collect();
}

#[cfg(not(windows))]
fn subfolders(folder: &Path) -> Vec<Shortcut> {
    let Ok(entries) = fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut found: Vec<Shortcut> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| Shortcut {
            name: entry.file_name().to_string_lossy().into_owned(),
            path: entry.path(),
            kind: ShortcutKind::System,
        })
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    return found;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_toml() {
        let folder =
            std::env::temp_dir().join(format!("octamedic-shortcuts-{}", std::process::id()));
        let path = folder.join("shortcuts.toml");
        let config = ShortcutsConfig {
            shortcuts: vec![UserShortcut {
                name: "Samples".to_owned(),
                path: folder.clone(),
                priority: 2,
            }],
        };
        config.save_to_path(&path).unwrap();
        let (loaded, error) = ShortcutsConfig::load_from_path(&path);
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(error, None);
        assert_eq!(loaded, config);
    }

    #[test]
    fn missing_file_is_created_empty() {
        let folder =
            std::env::temp_dir().join(format!("octamedic-shortcuts-new-{}", std::process::id()));
        let path = folder.join("shortcuts.toml");
        let (loaded, error) = ShortcutsConfig::load_from_path(&path);
        let created = path.exists();
        let _ = fs::remove_dir_all(&folder);
        assert_eq!(error, None);
        assert!(created);
        assert_eq!(loaded, ShortcutsConfig::default());
    }

    #[test]
    fn damaged_file_is_kept_aside() {
        let folder =
            std::env::temp_dir().join(format!("octamedic-shortcuts-bad-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let path = folder.join("shortcuts.toml");
        fs::write(&path, "shortcut = 3").unwrap();
        let (loaded, error) = ShortcutsConfig::load_from_path(&path);
        let kept = path.with_extension("toml.bak").exists();
        let _ = fs::remove_dir_all(&folder);
        assert!(error.is_some());
        assert!(kept);
        assert_eq!(loaded, ShortcutsConfig::default());
    }

    fn system(name: &str) -> Shortcut {
        return Shortcut {
            name: name.to_owned(),
            path: std::env::temp_dir(),
            kind: ShortcutKind::System,
        };
    }

    fn saved(name: &str, priority: usize) -> UserShortcut {
        return UserShortcut {
            name: name.to_owned(),
            path: std::env::temp_dir(),
            priority,
        };
    }

    fn names_after_inserting(saved: Vec<UserShortcut>) -> Vec<String> {
        let mut list = vec![system("s0"), system("s1"), system("s2")];
        ShortcutsConfig { shortcuts: saved }.insert_into(&mut list);
        return list.into_iter().map(|shortcut| shortcut.name).collect();
    }

    #[test]
    fn priority_is_the_index_among_all_shortcuts() {
        let names =
            names_after_inserting(vec![saved("late", 4), saved("early", 0), saved("mid", 2)]);
        assert_eq!(
            names,
            ["early", "s0", "mid", "s1", "late", "s2"].map(String::from)
        );
    }

    #[test]
    fn priority_past_the_end_goes_last() {
        let names = names_after_inserting(vec![saved("far", 99)]);
        assert_eq!(names, ["s0", "s1", "s2", "far"].map(String::from));
    }

    #[test]
    fn shared_priority_keeps_file_order() {
        let names = names_after_inserting(vec![saved("a", 1), saved("b", 1)]);
        assert_eq!(names, ["s0", "a", "b", "s1", "s2"].map(String::from));
    }

    #[test]
    fn missing_folders_are_skipped() {
        let mut gone = saved("gone", 0);
        gone.path = gone.path.join("octamedic-no-such-folder");
        let names = names_after_inserting(vec![gone]);
        assert_eq!(names, ["s0", "s1", "s2"].map(String::from));
    }
}
