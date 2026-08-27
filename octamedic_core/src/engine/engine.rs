use crate::{
    data::{project::OctamedicProject, song::SongId},
    engine::transport::OctamedicTransport,
};

pub struct OctamedicEngine {
    transport: OctamedicTransport,
    project: OctamedicProject,
    /// Samples remaining before the next sequencer tick fires.
    samples_until_tick: usize,
}

impl OctamedicEngine {
    pub fn new(project: OctamedicProject) -> Self {
        Self {
            transport: OctamedicTransport::new(),
            project,
            samples_until_tick: 0,
        }
    }

    /// Fill `sample_buffer` with unsigned 8-bit PCM (128 = silence).
    /// Returns `true` when the end of the song has been reached.
    ///
    /// # Next steps for implementors
    /// 1. Add a `Voice` type tracking per-track sample position, volume, and pitch.
    /// 2. On tick-0 of each row, create/trigger voices from the row's note events
    ///    (currently dispatched as TODO in `OctamedicTransport::process`).
    /// 3. Replace the `fill(128)` below with a call to mix all active voices
    ///    into the slice at the correct output sample rate.
    pub fn process(&mut self, sample_buffer: &mut [u8], output_sample_rate: u32) -> bool {
        let mut pos = 0;

        while pos < sample_buffer.len() {
            if self.samples_until_tick == 0 {
                if self.transport.process(&self.project) {
                    sample_buffer[pos..].fill(128);
                    return true;
                }
                self.samples_until_tick = self.samples_per_tick(output_sample_rate);
            }

            let chunk = self.samples_until_tick.min(sample_buffer.len() - pos);

            // TODO: mix active voices into sample_buffer[pos..pos + chunk]
            sample_buffer[pos..pos + chunk].fill(128);

            pos += chunk;
            self.samples_until_tick -= chunk;
        }

        false
    }

    /// Number of output samples per sequencer tick at the given sample rate.
    fn samples_per_tick(&self, output_sample_rate: u32) -> usize {
        let song = self.project.get_song(SongId(0)).unwrap();
        let tick_hz = song.tempo.get_tick_rate().as_hertz();
        ((output_sample_rate as f32) / tick_hz).round() as usize
    }
}
