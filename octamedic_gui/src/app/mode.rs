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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MainMode {
    #[default]
    BlockEditor,
    NotationEditor,
    SampleEditor,
    SynthEditor,
    SongList,
    Midi,
    Files,
}

impl MainMode {
    pub fn is_editor(self) -> bool {
        return matches!(self, MainMode::BlockEditor | MainMode::NotationEditor);
    }
}
