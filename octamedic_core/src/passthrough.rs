use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

// in milliseconds. the two devices' clocks drift apart, so what's waiting can only grow so far
// before it's cut back, which keeps the delay down
const MOST_WAITING_MS: u32 = 50;
const CUT_BACK_TO_MS: u32 = 10;

/// The input on its way to the output while the sample editor listens to it, so it can be heard
/// like the sampler on an Amiga. The input pushes, and the engine's stream mixes it in at its
/// own rate.
#[derive(Clone, Default)]
pub struct Passthrough {
    shared: Arc<Shared>,
}

#[derive(Default)]
struct Shared {
    waiting: Mutex<VecDeque<f32>>,
    // 0 while there's no input
    input_rate: AtomicU32,
}

impl Passthrough {
    pub(crate) fn start(&self, input_rate: u32) {
        if let Ok(mut waiting) = self.shared.waiting.lock() {
            waiting.clear();
        }
        self.shared.input_rate.store(input_rate, Ordering::Relaxed);
    }

    pub(crate) fn stop(&self) {
        self.shared.input_rate.store(0, Ordering::Relaxed);
    }

    pub(crate) fn push(&self, samples: impl Iterator<Item = f32>) {
        let rate = self.shared.input_rate.load(Ordering::Relaxed);
        let Ok(mut waiting) = self.shared.waiting.lock() else {
            return;
        };
        waiting.extend(samples);
        if waiting.len() > milliseconds(rate, MOST_WAITING_MS) {
            let over = waiting.len() - milliseconds(rate, CUT_BACK_TO_MS);
            waiting.drain(..over);
        }
    }

    pub(crate) fn listener(&self, output_rate: u32) -> Listener {
        return Listener {
            shared: self.shared.clone(),
            output_rate,
            position: 0.0,
            from: 0.0,
            to: 0.0,
        };
    }
}

fn milliseconds(rate: u32, milliseconds: u32) -> usize {
    return (rate as usize * milliseconds as usize / 1000).max(1);
}

/// The output's end, on its stream's thread.
pub(crate) struct Listener {
    shared: Arc<Shared>,
    output_rate: u32,
    // how far it is from `from` to `to`, the two input samples it's between
    position: f64,
    from: f32,
    to: f32,
}

impl Listener {
    /// Adds the input to every channel of `mix`, at full level.
    pub(crate) fn mix_into(&mut self, mix: &mut [f32], channels: usize) {
        let input_rate = self.shared.input_rate.load(Ordering::Relaxed);
        if input_rate == 0 || self.output_rate == 0 {
            return;
        }
        // the output can't wait on the input's thread, a frame without it is only a click
        let Ok(mut waiting) = self.shared.waiting.try_lock() else {
            return;
        };
        let step = input_rate as f64 / self.output_rate as f64;
        for frame in mix.chunks_mut(channels.max(1)) {
            self.position += step;
            while self.position >= 1.0 {
                let Some(next) = waiting.pop_front() else {
                    // run out: stay on the last one until more comes
                    self.position = 0.0;
                    self.from = self.to;
                    break;
                };
                self.from = self.to;
                self.to = next;
                self.position -= 1.0;
            }
            let level = self.from + (self.to - self.from) * self.position as f32;
            for out in frame {
                *out += level;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_rate_comes_through_a_sample_behind() {
        let passthrough = Passthrough::default();
        passthrough.start(8000);
        passthrough.push([0.5, -0.5, 0.25].into_iter());
        let mut listener = passthrough.listener(8000);
        let mut mix = [0.0; 6];
        listener.mix_into(&mut mix, 2);
        // it goes from one input sample to the next, so it starts from silence
        assert_eq!(mix, [0.0, 0.0, 0.5, 0.5, -0.5, -0.5]);
    }

    #[test]
    fn it_adds_to_whats_playing() {
        let passthrough = Passthrough::default();
        passthrough.start(8000);
        passthrough.push([0.5, 0.5].into_iter());
        let mut listener = passthrough.listener(8000);
        let mut mix = [0.25, 0.25];
        listener.mix_into(&mut mix, 1);
        assert_eq!(mix, [0.25, 0.75]);
    }

    #[test]
    fn twice_the_output_rate_goes_between_samples() {
        let passthrough = Passthrough::default();
        passthrough.start(8000);
        passthrough.push([1.0, 0.0].into_iter());
        let mut listener = passthrough.listener(16000);
        let mut mix = [0.0; 4];
        listener.mix_into(&mut mix, 1);
        // halfway to each input sample, then on it
        assert_eq!(mix, [0.0, 0.0, 0.5, 1.0]);
    }

    #[test]
    fn nothing_without_an_input() {
        let passthrough = Passthrough::default();
        passthrough.push([1.0].into_iter());
        let mut listener = passthrough.listener(8000);
        let mut mix = [0.0];
        listener.mix_into(&mut mix, 1);
        assert_eq!(mix, [0.0]);
        passthrough.start(8000);
        passthrough.stop();
        listener.mix_into(&mut mix, 1);
        assert_eq!(mix, [0.0]);
    }

    #[test]
    fn a_long_wait_is_cut_back() {
        let passthrough = Passthrough::default();
        passthrough.start(1000);
        // a second at 1 kHz, far more than 50 ms
        passthrough.push(std::iter::repeat_n(0.0, 1000));
        let waiting = passthrough.shared.waiting.lock().unwrap().len();
        assert_eq!(waiting, milliseconds(1000, CUT_BACK_TO_MS));
    }
}
