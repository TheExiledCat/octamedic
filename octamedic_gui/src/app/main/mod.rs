pub mod block_editor;

use rustuition::gadget::Gadget;

use crate::{
    app::{main::block_editor::BlockEditor, mode::MainMode, state::AppState},
    framework::event::Event,
};

pub struct MainScreens {
    pub block_editor: BlockEditor,
}

impl MainScreens {
    pub fn new() -> Self {
        return Self {
            block_editor: BlockEditor::new(),
        };
    }

    pub fn screen(&mut self, mode: MainMode) -> Option<&mut dyn Gadget<AppState, Event>> {
        return match mode {
            MainMode::BlockEditor => Some(&mut self.block_editor),
            MainMode::NotationEditor
            | MainMode::SampleEditor
            | MainMode::SynthEditor
            | MainMode::SongList
            | MainMode::Midi
            | MainMode::Files => None,
        };
    }
}
