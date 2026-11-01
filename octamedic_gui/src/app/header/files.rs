use std::array;

use rustuition::{
    font::Font,
    gadget::Gadget,
    gadgets::button::{ButtonContent, ContentArea},
    painter::{Painter, Pen},
    rect::Rect,
};

use crate::{
    app::{fonts, pens, state::AppState},
    framework::{
        event::Event,
        widgets::{Button, Textbox},
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
        return Self {
            formats: FORMAT_LABELS.map(|(label, font)| Button::new(label).font(font)),
            info: label_button("INFO"),
            icon: label_button("ICON"),
            song: ["LOAD SONG", "SAVE SONG"].map(label_button),
            instr: ["LOAD INSTR", "SAVE INSTR"].map(label_button),
            del_file: Button::new(StackedText(&DEL_FILE_LINES)),
            msg: array::from_fn(|_| label_button("MSG")),
            map: array::from_fn(|_| label_button("MAP")),
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
    match crate::app::files::load_module(&path) {
        Ok(project) => state.load_project(project),
        Err(error) => state.notify(format!("Can't load {}: {error}", path.display())),
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

struct StackedText(&'static [(&'static str, [i32; 2])]);

impl ButtonContent for StackedText {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        for (text, [x, y]) in self.0 {
            let (x, y) = (area.inside.x + x, area.inside.y + y);
            painter.text_in_font(x, y, text, pen, fonts::SMALL);
        }
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
        state.set_song_name("kept");
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
        assert!(state.notification.unwrap().starts_with("Can't load"));
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
