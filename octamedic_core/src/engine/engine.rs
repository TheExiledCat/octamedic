use crate::{data::project::OctamedicProject, engine::transport::OctamedicTransport};

pub struct OctamedicEngine {
    transport: OctamedicTransport,
    project: OctamedicProject,
    samples_until_tick: usize,
}

impl OctamedicEngine {
    pub fn new(project: OctamedicProject) -> Self {

        return Self {
            transport: OctamedicTransport::new(),
            project,
            samples_until_tick: 0,
        };
    }

    /// Fills a sample buffer with samples from the loaded song. Returns true if the end of the song was reached while filling the buffer.

    pub fn process(&mut self, sample_buffer: &mut [u8], output_sample_rate: u32) -> bool {

        let pos = 0;

        while pos < sample_buffer.len() {

            //todo get chunk
            //todo mix_voices and resample to output sample rate

            //add chunk to position
            //todo decrement samples_until_next_tick
            if self.samples_until_tick == 0 {

                self.transport.process(&self.project);
            }
        }

        return false;
    }
}
