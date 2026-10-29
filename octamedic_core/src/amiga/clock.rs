/// The clocks that differ between PAL and NTSC Amigas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AmigaClock {
    /// The CIA timers' clock: the CPU clock / 10.
    pub e_clock: u32,
    /// A Paula channel plays `paula_clock / period` bytes a second.
    pub paula_clock: u32,
}

impl AmigaClock {
    pub const PAL: Self = Self {
        e_clock: 709_379,
        paula_clock: 3_546_895,
    };

    pub const NTSC: Self = Self {
        e_clock: 715_909,
        paula_clock: 3_579_545,
    };
}
