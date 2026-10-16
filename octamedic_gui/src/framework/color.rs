use ggez::graphics::Color;

pub trait IntoColorHex {
    fn as_hex(&self) -> [u8; 4];
}

pub trait FromHex {
    fn hex(hex: impl IntoColorHex) -> Self
    where
        Self: Sized;
}

impl<T: ToString> IntoColorHex for T {
    fn as_hex(&self) -> [u8; 4] {

        let str = self.to_string();

        let count = str.chars().count();

        match count {
            3 | 4 => {

                let byte1 = u8::from_str_radix(&str[0..1], 16).unwrap() * 17;

                let byte2 = u8::from_str_radix(&str[1..2], 16).unwrap() * 17;

                let byte3 = u8::from_str_radix(&str[2..3], 16).unwrap() * 17;

                let byte4 = if count == 4 {

                    u8::from_str_radix(&str[3..4], 16).unwrap() * 17
                } else {

                    u8::MAX
                };

                [byte1, byte2, byte3, byte4]
            }

            6 | 8 => {

                let byte1 = u8::from_str_radix(&str[0..2], 16).unwrap();

                let byte2 = u8::from_str_radix(&str[2..4], 16).unwrap();

                let byte3 = u8::from_str_radix(&str[4..6], 16).unwrap();

                let byte4 = if count == 8 {

                    u8::from_str_radix(&str[6..8], 16).unwrap()
                } else {

                    u8::MAX
                };

                [byte1, byte2, byte3, byte4]
            }
            _ => panic!("invalid hex string"),
        }
    }
}

impl FromHex for Color {
    fn hex(hex: impl IntoColorHex) -> Self {

        let hex = hex.as_hex();

        return Self::from_rgba(hex[0], hex[1], hex[2], hex[3]);
    }
}
