use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::app::files::{self, Shortcut};

/// The file browser's current folder. Lives on `AppState`, since the FILES panel and screen both
/// use it.
pub struct Browser {
    path: PathBuf,
    folders: Vec<String>,
    files: Vec<FileEntry>,
    shortcuts: Vec<Shortcut>,
    selected_file: Option<String>,
    disk_space: Option<DiskSpace>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiskSpace {
    pub total: u64,
    pub available: u64,
}

impl DiskSpace {
    fn of(path: &Path) -> Option<Self> {
        return Some(Self {
            total: fs4::total_space(path).ok()?,
            available: fs4::available_space(path).ok()?,
        });
    }

    pub fn percent_full(&self) -> u64 {
        if self.total == 0 {
            return 0;
        }
        let used = self.total - self.available.min(self.total);
        return (used as u128 * 100 / self.total as u128) as u64;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub bytes: u64,
}

#[derive(Default)]
struct FolderContents {
    folders: Vec<String>,
    files: Vec<FileEntry>,
}

impl Browser {
    pub fn new() -> Self {
        let path = files::get_projects_directory_root().unwrap_or_else(|| PathBuf::from("."));
        let FolderContents { folders, files } = read_contents(&path).unwrap_or_default();
        return Self {
            disk_space: DiskSpace::of(&path),
            path,
            folders,
            files,
            shortcuts: files::get_shortcuts(),
            selected_file: None,
        };
    }

    pub fn path(&self) -> &Path {
        return &self.path;
    }

    pub fn folders(&self) -> &[String] {
        return &self.folders;
    }

    pub fn files(&self) -> &[FileEntry] {
        return &self.files;
    }

    pub fn shortcuts(&self) -> &[Shortcut] {
        return &self.shortcuts;
    }

    pub fn disk_space(&self) -> Option<DiskSpace> {
        return self.disk_space;
    }

    pub fn selected_file(&self) -> Option<&str> {
        return self.selected_file.as_deref();
    }

    pub fn select_file(&mut self, name: Option<String>) {
        self.selected_file = name;
    }

    pub fn open(&mut self, path: impl Into<PathBuf>) -> io::Result<()> {
        let path = path.into();
        let FolderContents { folders, files } = read_contents(&path)?;
        self.disk_space = DiskSpace::of(&path);
        self.path = path;
        self.folders = folders;
        self.files = files;
        self.selected_file = None;
        return Ok(());
    }

    pub fn add_shortcut(&mut self, shortcut: Shortcut) {
        self.shortcuts.push(shortcut);
    }

    /// Moves a shortcut one place, as the list it's shown in does.
    pub fn reorder_shortcut(&mut self, from: usize, to: usize) {
        if from.max(to) < self.shortcuts.len() {
            self.shortcuts.swap(from, to);
        }
    }

    pub fn save_shortcuts(&self) -> io::Result<()> {
        return files::shortcuts::save_user_shortcuts(&self.shortcuts);
    }

    /// Takes out a user shortcut that has no name or no path, like one that was just added and
    /// never filled in.
    pub fn discard_blank_shortcut(&mut self, index: usize) {
        let blank = self.shortcuts.get(index).is_some_and(|shortcut| {
            matches!(shortcut.kind, files::shortcuts::ShortcutKind::User { .. })
                && (shortcut.name.trim().is_empty() || shortcut.path.as_os_str().is_empty())
        });
        if blank {
            self.shortcuts.remove(index);
        }
    }

    pub fn remove_user_shortcut(&mut self, i: usize) {
        if i >= self.shortcuts.len() {
            return;
        }
        let Shortcut {
            kind: files::shortcuts::ShortcutKind::User { .. },
            ..
        } = self.shortcuts[i]
        else {
            return;
        };

        self.shortcuts.remove(i);
    }

    pub fn update_user_shortcut(&mut self, index: usize, name: impl Into<String>, path: PathBuf) {
        let Some(shortcut) = self.shortcuts.get_mut(index) else {
            return;
        };
        if !matches!(shortcut.kind, files::shortcuts::ShortcutKind::User { .. }) {
            return;
        }
        shortcut.name = name.into();
        shortcut.path = path;
    }
}

// fs::metadata follows links, so a link to a folder is listed as a folder
fn read_contents(path: &Path) -> io::Result<FolderContents> {
    let mut contents = FolderContents::default();
    for entry in fs::read_dir(path)?.filter_map(Result::ok) {
        let Ok(metadata) = fs::metadata(entry.path()) else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if metadata.is_dir() {
            contents.folders.push(name);
        } else {
            let bytes = metadata.len();
            contents.files.push(FileEntry { name, bytes });
        }
    }
    contents.folders.sort_by_key(|name| name.to_lowercase());
    contents.files.sort_by_key(|file| file.name.to_lowercase());
    return Ok(contents);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempFolder(PathBuf);

    impl TempFolder {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir()
                .join(format!("octamedic-browser-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap();
            return Self(path);
        }
    }

    impl Drop for TempFolder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn starts_in_projects() {
        let browser = Browser::new();
        if let Some(projects) = files::get_projects_directory_root() {
            assert_eq!(browser.path(), projects);
        }
    }

    #[test]
    fn folders_and_files_sorted() {
        let temp = TempFolder::new("sorted");
        for folder in ["songs", "Bass", "drums"] {
            fs::create_dir(temp.0.join(folder)).unwrap();
        }
        fs::write(temp.0.join("tune.mmd1"), [0; 1843]).unwrap();
        fs::write(temp.0.join("Intro.mmd0"), []).unwrap();
        let mut browser = Browser::new();
        browser.open(&temp.0).unwrap();
        assert_eq!(browser.path(), temp.0);
        assert_eq!(browser.folders(), ["Bass", "drums", "songs"]);
        let file = |name: &str, bytes| FileEntry {
            name: name.to_owned(),
            bytes,
        };
        assert_eq!(
            browser.files(),
            [file("Intro.mmd0", 0), file("tune.mmd1", 1843)]
        );
    }

    #[test]
    fn percent_full() {
        let space = |total, available| DiskSpace { total, available };
        assert_eq!(space(1000, 10).percent_full(), 99);
        assert_eq!(space(1000, 1000).percent_full(), 0);
        assert_eq!(space(0, 0).percent_full(), 0);
        assert_eq!(space(u64::MAX, 0).percent_full(), 100);
    }

    #[test]
    fn disk_space_of_open_folder() {
        let temp = TempFolder::new("disk");
        let mut browser = Browser::new();
        browser.open(&temp.0).unwrap();
        let space = browser.disk_space().unwrap();
        assert!(space.total > 0 && space.available <= space.total);
    }

    #[test]
    fn opening_clears_selection() {
        let temp = TempFolder::new("selection");
        let mut browser = Browser::new();
        browser.select_file(Some("tune.mmd1".to_owned()));
        browser.open(&temp.0).unwrap();
        assert_eq!(browser.selected_file(), None);
    }

    #[test]
    fn unreadable_folder_keeps_the_old_one() {
        let temp = TempFolder::new("stays");
        fs::create_dir(temp.0.join("inside")).unwrap();
        let mut browser = Browser::new();
        browser.open(&temp.0).unwrap();
        assert!(browser.open(temp.0.join("missing")).is_err());
        assert_eq!(browser.path(), temp.0);
        assert_eq!(browser.folders(), ["inside"]);
    }
}
