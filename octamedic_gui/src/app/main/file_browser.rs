use std::path::{Path, PathBuf};

use bytesize::ByteSize;
use rustuition::{
    event::{MouseButton, MouseEvent, PointerEvent},
    gadget::Gadget,
    painter::{CHAR_HEIGHT, CHAR_WIDTH, Painter, Pen, text_width},
    rect::Rect,
};

use crate::{
    app::{
        files::browser::Browser,
        main::list_scroller::{ListScroller, SCROLLER_WIDTH},
        state::AppState,
    },
    framework::event::Event,
};

// from the screen's top left. the lists take the height that's left, so they still fit when NTSC
// cuts the screen short
const DISK_USAGE: [i32; 2] = [8, 3];
const LISTS_TOP: i32 = 11;
const LISTS_BOTTOM_MARGIN: i32 = 3;
// x, width
const FILE_LIST: [i32; 2] = [6, 428];
const SHORTCUT_LIST: [i32; 2] = [454, 164];
// from the list's top left
const ENTRY_OFFSET: [i32; 2] = [3, 1];
const SIZE_RIGHT_MARGIN: i32 = 2;
const PARENT_ENTRY: &str = "(dir) / Parent Dir";
const FOLDER_PREFIX: &str = "(dir) ";
// so a long list's handle can still be grabbed

// index of the file list in the two lists, the shortcuts are the other one
const FILES: usize = 0;

/// The FILES screen: the current folder on the left, shortcuts on the right.
pub struct FileBrowser {
    scrollers: [ListScroller; 2],
    scrolled_folder: Option<PathBuf>,
    // from the last render, for clicks
    list_areas: [Rect; 2],
}

impl FileBrowser {
    pub fn new() -> Self {
        return Self {
            scrollers: [ListScroller::new(), ListScroller::new()],
            scrolled_folder: None,
            list_areas: [Rect::default(); 2],
        };
    }

    fn entry_under(&self, pos: [i32; 2]) -> Option<(usize, usize)> {
        let list = self.list_areas.iter().position(|area| area.contains(pos))?;
        let line = (pos[1] - self.list_areas[list].y - ENTRY_OFFSET[1]).max(0) / CHAR_HEIGHT;
        return Some((
            list,
            self.scrollers[list].first_visible_entry + line as usize,
        ));
    }

    fn click(&self, state: &mut AppState, pos: [i32; 2]) -> bool {
        let Some((list, index)) = self.entry_under(pos) else {
            return false;
        };
        let lists = list_entries(&state.browser);
        if let Some(entry) = lists[list].get(index) {
            entry.target.clone().click(state);
        }
        return true;
    }
}

