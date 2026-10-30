pub mod block_editor;

use crate::{
    app::{main::block_editor::BlockEditor, mode::MainMode},
    framework::widget::OctamedicWidget,
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

    /// The screen `mode` shows; `None` for the ones that aren't made yet.
    pub fn screen(&mut self, mode: MainMode) -> Option<&mut dyn OctamedicWidget> {
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
