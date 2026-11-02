use std::array;

use rustuition::{
    font::Font,
    gadget::Gadget,
    gadgets::dialog::Label,
    painter::{Painter, Pen},
    rect::Rect,
};

use crate::{
    app::{files::load_module, fonts, pens, state::AppState},
    framework::{
        event::Event,
        widgets::{Button, Dialog, Textbox, symbols::stacked_text},
    },
};

// from the panel's top left
const SAVE_FORMAT: Rect = Rect::new(0, 0, 90, 45);
const SAVE_FORMAT_LABEL: [i32; 2] = [6, 2];
const FORMATS: Rect = Rect::new(2, 8, 86, 36);
// the MOD1 ones are squeezed in on the real thing
const FORMAT_LABELS: [(&str, Font); 4] = [
    ("MOD1(+INSTR)", fonts::NARROW_LABEL),
    ("MOD1(NO INST)", fonts::NARROW_LABEL),
    ("MOD0 (OLD)", fonts::LABEL),
    ("ST-MODULE", fonts::LABEL),
];
const INFO: Rect = Rect::new(90, 0, 34, 11);
const ICON: Rect = Rect::new(90, 11, 34, 11);
const SONG: Rect = Rect::new(124, 0, 94, 22);
const INSTR: Rect = Rect::new(218, 0, 94, 22);
const DEL_FILE: Rect = Rect::new(312, 0, 35, 22);
const MESSAGES: Rect = Rect::new(347, 0, 101, 22);
const LOAD_LABEL: [i32; 2] = [350, 4];
const SAVE_LABEL: [i32; 2] = [349, 13];
const MSG: Rect = Rect::new(380, 2, 33, 18);
const MAP: Rect = Rect::new(413, 2, 33, 18);
const DIR_LABEL: [i32; 2] = [96, 27];
const FILE_LABEL: [i32; 2] = [92, 37];
const DIR: Rect = Rect::new(124, 23, 324, 11);
const FILE: Rect = Rect::new(124, 34, 324, 11);

// from inside the button's frame
const DEL_FILE_LINES: [(&str, [i32; 2]); 2] = [("DEL", [5, 4]), ("FILE", [4, 11])];

pub struct Files {
    formats: [Button; 4],
    info: Button,
    icon: Button,
    // load, save
    song: [Button; 2],
    instr: [Button; 2],
    del_file: Button,
    msg: [Button; 2],
    map: [Button; 2],
    dir: Textbox,
    file: Textbox,
    dir_was_focused: bool,
    file_was_focused: bool,
}

impl Files {
    pub fn new() -> Self {
        let name_box = || {
            Textbox::new()
                .pens(Pen::BACKGROUND, Pen::SHINE, Pen::FILL, pens::RED)
                .padding(0)
        };
        let unhooked = |button: Button| button.enabled(false);
        return Self {
            formats: FORMAT_LABELS.map(|(label, font)| unhooked(Button::new(label).font(font))),
            info: unhooked(label_button("INFO")),
            icon: unhooked(label_button("ICON")),
            song: [
                label_button("LOAD SONG"),
                unhooked(label_button("SAVE SONG")),
            ],
            instr: [
                label_button("LOAD INSTR"),
                unhooked(label_button("SAVE INSTR")),
            ],
            del_file: Button::new(stacked_text(&DEL_FILE_LINES)),
            msg: array::from_fn(|_| unhooked(label_button("MSG"))),
            map: array::from_fn(|_| unhooked(label_button("MAP"))),
            dir: name_box(),
            file: name_box(),
            dir_was_focused: false,
            file_was_focused: false,
        };
    }

    fn buttons(&mut self) -> impl Iterator<Item = &mut Button> {
        return self
            .formats
            .iter_mut()
            .chain([&mut self.info, &mut self.icon, &mut self.del_file])
            .chain(&mut self.song)
            .chain(&mut self.instr)
            .chain(&mut self.msg)
            .chain(&mut self.map);
    }
}

