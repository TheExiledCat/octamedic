use std::ops::Deref;

use octamed::utility::bytes::UByte;

use crate::data::{
    pattern::{OctamedicPattern, PatternId},
    project::OctamedicProject,
    song::SongId,
    tempo::OctamedicTempo,
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

        return Self {
            state: OctamedicTransportState::Playing,
            paused: true,
            row: 0,
            tick: 0,
            sequence_position: 0,
            pattern_id: PatternId(0),
            song_id: SongId(0),
        };
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

    pub fn process(&mut self, project: &OctamedicProject) {

        let tempo = project.get_song(self.song_id).unwrap().tempo;

        if self.tick == 0 {

            //todo collect and trigger row events
            return;
        }

        if self.tick > 0 {
            // todo apply effect ticks
        }

        self.tick += 1;

        let ticks_per_line = tempo.ticks_per_line.0;

        if self.tick >= ticks_per_line as usize {

            self.next_row(project);
        }
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

        return pattern;
    }
}

pub enum OctamedicTransportState {
    Playing,
    Looping,
}
