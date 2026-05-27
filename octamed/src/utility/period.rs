use crate::utility::{amiga::PAL_HERTZ, frequency::Frequency};

#[derive(Clone, Copy)]

pub struct AmigaPalPeriod(u16);

impl AmigaPalPeriod {
    pub fn new(period: u16) -> Self {

        return Self(period);
    }

    /// returns the equivelant sample rate for a period in Hz

    pub fn get_frequency(&self) -> Frequency {

        return Frequency::hertz(PAL_HERTZ / ((self.0 * 2) as f32));
    }
}
