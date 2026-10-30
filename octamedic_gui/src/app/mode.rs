/// Which panel the header shows above the status row; picked with the buttons at its top right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeaderMode {
    Files,
    #[default]
    Play,
    Instrument,
    Block,
    Edit,
    Misc,
    Volume,
    Transpose,
    Range,
}

/// What fills the screen under the header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MainMode {
    #[default]
    BlockEditor,
    /// The block shown as notes on staves.
    NotationEditor,
    SampleEditor,
    SynthEditor,
    SongList,
    Midi,
    Files,
}

impl MainMode {
    /// The block and notation editors, which playing goes back to.
    pub fn is_editor(self) -> bool {
        return matches!(self, MainMode::BlockEditor | MainMode::NotationEditor);
    }
}
