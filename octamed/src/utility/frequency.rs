use crate::utility::{ amiga::PAL_HERTZ, period::AmigaPalPeriod };

#[derive(Clone, Copy)]
pub struct Frequency(f32);

impl Frequency {
    pub fn hertz(hz: f32) -> Self {
        return Self(hz);
    }
    pub fn period(period: AmigaPalPeriod) -> Self {
        return period.get_frequency();
    }
    pub fn as_hertz(&self) -> f32 {
        return self.0;
    }
    pub fn as_period(&self) -> AmigaPalPeriod {
        let period = PAL_HERTZ / (2.0 * self.0);
        return AmigaPalPeriod::new(period.round() as u16);
    }
}
