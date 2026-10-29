use octamed::mmd::module::OctamedMMD0ColorPallete;

pub const PALETTE_SIZE: usize = 8;

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
            colors: pallete.colors.map(|color| color.value.0 & 0x0FFF),
        };
    }

    pub fn get(&self, pen: usize) -> u16 {
        return self.colors[pen % PALETTE_SIZE];
    }

    pub fn set(&mut self, pen: usize, color: u16) {
        self.colors[pen % PALETTE_SIZE] = color & 0x0FFF;
    }

    /// 8 bits per channel, each 4-bit level spread evenly over 0-255.
    pub fn rgb(&self, pen: usize) -> [u8; 3] {
        let color = self.get(pen);
        let level = |shift: u16| ((color >> shift) & 0xF) as u8 * 17;
        return [level(8), level(4), level(0)];
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
