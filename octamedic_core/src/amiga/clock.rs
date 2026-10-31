#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmigaClock {
    /// CIA timer clock, cpu clock / 10
    pub e_clock: u32,
    /// bytes per second = paula_clock / period
    pub paula_clock: u32,
    pub frame_rate: u32,
}

impl AmigaClock {
    pub const PAL: Self = Self {
        e_clock: 709_379,
        paula_clock: 3_546_895,
        frame_rate: 50,
    };

    pub const NTSC: Self = Self {
        e_clock: 715_909,
        paula_clock: 3_579_545,
        frame_rate: 60,
    };
}
