pub const MAX_VOLUME: u8 = 64;

/// The volume a volume command's argument sets: two decimal digits, or hex with VOL's HEX on. A decimal
/// argument with a hex digit still counts it as ten or more, like OctaMED. Never above 64.
pub fn set_volume_value(argument: u8, hex: bool) -> u8 {
    let volume = if hex {
        argument
    } else {
        (argument >> 4) * 10 + (argument & 0x0F)
    };
    return volume.min(MAX_VOLUME);
}

/// The volume command argument for `volume`, the other way round from [`set_volume_value`].
pub fn set_volume_argument(volume: u8, hex: bool) -> u8 {
    let volume = volume.min(MAX_VOLUME);
    if hex {
        return volume;
    }
    return ((volume / 10) << 4) | (volume % 10);
}

#[derive(Clone, Copy, Debug)]
pub struct OctamedicVolume {
    value: u8,
}

impl OctamedicVolume {
    pub fn new(value: u8) -> Self {
        let mut s = Self { value: 0 };
        s.set(value);
        return s;
    }

    pub fn get(&self) -> u8 {
        return self.value;
    }

    pub fn set(&mut self, value: u8) {
        self.value = value.min(MAX_VOLUME);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_volume_arguments() {
        assert_eq!(set_volume_value(0x20, false), 20);
        assert_eq!(set_volume_value(0x20, true), 32);
        assert_eq!(set_volume_value(0x99, false), MAX_VOLUME);
        assert_eq!(set_volume_argument(20, false), 0x20);
        assert_eq!(set_volume_argument(32, true), 0x20);
        assert_eq!(set_volume_argument(64, false), 0x64);
        for volume in 0..=MAX_VOLUME {
            for hex in [false, true] {
                assert_eq!(
                    set_volume_value(set_volume_argument(volume, hex), hex),
                    volume
                );
            }
        }
    }
}
