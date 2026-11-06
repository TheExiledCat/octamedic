#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CommandId(pub u8);

/// Sets the track's volume.
pub const SET_VOLUME: u8 = 0x0C;
