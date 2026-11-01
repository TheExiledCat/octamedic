/// The highest level of a colour channel, 4 bits.
pub const CHANNEL_MAX: u8 = 15;

use crate::target::Rgba;

/// An index into the screen's palette, so what's drawn with it follows the palette when it changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pen(pub usize);

// what rustuition draws its own bevels and shadowed text with. an app's other pens are its own
impl Pen {
    /// Pen 0, the screen's background.
    pub const BACKGROUND: Self = Self(0);
    /// Pen 1, the lit edges of bevels.
    pub const SHINE: Self = Self(1);
    /// Pen 2, the shaded edges of bevels and text shadows.
    pub const SHADOW: Self = Self(2);
    /// Pen 3, gadget backgrounds.
    pub const FILL: Self = Self(3);
}

/// One of a colour's red, green and blue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Red,
    Green,
    Blue,
}

impl Channel {
    /// Red, green and blue, in that order.
    pub const ALL: [Channel; 3] = [Channel::Red, Channel::Green, Channel::Blue];

    fn shift(self) -> u16 {
        return match self {
            Channel::Red => 8,
            Channel::Green => 4,
            Channel::Blue => 0,
        };
    }
}

/// A 12-bit `0RGB` colour, 4 bits a channel, like the OCS/ECS colour registers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color(pub u16);

impl Color {
    /// The bits that are colour. The top 4 are ignored, and cut off when it goes in a palette.
    pub const MASK: u16 = 0x0fff;
    /// `0x000`.
    pub const BLACK: Self = Self(0x000);
    /// `0xFFF`.
    pub const WHITE: Self = Self(0xFFF);

    /// A colour from red, green and blue levels, 0 to 15 each. Higher levels are cut to 15.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        return Self(level(r) << 8 | level(g) << 4 | level(b));
    }

    /// One of its channels, 0 to 15.
    pub fn channel(self, channel: Channel) -> u8 {
        return ((self.0 >> channel.shift()) & CHANNEL_MAX as u16) as u8;
    }

    /// The same colour with one channel set to `level`, cut to 15.
    pub fn with_channel(self, channel: Channel, level: u8) -> Self {
        let mask = (CHANNEL_MAX as u16) << channel.shift();
        let level = (level.min(CHANNEL_MAX) as u16) << channel.shift();
        return Self((self.0 & !mask) | level);
    }

    /// As 8 bits a channel, for drawing. 4 bit levels are spread evenly, so 15 becomes 255.
    pub fn to_rgba(self) -> Rgba {
        let [r, g, b] = Channel::ALL.map(|channel| {
            (self.channel(channel) as u16 * u8::MAX as u16 / CHANNEL_MAX as u16) as u8
        });
        return Rgba::rgb(r, g, b);
    }

    fn masked(self) -> Self {
        return Self(self.0 & Self::MASK);
    }
}

impl From<Color> for Rgba {
    fn from(color: Color) -> Self {
        return color.to_rgba();
    }
}

// const so Color::rgb can be, which rules out min
const fn level(level: u8) -> u16 {
    return if level > CHANNEL_MAX {
        CHANNEL_MAX as u16
    } else {
        level as u16
    };
}

/// Screen colours, any number of them. Pens past the end wrap around.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IntuitionColorPalette {
    colors: Vec<Color>,
}

impl IntuitionColorPalette {
    /// A palette of `colors`, each cut to 12 bits.
    pub fn new(colors: impl Into<Vec<Color>>) -> Self {
        let colors = colors.into().into_iter().map(Color::masked).collect();
        return Self { colors };
    }

    /// How many colours it has.
    pub fn len(&self) -> usize {
        return self.colors.len();
    }

    /// Whether it has no colours.
    pub fn is_empty(&self) -> bool {
        return self.colors.is_empty();
    }

    /// Every colour, pen 0 first.
    pub fn colors(&self) -> &[Color] {
        return &self.colors;
    }

    /// The colour of `pen`, or black if the palette is empty.
    pub fn get(&self, pen: Pen) -> Color {
        return self
            .index(pen)
            .map_or(Color::BLACK, |index| self.colors[index]);
    }

    /// Sets the colour of `pen`, cut to 12 bits.
    pub fn set(&mut self, pen: Pen, color: Color) {
        if let Some(index) = self.index(pen) {
            self.colors[index] = color.masked();
        }
    }

    /// One channel of `pen`'s colour, 0 to 15.
    pub fn channel(&self, pen: Pen, channel: Channel) -> u8 {
        return self.get(pen).channel(channel);
    }

    /// Sets one channel of `pen`'s colour, 0 to 15.
    pub fn set_channel(&mut self, pen: Pen, channel: Channel, level: u8) {
        self.set(pen, self.get(pen).with_channel(channel, level));
    }

    fn index(&self, pen: Pen) -> Option<usize> {
        return pen.0.checked_rem(self.colors.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> IntuitionColorPalette {
        return IntuitionColorPalette::new([
            Color(0x000),
            Color(0xCCC),
            Color(0x667),
            Color(0x99A),
            Color(0xF00),
        ]);
    }

    #[test]
    fn set_channel() {
        let mut palette = palette();
        assert_eq!(palette.channel(Pen(2), Channel::Blue), 7);
        palette.set_channel(Pen(2), Channel::Green, 0xA);
        assert_eq!(palette.get(Pen(2)), Color(0x6A7));
        palette.set_channel(Pen(2), Channel::Red, 99);
        assert_eq!(palette.get(Pen(2)), Color(0xFA7));
    }

    #[test]
    fn rgba_range() {
        let palette = palette();
        assert_eq!(palette.get(Pen(0)).to_rgba(), Rgba::rgb(0, 0, 0));
        assert_eq!(palette.get(Pen(3)).to_rgba(), Rgba::rgb(0x99, 0x99, 0xAA));
        assert_eq!(Rgba::from(palette.get(Pen(4))), Rgba::rgb(0xFF, 0, 0));
    }

    #[test]
    fn twelve_bits() {
        let mut palette = IntuitionColorPalette::new([Color(0xF123), Color::BLACK]);
        assert_eq!(palette.get(Pen(0)), Color(0x123));
        palette.set(Pen(1), Color(0xF456));
        assert_eq!(palette.get(Pen(1)), Color(0x456));
    }

    #[test]
    fn from_levels() {
        assert_eq!(Color::rgb(0xA, 0xB, 0xC), Color(0xABC));
        assert_eq!(Color::rgb(99, 0, 0), Color(0xF00));
        assert_eq!(Color(0xF123).channel(Channel::Red), 1);
        assert_eq!(Color(0x123).with_channel(Channel::Blue, 0xF), Color(0x12F));
    }

    #[test]
    fn pens_wrap() {
        assert_eq!(palette().get(Pen(6)), Color(0xCCC));
        let mut empty = IntuitionColorPalette::default();
        empty.set(Pen(1), Color::WHITE);
        assert_eq!(empty.get(Pen(1)), Color::BLACK);
    }
}
