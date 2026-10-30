use octamed::mmd::module::{OctamedMMD0Color, OctamedMMD0ColorPallete, PALETTE_COLORS};

pub const PALETTE_SIZE: usize = PALETTE_COLORS;

/// A channel's brightest level; each has 16.
pub const CHANNEL_MAX: u8 = 15;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Red,
    Green,
    Blue,
}

impl Channel {
    pub const ALL: [Channel; 3] = [Channel::Red, Channel::Green, Channel::Blue];

    /// Where the channel's 4 bits are in a colour.
    fn shift(self) -> u16 {
        return match self {
            Channel::Red => 8,
            Channel::Green => 4,
            Channel::Blue => 0,
        };
    }
}

/// The screen's colours, as the Amiga's 12-bit `0RGB` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OctamedColorPallete {
    colors: [u16; PALETTE_SIZE],
}

impl OctamedColorPallete {
    /// OctaMED v4's own colours.
    pub const DEFAULT: Self = Self {
        colors: [0x000, 0xCCC, 0x667, 0x99A, 0xF00, 0xF88, 0xFAA, 0xFCC],
    };

    pub fn from_octamed(pallete: &OctamedMMD0ColorPallete) -> Self {
        return Self {
            colors: pallete
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

    /// 8 bits per channel, each 4-bit level spread evenly over 0-255.
    pub fn rgb(&self, pen: usize) -> [u8; 3] {
        return OctamedMMD0Color::rgb(self.get(pen));
    }
}

impl Default for OctamedColorPallete {
    fn default() -> Self {
        return Self::DEFAULT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_are_read_and_set_alone() {
        let mut pallete = OctamedColorPallete::DEFAULT;
        // pen 2 is 0x667
        assert_eq!(pallete.channel(2, Channel::Blue), 7);
        pallete.set_channel(2, Channel::Green, 0xA);
        assert_eq!(pallete.get(2), 0x6A7);
        pallete.set_channel(2, Channel::Red, 99);
        assert_eq!(pallete.get(2), 0xFA7);
    }

    #[test]
    fn levels_spread_over_the_full_range() {
        let pallete = OctamedColorPallete::DEFAULT;
        assert_eq!(pallete.rgb(0), [0, 0, 0]);
        assert_eq!(pallete.rgb(3), [0x99, 0x99, 0xAA]);
        assert_eq!(pallete.rgb(4), [0xFF, 0, 0]);
    }

    #[test]
    fn colours_keep_to_12_bits() {
        let mut pallete = OctamedColorPallete::DEFAULT;
        pallete.set(1, 0xF123);
        assert_eq!(pallete.get(1), 0x123);
    }
}