impl Gadget<AppState, Event> for Files {
    // a folder that can't be opened leaves the browser where it was, so DIR goes back to its path
    fn update(&mut self, state: &mut AppState) {
        if let Some(dir) = finished_edit(&self.dir, &mut self.dir_was_focused) {
            if !dir.is_empty() {
                let _ = state.browser.open(dir);
            }
        }
        if let Some(file) = finished_edit(&self.file, &mut self.file_was_focused) {
            state
                .browser
                .select_file((!file.is_empty()).then_some(file));
        }
        let [load_song, _] = &mut self.song;
        if load_song.clicked().is_some() {
            load_selected_song(state);
        }
        let [load_instrument, _] = &mut self.instr;
        if load_instrument.clicked().is_some() {
            crate::app::files::load_instrument::load_selected_instrument(state);
        }
        if self.del_file.clicked().is_some() {
            delete_selected_file(state);
        }
        if !self.dir.is_focused() {
            self.dir.set_text(state.browser.path().to_string_lossy());
        }
        if !self.file.is_focused() {
            self.file
                .set_text(state.browser.selected_file().unwrap_or_default());
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.dir.handle_event(state, event)
            || self.file.handle_event(state, event)
            || self
                .buttons()
                .any(|button| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        painter.frame(SAVE_FORMAT.within(area), 1);
        draw_label(
            painter,
            area,
            SAVE_FORMAT_LABEL,
            "SAVE FORMAT:",
            fonts::NARROW_LABEL,
        );
        let formats = FORMATS.within(area).split_rows(self.formats.len() as i32);
        for (button, rect) in self.formats.iter_mut().zip(formats) {
            button.render(state, rect, painter);
        }
        self.info.render(state, INFO.within(area), painter);
        self.icon.render(state, ICON.within(area), painter);
        render_stacked_pair(&mut self.song, SONG.within(area), state, painter);
        render_stacked_pair(&mut self.instr, INSTR.within(area), state, painter);
        self.del_file.render(state, DEL_FILE.within(area), painter);

        painter.frame(MESSAGES.within(area), 1);
        draw_label(painter, area, LOAD_LABEL, "LOAD", fonts::LABEL);
        draw_label(painter, area, SAVE_LABEL, "SAVE", fonts::LABEL);
        render_stacked_pair(&mut self.msg, MSG.within(area), state, painter);
        render_stacked_pair(&mut self.map, MAP.within(area), state, painter);

        draw_label(painter, area, DIR_LABEL, "DIR:", fonts::LABEL);
        draw_label(painter, area, FILE_LABEL, "FILE:", fonts::LABEL);
        self.dir.render(state, DIR.within(area), painter);
        self.file.render(state, FILE.within(area), painter);
    }
}

fn load_selected_song(state: &mut AppState) {
    let Some(name) = state.browser.selected_file() else {
        state.notify("Select a song to load first");
        return;
    };
    let path = state.browser.path().join(name);
    if state.is_saved() {
        load_song(state, &path);
        return;
    }
    let dialog = Dialog::new("Load song")
        .with(Label::new("The song has unsaved changes.\nLoad anyway?"))
        .submit_button("Load")
        .cancel_button("Cancel")
        .on_submit_checked(move |state, _| {
            let project = load_module(&path).map_err(|error| error.to_string())?;
            state.load_project(project);
            return Ok(());
        });
    state.open_dialog(dialog);
}

fn delete_selected_file(state: &mut AppState) {
    let Some(name) = state.browser.selected_file() else {
        state.notify("Select a file to delete first");
        return;
    };
    let path = state.browser.path().join(name);
    let dialog = Dialog::confirm("Delete file", &format!("Delete {name}?")).on_submit_checked(
        move |state, _| {
            std::fs::remove_file(&path).map_err(|_| "Can't delete it".to_owned())?;
            let folder = state.browser.path().to_path_buf();
            let _ = state.browser.open(folder);
            return Ok(());
        },
    );
    state.open_dialog(dialog);
}

fn load_song(state: &mut AppState, path: &std::path::Path) {
    match load_module(path) {
        Ok(project) => state.load_project(project),
        Err(error) => state.notify(error.to_string()),
    }
}

fn finished_edit(textbox: &Textbox, was_focused: &mut bool) -> Option<String> {
    let left = *was_focused && !textbox.is_focused();
    *was_focused = textbox.is_focused();
    return left.then(|| textbox.text().to_owned());
}

fn label_button(label: &'static str) -> Button {
    return Button::new(label).font(fonts::LABEL);
}

fn draw_label(painter: &mut Painter, area: Rect, [x, y]: [i32; 2], text: &str, font: Font) {
    painter.text_in_font(area.x + x, area.y + y, text, Pen::BACKGROUND, font);
}

fn render_stacked_pair(
    buttons: &mut [Button; 2],
    rect: Rect,
    state: &AppState,
    painter: &mut Painter,
) {
    for (button, rect) in buttons.iter_mut().zip(rect.split_rows(2)) {
        button.render(state, rect, painter);
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;

    fn enter_dir(files: &mut Files, state: &mut AppState, text: &str) {
        files.dir.set_focused(true);
        files.update(state);
        files.dir.set_text(text);
        files.dir.set_focused(false);
        files.update(state);
    }

    #[test]
    fn dir_shows_path() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut files = Files::new();
        files.update(&mut state);
        assert_eq!(files.dir.text(), state.browser.path().to_string_lossy());
    }

    #[test]
    fn dir_opens_folder() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut files = Files::new();
        let temp = std::env::temp_dir();
        enter_dir(&mut files, &mut state, &temp.to_string_lossy());
        assert_eq!(state.browser.path(), temp);
        assert_eq!(files.dir.text(), temp.to_string_lossy());
    }

    #[test]
    fn unreadable_dir_goes_back() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut files = Files::new();
        let before = state.browser.path().to_path_buf();
        enter_dir(&mut files, &mut state, "/no/such/folder/here");
        assert_eq!(state.browser.path(), before);
        assert_eq!(files.dir.text(), before.to_string_lossy());
    }