impl Gadget<AppState, Event> for FileBrowser {
    fn update(&mut self, state: &mut AppState) {
        let browser = &state.browser;
        if self.scrolled_folder.as_deref() != Some(browser.path()) {
            self.scrollers[FILES].first_visible_entry = 0;
            self.scrolled_folder = Some(browser.path().to_path_buf());
        }
        let totals = [file_entry_count(browser), browser.shortcuts().len()];
        for (scroller, total) in self.scrollers.iter_mut().zip(totals) {
            scroller.update(total);
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        if self
            .scrollers
            .iter_mut()
            .any(|scroller| scroller.handle_event(state, event))
        {
            return true;
        }
        return match event.mouse() {
            Some(MouseEvent::Down {
                pos,
                button: MouseButton::Left,
            }) => self.click(state, pos),
            _ => false,
        };
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.fill(area, Pen::FILL);
        painter.frame(area, 1);
        if let Some(usage) = disk_usage(&state.browser) {
            let [x, y] = DISK_USAGE;
            painter.text(area.x + x, area.y + y, &usage, Pen::BACKGROUND);
        }
        let height = area.h - LISTS_TOP - LISTS_BOTTOM_MARGIN;
        let lists = [FILE_LIST, SHORTCUT_LIST]
            .into_iter()
            .zip(list_entries(&state.browser))
            .zip(&mut self.list_areas);
        for ((([x, w], entries), list_area), scroller) in lists.zip(&mut self.scrollers) {
            let list = Rect::new(x, LISTS_TOP, w, height).within(area);
            *list_area = list;
            let inside = painter.frame(list, 1);
            scroller.lines_that_fit = (inside.h / CHAR_HEIGHT) as usize;
            let shown = &entries[scroller.first_visible_entry.min(entries.len())..];
            render_entries(painter, list, inside, shown);
            let scroller_area = Rect::new(list.right(), list.y, SCROLLER_WIDTH, height);
            scroller.render(state, scroller_area, entries.len(), painter);
        }
    }
}

// OctaMED's wording, with the free space in KiB, MiB and so on rather than bytes
fn disk_usage(browser: &Browser) -> Option<String> {
    let space = browser.disk_space()?;
    let free = ByteSize(space.available);
    return Some(format!(
        "Disk {:>3} % full ({free} free)",
        space.percent_full()
    ));
}

fn has_parent(browser: &Browser) -> bool {
    return browser.path().parent().is_some();
}

fn file_entry_count(browser: &Browser) -> usize {
    return has_parent(browser) as usize + browser.folders().len() + browser.files().len();
}

#[derive(Debug, PartialEq, Eq)]
struct Entry {
    text: String,
    target: ClickTarget,
}

impl Entry {
    fn bytes(&self) -> Option<u64> {
        return match self.target {
            ClickTarget::File { bytes, .. } => Some(bytes),
            _ => None,
        };
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ClickTarget {
    Parent,
    Folder(String),
    File { name: String, bytes: u64 },
    Shortcut(PathBuf),
}

impl ClickTarget {
    fn click(self, state: &mut AppState) {
        let current = state.browser.path().to_path_buf();
        match self {
            ClickTarget::Parent => open_or_notify(state, current.parent().unwrap_or(&current)),
            ClickTarget::Folder(name) => open_or_notify(state, &current.join(name)),
            ClickTarget::File { name, .. } => state.browser.select_file(Some(name)),
            ClickTarget::Shortcut(path) => open_or_notify(state, &path),
        }
    }
}

fn open_or_notify(state: &mut AppState, path: &Path) {
    if state.browser.open(path).is_err() {
        state.notify("Can't open folder");
    }
}

fn list_entries(browser: &Browser) -> [Vec<Entry>; 2] {
    let shortcuts = browser.shortcuts().iter().map(|shortcut| Entry {
        text: shortcut.name.clone(),
        target: ClickTarget::Shortcut(shortcut.path.clone()),
    });
    return [file_entries(browser), shortcuts.collect()];
}

fn file_entries(browser: &Browser) -> Vec<Entry> {
    let parent = has_parent(browser).then(|| Entry {
        text: PARENT_ENTRY.to_owned(),
        target: ClickTarget::Parent,
    });
    let folders = browser.folders().iter().map(|folder| Entry {
        text: format!("{FOLDER_PREFIX}{folder}"),
        target: ClickTarget::Folder(folder.clone()),
    });
    let files = browser.files().iter().map(|file| Entry {
        text: file.name.clone(),
        target: ClickTarget::File {
            name: file.name.clone(),
            bytes: file.bytes,
        },
    });
    return parent.into_iter().chain(folders).chain(files).collect();
}

// a long name is cut off a char before its size
fn render_entries(painter: &mut Painter, list: Rect, inside: Rect, entries: &[Entry]) {
    let [x, y] = ENTRY_OFFSET;
    for (line, entry) in entries.iter().enumerate() {
        let top = list.y + y + line as i32 * CHAR_HEIGHT;
        if top >= list.bottom() {
            break;
        }
        let mut name_right = inside.right();
        if let Some(bytes) = entry.bytes() {
            let bytes = bytes.to_string();
            let bytes_x = list.right() - SIZE_RIGHT_MARGIN - text_width(&bytes);
            painter.clipped(inside, |painter| {
                painter.text(bytes_x, top, &bytes, Pen::SHADOW)
            });
            name_right = bytes_x - CHAR_WIDTH;
        }
        let name_area = Rect::new(inside.x, inside.y, name_right - inside.x, inside.h);
        painter.clipped(name_area.intersection(inside), |painter| {
            painter.text(list.x + x, top, &entry.text, Pen::SHADOW);
        });
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::target::Framebuffer;

    use super::*;

    const SCREEN_AREA: Rect = Rect::new(0, 77, 640, 120);
    const SHORTCUTS: usize = 1;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    fn temp_folder_with(name: &str, folder_count: usize) -> PathBuf {
        let path = std::env::temp_dir().join(format!("octamedic-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        for index in 0..folder_count {
            fs::create_dir_all(path.join(index.to_string())).unwrap();
        }
        return path;
    }

    #[test]
    fn parent_folders_then_files() {
        let temp = temp_folder_with("entries", 0);
        fs::create_dir_all(temp.join("songs")).unwrap();
        fs::create_dir_all(temp.join("Bass")).unwrap();
        fs::write(temp.join("tune.mmd1"), [0; 1843]).unwrap();
        let mut browser = Browser::new();
        browser.open(&temp).unwrap();
        let entries = file_entries(&browser);
        let _ = fs::remove_dir_all(&temp);
        let texts: Vec<&str> = entries.iter().map(|entry| entry.text.as_str()).collect();
        assert_eq!(
            texts,
            [
                "(dir) / Parent Dir",
                "(dir) Bass",
                "(dir) songs",
                "tune.mmd1"
            ]
        );
        let bytes: Vec<Option<u64>> = entries.iter().map(Entry::bytes).collect();
        assert_eq!(bytes, [None, None, None, Some(1843)]);
    }

    #[cfg(not(windows))]
    #[test]
    fn no_parent_at_root() {
        let mut browser = Browser::new();
        browser.open("/").unwrap();
        let entries = file_entries(&browser);
        assert!(entries.iter().all(|entry| entry.text != PARENT_ENTRY));
        let folders = entries.iter().filter(|entry| entry.bytes().is_none());
        assert!(
            folders
                .into_iter()
                .all(|entry| entry.text.starts_with(FOLDER_PREFIX))
        );
    }

    #[test]
    fn disk_usage_text() {
        let browser = Browser::new();
        let usage = disk_usage(&browser).unwrap();
        assert!(usage.starts_with("Disk "), "{usage}");
        assert!(
            usage.contains(" % full (") && usage.ends_with("iB free)"),
            "{usage}"
        );
    }

    #[test]
    fn new_folder_scrolls_to_top() {
        let mut state = state();
        let [first, second] = ["scroll-a", "scroll-b"].map(|name| temp_folder_with(name, 20));
        let mut browser = FileBrowser::new();
        state.browser.open(&first).unwrap();
        browser.update(&mut state);
        browser.scrollers[FILES].first_visible_entry = 3;
        browser.update(&mut state);
        assert_eq!(browser.scrollers[FILES].first_visible_entry, 3);
        state.browser.open(&second).unwrap();
        browser.update(&mut state);
        let _ = [first, second].map(fs::remove_dir_all);
        assert_eq!(browser.scrollers[FILES].first_visible_entry, 0);
    }

    fn render(browser: &mut FileBrowser, state: &mut AppState) {
        let mut screen = Framebuffer::new(640, 256);
        browser.update(state);
        browser.render(
            state,
            SCREEN_AREA,
            &mut Painter::new(&mut screen, &state.palette),
        );
    }

    fn click(browser: &mut FileBrowser, state: &mut AppState, list: usize, line: i32) {
        let x = if list == FILES {
            FILE_LIST[0]
        } else {
            SHORTCUT_LIST[0]
        } + 20;
        let y = SCREEN_AREA.y + LISTS_TOP + ENTRY_OFFSET[1] + line * CHAR_HEIGHT + 3;
        let press = Event::MouseDown {
            pos: [x, y],
            button: MouseButton::Left,
        };
        assert!(browser.handle_event(state, &press));
        render(browser, state);
    }

    #[test]
    fn clicking_entries() {
        let mut state = state();
        let temp = temp_folder_with("clicks", 0);
        fs::create_dir_all(temp.join("songs")).unwrap();
        fs::write(temp.join("tune.mmd1"), [0; 10]).unwrap();
        state.browser.open(&temp).unwrap();
        let mut browser = FileBrowser::new();
        render(&mut browser, &mut state);
        // parent, songs, tune.mmd1
        click(&mut browser, &mut state, FILES, 1);
        assert_eq!(state.browser.path(), temp.join("songs"));
        click(&mut browser, &mut state, FILES, 0);
        assert_eq!(state.browser.path(), temp);
        click(&mut browser, &mut state, FILES, 2);
        assert_eq!(state.browser.selected_file(), Some("tune.mmd1"));
        // past the last entry
        click(&mut browser, &mut state, FILES, 5);
        assert_eq!(state.browser.path(), temp);
        let first_shortcut = state.browser.shortcuts()[0].path.clone();
        click(&mut browser, &mut state, SHORTCUTS, 0);
        let _ = fs::remove_dir_all(&temp);
        assert_eq!(state.browser.path(), first_shortcut);
    }
}
