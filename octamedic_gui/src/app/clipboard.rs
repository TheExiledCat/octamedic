use octamedic_core::data::pattern::OctamedicPattern;

// everything here survives loading another song, so you can copy between songs
#[derive(Default)]
pub struct Clipboard {
    // TRK:'s buffer. A copied track is kept as a block one track wide
    pub track: Option<OctamedicPattern>,
    // BLK:'s buffer
    pub block: Option<OctamedicPattern>,
    // the RANGE panel's CUT and COPY
    pub range: Option<OctamedicPattern>,
    // the sample editor's CBUFF
    pub sample: Vec<i8>,
}

impl Clipboard {
    // DEL BUFF on the block screen. CBUFF has its own DEL CB
    pub fn clear_patterns(&mut self) {
        self.track = None;
        self.block = None;
        self.range = None;
    }
}
