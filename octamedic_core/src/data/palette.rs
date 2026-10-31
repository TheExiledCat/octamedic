use octamed::mmd::module::{OctamedMMD0Color, OctamedMMD0ColorPalette, PALETTE_COLORS};

pub const PALETTE_SIZE: usize = PALETTE_COLORS;

pub const CHANNEL_MAX: u8 = 15;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Red,
    Green,
    Blue,
}

impl Channel {
    pub const ALL: [Channel; 3] = [Channel::Red, Channel::Green, Channel::Blue];

    fn shift(self) -> u16 {
        return match self {
            Channel::Red => 8,
            Channel::Green => 4,
            Channel::Blue => 0,
        };
    }
}

/// Screen colours as 12-bit `0RGB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OctamedColorPalette {
    colors: [u16; PALETTE_SIZE],
}

impl OctamedColorPalette {
    // v4's default colours
    pub const DEFAULT: Self = Self {
        colors: [0x000, 0xCCC, 0x667, 0x99A, 0xF00, 0xF88, 0xFAA, 0xFCC],
    };

    pub fn from_octamed(palette: &OctamedMMD0ColorPalette) -> Self {
        return Self {
            colors: palette
                .colors
                .map(|color| color.value.0 & OctamedMMD0Color::MASK),
        };
    }

    pub fn get(&self, pen: usize) -> u16 {
        return self.colors[pen % PALETTE_SIZE];
    }

    pub fn set(&mut self, pen: usize, color: u16) {
        self.colors[pen % PALETTE_SIZE] = color & OctamedMMD0Color::MASK;
    }

    pub fn channel(&self, pen: usize, channel: Channel) -> u8 {
        return ((self.get(pen) >> channel.shift()) & CHANNEL_MAX as u16) as u8;
    }

    pub fn set_channel(&mut self, pen: usize, channel: Channel, level: u8) {
        let mask = (CHANNEL_MAX as u16) << channel.shift();
        let level = (level.min(CHANNEL_MAX) as u16) << channel.shift();
        self.set(pen, (self.get(pen) & !mask) | level);
    }

    pub fn rgb(&self, pen: usize) -> [u8; 3] {
        return OctamedMMD0Color::rgb(self.get(pen));
    }
}

impl Default for OctamedColorPalette {
    fn default() -> Self {
        return Self::DEFAULT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_channel() {
        let mut palette = OctamedColorPalette::DEFAULT;
        // pen 2 is 0x667
        assert_eq!(palette.channel(2, Channel::Blue), 7);
        palette.set_channel(2, Channel::Green, 0xA);
        assert_eq!(palette.get(2), 0x6A7);
        palette.set_channel(2, Channel::Red, 99);
        assert_eq!(palette.get(2), 0xFA7);
    }

    #[test]
    fn rgb_range() {
        let palette = OctamedColorPalette::DEFAULT;
        assert_eq!(palette.rgb(0), [0, 0, 0]);
        assert_eq!(palette.rgb(3), [0x99, 0x99, 0xAA]);
        assert_eq!(palette.rgb(4), [0xFF, 0, 0]);
    }

    #[test]
    fn twelve_bits() {
        let mut palette = OctamedColorPalette::DEFAULT;
        palette.set(1, 0xF123);
        assert_eq!(palette.get(1), 0x123);
    }
}