    #[test]
    fn file_shows_and_sets_selection() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut files = Files::new();
        state.browser.select_file(Some("tune.mmd1".to_owned()));
        files.update(&mut state);
        assert_eq!(files.file.text(), "tune.mmd1");
        files.file.set_focused(true);
        files.update(&mut state);
        files.file.set_text("other.mmd0");
        files.file.set_focused(false);
        files.update(&mut state);
        assert_eq!(state.browser.selected_file(), Some("other.mmd0"));
    }

    fn click_load_song(files: &mut Files, state: &mut AppState) {
        let mut screen = rustuition::target::Framebuffer::new(460, 50);
        let area = screen.bounds();
        files.render(state, area, &mut Painter::new(&mut screen, &state.palette));
        let press = Event::MouseDown {
            pos: [SONG.x + 5, SONG.y + 3],
            button: rustuition::event::MouseButton::Left,
        };
        assert!(files.handle_event(state, &press));
        files.update(state);
    }

    #[test]
    fn load_song_needs_a_selection() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut files = Files::new();
        click_load_song(&mut files, &mut state);
        assert_eq!(
            state.notification.as_deref(),
            Some("Select a song to load first")
        );
    }

    #[test]
    fn unloadable_song_keeps_the_old_one() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut kept = OctamedicProject::new();
        kept.get_song_mut(octamedic_core::data::song::SongId(0))
            .unwrap()
            .set_name("kept");
        state.load_project(kept);
        let temp = std::env::temp_dir().join(format!("octamedic-load-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        std::fs::write(temp.join("broken.mmd0"), b"not a module").unwrap();
        state.browser.open(&temp).unwrap();
        state.browser.select_file(Some("broken.mmd0".to_owned()));
        let mut files = Files::new();
        click_load_song(&mut files, &mut state);
        let _ = std::fs::remove_dir_all(&temp);
        assert_eq!(state.song().name(), "kept");
        assert_eq!(state.notification.as_deref(), Some("Not a song"));
    }

    #[test]
    fn unsaved_changes_ask_first() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.set_song_name("edited");
        state.browser.select_file(Some("anything.mmd0".to_owned()));
        let mut files = Files::new();
        click_load_song(&mut files, &mut state);
        assert_eq!(state.take_pending_dialogs().len(), 1);
        assert_eq!(state.song().name(), "edited");
    }

    fn folder_with_file(name: &str) -> (AppState, std::path::PathBuf) {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let folder =
            std::env::temp_dir().join(format!("octamedic-del-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(name), b"x").unwrap();
        state.browser.open(&folder).unwrap();
        state.browser.select_file(Some(name.to_owned()));
        return (state, folder);
    }

    fn key(key: ggez::input::keyboard::KeyCode) -> Event {
        return Event::Key {
            key,
            mods: ggez::input::keyboard::KeyMods::NONE,
            repeat: false,
        };
    }

    #[test]
    fn del_file_asks_then_deletes() {
        let (mut state, folder) = folder_with_file("old.wav");
        delete_selected_file(&mut state);
        let mut dialogs = state.take_pending_dialogs();
        dialogs[0].handle_event(&mut state, &key(ggez::input::keyboard::KeyCode::Return));
        let gone = !folder.join("old.wav").exists();
        let _ = std::fs::remove_dir_all(&folder);
        assert!(gone);
        assert!(state.browser.files().is_empty());
        assert_eq!(state.browser.selected_file(), None);
    }

    #[test]
    fn del_file_cancelled_keeps_it() {
        let (mut state, folder) = folder_with_file("keep.wav");
        delete_selected_file(&mut state);
        let mut dialogs = state.take_pending_dialogs();
        dialogs[0].handle_event(&mut state, &key(ggez::input::keyboard::KeyCode::Escape));
        let kept = folder.join("keep.wav").exists();
        let _ = std::fs::remove_dir_all(&folder);
        assert!(kept);
    }

    #[test]
    fn del_file_needs_a_selection() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        delete_selected_file(&mut state);
        assert_eq!(
            state.notification.as_deref(),
            Some("Select a file to delete first")
        );
        assert!(state.take_pending_dialogs().is_empty());
    }

    #[test]
    fn unhooked_buttons_are_ghosted() {
        let files = Files::new();
        assert!(!files.info.is_enabled() && !files.song[1].is_enabled());
        assert!(files.song[0].is_enabled() && files.del_file.is_enabled());
    }

    #[test]
    fn typing_doesnt_open() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut files = Files::new();
        let before = state.browser.path().to_path_buf();
        files.dir.set_focused(true);
        files.update(&mut state);
        files.dir.set_text(std::env::temp_dir().to_string_lossy());
        files.update(&mut state);
        assert_eq!(state.browser.path(), before);
    }
}
