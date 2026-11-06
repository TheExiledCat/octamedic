use std::{array, path::PathBuf};

use mmd::mmd::{conversion::ToModule, writer::OctamedMMDWriter};
use octamedic_core::{data::instrument::InstrumentKind, engine::record::Recorder, export};

use rustuition::{
    font::Font,
    gadget::Gadget,
    gadgets::dialog::{Checkbox, NumberField},
    painter::{Painter, Pen},
    rect::Rect,
};

use crate::{
    app::{
        files::load_module,
        fonts,
        header::finished_edit,
        pens,
        prompt::Prompt,
        state::{AppState, InstrumentSaveType, ModFormat, SaveFormat},
    },
    framework::{
        event::{Event, MouseButton},
        widgets::{Button, Dialog, Textbox, ToggleGroup, symbols::stacked_text},
    },
};

// from the panel's top left
const SAVE_FORMAT: Rect = Rect::new(0, 0, 90, 45);
const SAVE_FORMAT_LABEL: [i32; 2] = [6, 2];
const FORMATS: Rect = Rect::new(2, 8, 86, 36);
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
    mod_format: Button,
    save_wav: Button,
    save_mp3: Button,
    st_module: Button,
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
            mod_format: Button::new(mod_format_label(ModFormat::default()))
                .font(fonts::NARROW_LABEL),
            save_wav: label_button("WAV"),
            save_mp3: unhooked(label_button("MP3")),
            st_module: unhooked(Button::new("ST-MODULE").font(fonts::LABEL)),
            info: unhooked(label_button("INFO")),
            icon: unhooked(label_button("ICON")),
            song: [label_button("LOAD SONG"), label_button("SAVE SONG")],
            instr: [label_button("LOAD INSTR"), label_button("SAVE INSTR")],
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
        return [
            &mut self.mod_format,
            &mut self.save_wav,
            &mut self.save_mp3,
            &mut self.st_module,
            &mut self.info,
            &mut self.icon,
            &mut self.del_file,
        ]
        .into_iter()
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
        let [load_song, save_song] = &mut self.song;
        if load_song.clicked().is_some() {
            load_selected_song(state);
        }
        if save_song.clicked().is_some() {
            save_song_in_picked_format(state, self.file.text());
        }
        let [load_instrument, save_instrument] = &mut self.instr;
        if load_instrument.clicked().is_some() {
            crate::app::files::load_instrument::load_selected_instrument(state);
        }
        if save_instrument.clicked().is_some() {
            save_active_instrument(state);
        }
        if self.del_file.clicked().is_some() {
            delete_selected_file(state);
        }
        let was_mod_picked = state.save_format == SaveFormat::Mod;
        let mut formats = ToggleGroup::new([
            (&mut self.mod_format, SaveFormat::Mod),
            (&mut self.save_wav, SaveFormat::Wav),
            (&mut self.save_mp3, SaveFormat::Mp3),
            (&mut self.st_module, SaveFormat::StModule),
        ]);
        if let Some((format, click)) = formats.take_click() {
            // picking MOD for the first time just selects it; clicking it again cycles variants
            if format == SaveFormat::Mod && was_mod_picked {
                state.mod_format = match click.button {
                    MouseButton::Right => state.mod_format.previous(),
                    _ => state.mod_format.next(),
                };
            }
            state.save_format = format;
        }
        formats.sync(Some(state.save_format));
        self.mod_format.set_text(mod_format_label(state.mod_format));
        if !self.dir.is_focused() {
            self.dir.set_text(state.browser.path().to_string_lossy());
        }
        if !self.file.is_focused() {
            self.file
                .set_text(state.browser.selected_file().unwrap_or_default());
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        // both see every click, it's how a text box finds out it lost focus
        let handled = self.dir.handle_event(state, event) | self.file.handle_event(state, event);
        return handled
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
        let format_buttons = [
            &mut self.mod_format,
            &mut self.save_wav,
            &mut self.save_mp3,
            &mut self.st_module,
        ];
        let rows = FORMATS.within(area).split_rows(format_buttons.len() as i32);
        for (button, rect) in format_buttons.into_iter().zip(rows) {
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
    state.open_prompt(Prompt::yes_no(
        "Current song has been modified. Are you sure?",
        move |state| load_song(state, &path),
    ));
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

fn save_active_instrument(state: &mut AppState) {
    let Some(instrument) = state.song().instrument(state.active_instrument) else {
        state.notify("There's no instrument to save");
        return;
    };
    let InstrumentKind::Sample(sample) = &instrument.kind else {
        state.notify("Only samples can be saved");
        return;
    };
    let name = instrument.name.trim();
    if name.is_empty() {
        state.notify("Name the instrument first");
        return;
    }
    let save_type = state.instrument_save_type;
    let bytes = match save_type {
        InstrumentSaveType::Raw => export::raw(sample),
        InstrumentSaveType::Iff => export::iff(name, instrument, sample),
        InstrumentSaveType::Wav => export::wav(sample),
    };
    let file_name = instrument_file_name(name, save_type);
    let path = state.browser.path().join(&file_name);
    if path.is_dir() {
        state.notify(format!("There's a folder called {file_name}"));
        return;
    }
    if !path.exists() {
        write_instrument(state, path, &bytes);
        return;
    }
    let question = format!("{file_name} exists, replace it?");
    state.open_prompt(Prompt::yes_no(question, move |state| {
        write_instrument(state, path, &bytes)
    }));
}

fn mod_format_label(format: ModFormat) -> &'static str {
    return match format {
        ModFormat::Mod1WithInstruments => "MOD1(+INSTR)",
        ModFormat::Mod1NoInstruments => "MOD1(NO INST)",
        ModFormat::Mod0 => "MOD0 (OLD)",
    };
}

// a name can't reach into another folder. raw and IFF go without an extension like on the Amiga
fn instrument_file_name(name: &str, save_type: InstrumentSaveType) -> String {
    let name = name.replace(['/', '\\'], "_");
    let wav = save_type == InstrumentSaveType::Wav;
    if wav && !name.to_lowercase().ends_with(".wav") {
        return format!("{name}.wav");
    }
    return name;
}

fn write_instrument(state: &mut AppState, path: PathBuf, bytes: &[u8]) {
    if std::fs::write(&path, bytes).is_err() {
        state.notify("Can't save it");
        return;
    }
    let folder = state.browser.path().to_path_buf();
    let _ = state.browser.open(folder);
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    state.notify(format!(
        "Saved {}",
        file_name.as_deref().unwrap_or_default()
    ));
    state.browser.select_file(file_name);
}

fn save_song_in_picked_format(state: &mut AppState, file_name_hint: &str) {
    if file_name_hint.trim().is_empty() {
        state.notify("No file name");
        return;
    }
    match state.save_format {
        SaveFormat::Wav => {
            let sequence_length = state.song().sequence().len();
            state.open_dialog(save_wav_dialog(file_name_hint.to_owned(), sequence_length));
        }
        SaveFormat::Mod if state.mod_format == ModFormat::Mod1WithInstruments => {
            save_mod1_with_instruments(state, file_name_hint);
        }
        // MOD1(NO INST) and MOD0 (OLD) still need a post-processing step on the full MMD1
        // to_module() produces: stripping samples, or downgrading the block table to MMD0
        SaveFormat::Mod => state.notify("MOD saving isn't ready yet"),
        SaveFormat::Mp3 => state.notify("MP3 saving isn't ready yet"),
        SaveFormat::StModule => state.notify("ST-MODULE saving isn't ready yet"),
    }
}

fn save_mod1_with_instruments(state: &mut AppState, file_name_hint: &str) {
    let module = state.project_mut().to_module();
    let bytes = match OctamedMMDWriter::new().write_module(&module) {
        Ok(bytes) => bytes,
        Err(error) => {
            state.notify(error.to_string());
            return;
        }
    };
    let file_name = file_name_with_extension(file_name_hint, "mmd1");
    let path = state.browser.path().join(&file_name);
    if path.is_dir() {
        state.notify(format!("There's a folder called {file_name}"));
        return;
    }
    if !path.exists() {
        write_song(state, path, &bytes);
        return;
    }
    let question = format!("{file_name} exists, replace it?");
    state.open_prompt(Prompt::yes_no(question, move |state| {
        write_song(state, path, &bytes)
    }));
}

fn save_wav_dialog(file_name_hint: String, sequence_length: usize) -> Dialog {
    let mut dialog = Dialog::new("Render song to WAV")
        .submit_button("Save")
        .cancel_button("Cancel");
    let stereo = dialog.add_field(Checkbox::new("Stereo").checked(true));
    // the sequence is shown to the user as 1-based, like the status bar's position counter
    let last = sequence_length.max(1) as f32;
    let from = dialog.add_field(NumberField::new("From", 1.0, last, 1.0));
    let to = dialog.add_field(NumberField::new("To", 1.0, last, 1.0).value(last));
    return dialog.on_submit_checked(move |state, result| {
        let channels: u16 = if *result.get(stereo) { 2 } else { 1 };
        let range = wav_sequence_range(*result.get(from), *result.get(to));
        let recorder = Recorder {
            channels,
            ..Recorder::default()
        };
        let recording = recorder
            .record_range(state.song(), range)
            .map_err(|error| error.to_string())?;
        let bytes = recording.to_wav_bytes();
        let file_name = file_name_with_extension(&file_name_hint, "wav");
        let path = state.browser.path().join(&file_name);
        if path.is_dir() {
            return Err(format!("There's a folder called {file_name}"));
        }
        if !path.exists() {
            write_song(state, path, &bytes);
            return Ok(());
        }
        let question = format!("{file_name} exists, replace it?");
        state.open_prompt(Prompt::yes_no(question, move |state| {
            write_song(state, path, &bytes)
        }));
        return Ok(());
    });
}

// `from`/`to` are the dialog's 1-based, inclusive positions
fn wav_sequence_range(from: f32, to: f32) -> std::ops::Range<usize> {
    let start = (from.round() as usize).saturating_sub(1);
    let end = to.round() as usize;
    return start..end;
}

// save_song_in_picked_format already turns an empty FILE box away with "No file name"
fn file_name_with_extension(name: &str, extension: &str) -> String {
    let name = name.trim().replace(['/', '\\'], "_");
    let suffix = format!(".{extension}");
    if !name.to_lowercase().ends_with(&suffix.to_lowercase()) {
        return format!("{name}{suffix}");
    }
    return name;
}

fn write_song(state: &mut AppState, path: PathBuf, bytes: &[u8]) {
    if std::fs::write(&path, bytes).is_err() {
        state.notify("Can't save it");
        return;
    }
    let folder = state.browser.path().to_path_buf();
    let _ = state.browser.open(folder);
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    state.notify(format!(
        "Saved {}",
        file_name.as_deref().unwrap_or_default()
    ));
    state.browser.select_file(file_name);
}

fn load_song(state: &mut AppState, path: &std::path::Path) {
    match load_module(path) {
        Ok(project) => state.load_project(project),
        Err(error) => state.notify(error.to_string()),
    }
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
    use std::sync::Arc;

    use octamedic_core::{
        data::{
            instrument::{OctamedicInstrument, OctamedicSample, SampleOctaves},
            project::OctamedicProject,
        },
        engine::handle::EngineHandle,
    };
    use rustuition::rect::Point;

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
            pos: Point::new(SONG.x + 5, SONG.y + 3),
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

    fn click_mod_format(
        files: &mut Files,
        state: &mut AppState,
        button: rustuition::event::MouseButton,
    ) {
        let mut screen = rustuition::target::Framebuffer::new(460, 50);
        let area = screen.bounds();
        files.render(state, area, &mut Painter::new(&mut screen, &state.palette));
        let pos = Point::new(FORMATS.x + 5, FORMATS.y + 3);
        assert!(files.handle_event(state, &Event::MouseDown { pos, button }));
        files.handle_event(state, &Event::MouseUp { pos, button });
        files.update(state);
    }

    #[test]
    fn mod_format_cycles_on_click() {
        use ModFormat::{Mod0, Mod1NoInstruments, Mod1WithInstruments};
        use rustuition::event::MouseButton::{Left, Right};
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut files = Files::new();
        assert_eq!(state.mod_format, Mod1WithInstruments);
        click_mod_format(&mut files, &mut state, Left);
        assert_eq!(state.mod_format, Mod1NoInstruments);
        click_mod_format(&mut files, &mut state, Left);
        assert_eq!(state.mod_format, Mod0);
        click_mod_format(&mut files, &mut state, Left);
        assert_eq!(state.mod_format, Mod1WithInstruments);
        click_mod_format(&mut files, &mut state, Right);
        assert_eq!(state.mod_format, Mod0);
    }

    #[test]
    fn picking_mod_the_first_time_only_selects_it() {
        use rustuition::event::MouseButton::Left;
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut files = Files::new();
        let started_at = state.mod_format;
        state.save_format = SaveFormat::Wav;
        click_mod_format(&mut files, &mut state, Left);
        assert_eq!(state.save_format, SaveFormat::Mod);
        assert_eq!(state.mod_format, started_at);
        click_mod_format(&mut files, &mut state, Left);
        assert_eq!(state.mod_format, started_at.next());
    }

    #[test]
    fn save_song_needs_a_file_name() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.save_format = SaveFormat::Wav;
        save_song_in_picked_format(&mut state, "   ");
        assert_eq!(state.notification.as_deref(), Some("No file name"));
        assert!(state.take_pending_dialogs().is_empty());
    }

    #[test]
    fn wav_save_uses_the_file_box_name() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.save_format = SaveFormat::Wav;
        let folder = std::env::temp_dir().join(format!("octamedic-wav-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        state.browser.open(&folder).unwrap();
        save_song_in_picked_format(&mut state, "myrender");
        let mut dialogs = state.take_pending_dialogs();
        dialogs[0].handle_event(&mut state, &key(ggez::input::keyboard::KeyCode::Return));
        let saved = folder.join("myrender.wav").exists();
        let _ = std::fs::remove_dir_all(&folder);
        assert!(saved);
    }

    #[test]
    fn wav_sequence_range_is_1_based_inclusive() {
        // position 1 to 3, shown to the user, covers 0-based indices 0, 1 and 2
        assert_eq!(wav_sequence_range(1.0, 3.0), 0..3);
        // just the first position
        assert_eq!(wav_sequence_range(1.0, 1.0), 0..1);
        // starting partway through
        assert_eq!(wav_sequence_range(2.0, 4.0), 1..4);
    }

    #[test]
    fn wav_dialog_defaults_to_the_whole_song() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.save_format = SaveFormat::Wav;
        let folder =
            std::env::temp_dir().join(format!("octamedic-wav-range-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        state.browser.open(&folder).unwrap();
        // the dialog's Stereo checkbox defaults to checked
        let expected = Recorder {
            channels: 2,
            ..Recorder::default()
        }
        .record_song(state.song(), 0)
        .unwrap()
        .to_wav_bytes();
        save_song_in_picked_format(&mut state, "myrender");
        let mut dialogs = state.take_pending_dialogs();
        dialogs[0].handle_event(&mut state, &key(ggez::input::keyboard::KeyCode::Return));
        let saved = std::fs::read(folder.join("myrender.wav")).unwrap();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(saved, expected);
    }

    #[test]
    fn mod1_with_instruments_writes_a_file() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.save_format = SaveFormat::Mod;
        assert_eq!(state.mod_format, ModFormat::Mod1WithInstruments);
        let folder = std::env::temp_dir().join(format!("octamedic-mod-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        state.browser.open(&folder).unwrap();
        save_song_in_picked_format(&mut state, "mysong");
        let saved = folder.join("mysong.mmd1").exists();
        let _ = std::fs::remove_dir_all(&folder);
        assert!(saved);
        assert_eq!(state.notification.as_deref(), Some("Saved mysong.mmd1"));
    }

    #[test]
    fn other_mod_formats_still_say_not_ready() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.save_format = SaveFormat::Mod;
        for format in [ModFormat::Mod1NoInstruments, ModFormat::Mod0] {
            state.mod_format = format;
            save_song_in_picked_format(&mut state, "mysong");
            assert_eq!(
                state.notification.as_deref(),
                Some("MOD saving isn't ready yet"),
                "{format:?}"
            );
        }
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
        let prompt = state.prompt.as_ref().map(|prompt| prompt.text());
        assert_eq!(
            prompt,
            Some("Current song has been modified. Are you sure? (Y/N)")
        );
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

    fn saving(name: &str) -> (AppState, PathBuf) {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let folder = std::env::temp_dir().join(format!(
            "octamedic-save-{}-{}",
            name.replace('/', "_"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        state.browser.open(&folder).unwrap();
        let mut instrument = OctamedicInstrument::new(name);
        instrument.kind = InstrumentKind::Sample(Arc::new(OctamedicSample {
            data: vec![0, 10, -10, 20],
            repeat: None,
            octaves: SampleOctaves::One,
        }));
        let active = state.active_instrument;
        state.song_mut().set_instrument(active, Some(instrument));
        return (state, folder);
    }

    #[test]
    fn save_instr_saves_in_the_picked_format() {
        let (mut state, folder) = saving("kick");
        let sample = state
            .song()
            .instrument(state.active_instrument)
            .and_then(|instrument| instrument.sample().cloned())
            .unwrap();
        let instrument = state.song().instrument(state.active_instrument).unwrap();
        let expected = [
            (InstrumentSaveType::Raw, "kick", export::raw(&sample)),
            (
                InstrumentSaveType::Iff,
                "kick",
                export::iff("kick", instrument, &sample),
            ),
            (InstrumentSaveType::Wav, "kick.wav", export::wav(&sample)),
        ];
        for (save_type, file_name, bytes) in expected {
            let _ = std::fs::remove_file(folder.join("kick"));
            state.instrument_save_type = save_type;
            save_active_instrument(&mut state);
            let saved = std::fs::read(folder.join(file_name)).ok();
            assert_eq!(saved, Some(bytes), "{save_type:?}");
            assert_eq!(state.browser.selected_file(), Some(file_name));
        }
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn save_instr_asks_before_replacing() {
        let (mut state, folder) = saving("snare");
        let path = folder.join("snare");
        std::fs::write(&path, b"old").unwrap();
        save_active_instrument(&mut state);
        let prompt = state.prompt.as_ref().map(Prompt::text);
        assert_eq!(prompt, Some("snare exists, replace it? (Y/N)"));
        assert_eq!(std::fs::read(&path).unwrap(), b"old");
        crate::app::prompt::handle_event(&mut state, &key(ggez::input::keyboard::KeyCode::Y));
        let replaced = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_dir_all(&folder);
        assert_eq!(replaced.len(), 4);
    }

    #[test]
    fn save_instr_wont_replace_a_folder() {
        let (mut state, folder) = saving("..");
        save_active_instrument(&mut state);
        assert_eq!(
            state.notification.as_deref(),
            Some("There's a folder called ..")
        );
        assert!(state.prompt.is_none());
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn save_instr_needs_a_named_sample() {
        let (mut state, folder) = saving("  ");
        save_active_instrument(&mut state);
        assert_eq!(
            state.notification.as_deref(),
            Some("Name the instrument first")
        );
        state.edit_active_instrument(|instrument| instrument.kind = InstrumentKind::Empty);
        save_active_instrument(&mut state);
        assert_eq!(
            state.notification.as_deref(),
            Some("Only samples can be saved")
        );
        let active = state.active_instrument;
        state.song_mut().set_instrument(active, None);
        save_active_instrument(&mut state);
        assert_eq!(
            state.notification.as_deref(),
            Some("There's no instrument to save")
        );
        let empty = std::fs::read_dir(&folder).unwrap().next().is_none();
        let _ = std::fs::remove_dir_all(&folder);
        assert!(empty);
    }

    #[test]
    fn file_names_stay_in_the_folder() {
        assert_eq!(
            instrument_file_name("a/b\\c", InstrumentSaveType::Raw),
            "a_b_c"
        );
        assert_eq!(
            instrument_file_name("bass", InstrumentSaveType::Wav),
            "bass.wav"
        );
        assert_eq!(
            instrument_file_name("bass.WAV", InstrumentSaveType::Wav),
            "bass.WAV"
        );
    }

    #[test]
    fn unhooked_buttons_are_ghosted() {
        let files = Files::new();
        assert!(!files.info.is_enabled() && !files.save_mp3.is_enabled());
        assert!(!files.st_module.is_enabled());
        assert!(files.mod_format.is_enabled());
        assert!(files.song[0].is_enabled() && files.song[1].is_enabled());
        assert!(files.del_file.is_enabled());
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
