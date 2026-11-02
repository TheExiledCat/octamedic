pub mod block_editor;
pub mod file_browser;
pub mod list_scroller;
pub mod song_list;

use rustuition::gadget::Gadget;

use crate::{
    app::{
        main::{block_editor::BlockEditor, file_browser::FileBrowser, song_list::SongList},
        mode::MainMode,
        state::AppState,
    },
    framework::event::Event,
};

pub struct MainScreens {
    pub block_editor: BlockEditor,
    pub file_browser: FileBrowser,
    pub song_list: SongList,
}

impl MainScreens {
    pub fn new() -> Self {
        return Self {
            block_editor: BlockEditor::new(),
            file_browser: FileBrowser::new(),
            song_list: SongList::new(),
        };
    }

    pub fn screen(&mut self, mode: MainMode) -> Option<&mut dyn Gadget<AppState, Event>> {
        return match mode {
            MainMode::BlockEditor => Some(&mut self.block_editor),
            MainMode::Files => Some(&mut self.file_browser),
            MainMode::SongList => Some(&mut self.song_list),
            MainMode::NotationEditor
            | MainMode::SampleEditor
            | MainMode::SynthEditor
            | MainMode::Midi => None,
        };
    }
}
