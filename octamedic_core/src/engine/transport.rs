use crate::data::{
    pattern::{OctamedicPattern, PatternId},
    project::OctamedicProject,
    song::SongId,
};

pub struct OctamedicTransport {
    state: OctamedicTransportState,
    paused: bool,
    row: usize,
    tick: usize,
    sequence_position: usize,
    pattern_id: PatternId,
    song_id: SongId,
}

impl OctamedicTransport {
    pub fn new() -> Self {
        Self {
            state: OctamedicTransportState::Playing,
            paused: true,
            row: 0,
            tick: 0,
            sequence_position: 0,
            pattern_id: PatternId(0),
            song_id: SongId(0),
        }
    }

    pub fn play(&mut self) {
        self.paused = false;
        self.state = OctamedicTransportState::Playing;
    }

    pub fn play_loop(&mut self) {
        self.paused = false;
        self.state = OctamedicTransportState::Looping;
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Advance the sequencer by one tick. Returns `true` if playback has ended.
    pub fn process(&mut self, project: &OctamedicProject) -> bool {
        if self.paused {
            return true;
        }

        if self.tick == 0 {
            // TODO: dispatch note-on events for the current row to the voice allocator
        } else {
            // TODO: apply per-tick effect updates (slides, vibrato, etc.)
        }

        self.tick += 1;

        let ticks_per_line = project
            .get_song(self.song_id)
            .unwrap()
            .tempo
            .ticks_per_line
            .0 as usize;

        if self.tick >= ticks_per_line {
            self.tick = 0;
            self.next_row(project);
        }

        self.paused
    }

    fn next_row(&mut self, project: &OctamedicProject) {
        self.row += 1;

        let pattern = self.get_pattern(project, self.song_id, self.pattern_id);

        if self.row >= pattern.line_count.0 as usize {
            match self.state {
                OctamedicTransportState::Playing => {
                    self.sequence_position += 1;

                    let next_pattern: Option<PatternId> = project
                        .get_song(self.song_id)
                        .unwrap()
                        .get_sequence()
                        .get(self.sequence_position)
                        .map(|s| PatternId::from(*s));

                    match next_pattern {
                        Some(p) => {
                            self.pattern_id = p;
                            self.row = 0;
                            self.tick = 0;
                        }
                        None => self.pause(),
                    }
                }
                OctamedicTransportState::Looping => {
                    self.row = 0;
                    self.tick = 0;
                }
            }
        }
    }

    fn get_pattern<'p>(
        &self,
        project: &'p OctamedicProject,
        song: SongId,
        pattern: PatternId,
    ) -> &'p OctamedicPattern {
        let song = project.get_song(song).unwrap();
        let pattern = song.get_pattern(&pattern).unwrap();
        pattern
    }
}

pub enum OctamedicTransportState {
    Playing,
    Looping,
}
