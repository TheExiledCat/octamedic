use std::f64::consts::PI;

// the LED filter's sallen-key parts, same on the A500 and A1200
const R1: f64 = 10_000.0;
const R2: f64 = 10_000.0;
const C1: f64 = 6800e-12;
const C2: f64 = 3900e-12;

// way below anything audible, stops the state decaying into slow denormals
const FLUSH: f64 = 1e-20;

// low-pass biquad, so b1 = 2 * b0 and b2 = b0
#[derive(Clone, Copy)]
struct Coefficients {
    b0: f64,
    a1: f64,
    a2: f64,
}

// transposed direct form II
#[derive(Clone, Copy, Default)]
struct Delay {
    z1: f64,
    z2: f64,
}

impl Delay {
    fn next(&mut self, c: Coefficients, input: f64) -> f64 {
        let output = c.b0 * input + self.z1;
        self.z1 = 2.0 * c.b0 * input - c.a1 * output + self.z2;
        self.z2 = c.b0 * input - c.a2 * output;
        if self.z1.abs() < FLUSH && self.z2.abs() < FLUSH {
            *self = Delay::default();
        }
        return output;
    }
}

/// The "LED" filter, switched by the power LED bit. 2-pole low-pass at about 3.1kHz with a Q of
/// 0.66, so nearly Butterworth.
pub struct LedFilter {
    on: bool,
    coefficients: Coefficients,
    // one per output channel
    delays: Vec<Delay>,
}

impl LedFilter {
    // bilinear transform, prewarped so the cutoff lands where it should
    pub fn new(sample_rate: u32) -> Self {
        let rc = (R1 * R2 * C1 * C2).sqrt();
        let cutoff = 1.0 / (2.0 * PI * rc);
        let q = rc / (C2 * (R1 + R2));
        let w0 = 2.0 * PI * cutoff / sample_rate as f64;
        let alpha = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        return Self {
            on: false,
            coefficients: Coefficients {
                b0: (1.0 - w0.cos()) / 2.0 / a0,
                a1: -2.0 * w0.cos() / a0,
                a2: (1.0 - alpha) / a0,
            },
            delays: Vec::new(),
        };
    }

    pub fn set_on(&mut self, on: bool) {
        self.on = on;
    }

    // runs while off too, so turning it on doesn't click
    pub fn process(&mut self, buffer: &mut [f32], channels: usize) {
        self.delays.resize(channels, Delay::default());
        for frame in buffer.chunks_exact_mut(channels) {
            for (sample, delay) in frame.iter_mut().zip(&mut self.delays) {
                let filtered = delay.next(self.coefficients, *sample as f64);
                if self.on {
                    *sample = filtered as f32;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_RATE: u32 = 48_000;

    // 10k * sqrt(6800p * 3900p)
    const CUTOFF: f64 = 3090.5;

    fn filter() -> LedFilter {
        let mut filter = LedFilter::new(SAMPLE_RATE);
        filter.set_on(true);
        return filter;
    }

    // peak of a sine once the filter has settled, in dB
    fn gain(frequency: f64) -> f64 {
        let mut buffer: Vec<f32> = (0..SAMPLE_RATE as usize)
            .map(|i| (2.0 * PI * frequency * i as f64 / SAMPLE_RATE as f64).sin() as f32)
            .collect();
        filter().process(&mut buffer, 1);
        let peak = buffer[buffer.len() / 2..]
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        return 20.0 * (peak as f64).log10();
    }

    #[test]
    fn passes_dc() {
        let mut buffer = vec![0.5; 4800];
        filter().process(&mut buffer, 1);
        assert!((buffer[4799] - 0.5).abs() < 1e-4, "{}", buffer[4799]);
    }

    #[test]
    fn response() {
        assert!(gain(100.0) > -0.1);
        // a 2-pole low-pass is down to Q at its cutoff, 20 * log10(0.66)
        let at_cutoff = gain(CUTOFF);
        assert!((at_cutoff + 3.6).abs() < 0.1, "{at_cutoff}");
        // 12dB an octave past it
        let two_octaves = gain(CUTOFF * 4.0);
        assert!(two_octaves < -22.0, "{two_octaves}");
    }

    #[test]
    fn off_is_untouched() {
        let mut filter = LedFilter::new(SAMPLE_RATE);
        let input: Vec<f32> = (0..64)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let mut buffer = input.clone();
        filter.process(&mut buffer, 1);
        assert_eq!(buffer, input);
    }

    #[test]
    fn channels_separate() {
        let mut buffer: Vec<f32> = (0..200).flat_map(|_| [1.0, 0.0]).collect();
        filter().process(&mut buffer, 2);
        assert!(buffer.chunks(2).all(|frame| frame[1] == 0.0));
        assert!(buffer[0] > 0.0);
    }

    #[test]
    fn decays_to_zero() {
        let mut filter = filter();
        let mut buffer = vec![0.0; 48_000];
        buffer[0] = 1.0;
        filter.process(&mut buffer, 1);
        let delay = filter.delays[0];
        assert!(delay.z1 == 0.0 && delay.z2 == 0.0);
    }
}
