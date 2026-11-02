use octamed::mmd::module::{
    MAX_INSTRUMENTS, NOTATION_SELECTED_TRACK_WORDS, OctamedMMD0NotationInfo, SONG_TRACKS,
};

pub const NOTATION_PRESETS: usize = NOTATION_SELECTED_TRACK_WORDS;

const FLATS: u8 = 1;
const THREE_FOUR: u8 = 2;

/// What the notation editor shows. It has five display presets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OctamedicNotation {
    /// 0 to 6, flats instead if `flats` is set.
    pub sharps: u8,
    pub flats: bool,
    /// 12 lines a bar instead of 16.
    pub three_four: bool,
    pub selected_tracks: [Option<u16>; NOTATION_PRESETS],
    /// Per track, per preset.
    pub shown_tracks: [[bool; NOTATION_PRESETS]; SONG_TRACKS],
    pub ghosted_tracks: [[bool; NOTATION_PRESETS]; SONG_TRACKS],
    /// -24 to 24, with bit 6 flipped when the instrument is hidden.
    pub instrument_transposes: [i8; MAX_INSTRUMENTS],
}

impl Default for OctamedicNotation {
    fn default() -> Self {
        return Self {
            sharps: 0,
            flats: false,
            three_four: false,
            selected_tracks: [None; NOTATION_PRESETS],
            shown_tracks: [[false; NOTATION_PRESETS]; SONG_TRACKS],
            ghosted_tracks: [[false; NOTATION_PRESETS]; SONG_TRACKS],
            instrument_transposes: [0; MAX_INSTRUMENTS],
        };
    }
}

impl OctamedicNotation {
    pub fn from_octamed(info: &OctamedMMD0NotationInfo) -> Self {
        let presets = |bits: u8| std::array::from_fn(|preset| bits & (1 << preset) != 0);
        return Self {
            sharps: info.sharp_count.0,
            flats: info.flags.0 & FLATS != 0,
            three_four: info.flags.0 & THREE_FOUR != 0,
            // -1 is none
            selected_tracks: info
                .selected_tracks
                .map(|track| (track.0 as i16 >= 0).then_some(track.0)),
            shown_tracks: info.shown_tracks.map(|bits| presets(bits.0)),
            ghosted_tracks: info.ghosted_tracks.map(|bits| presets(bits.0)),
            instrument_transposes: info.note_transposes.map(|transpose| transpose.0),
        };
    }
}

#[cfg(test)]
mod tests {
    use octamed::utility::bytes::{Byte, UByte, UWord};

    use super::*;

    #[test]
    fn from_octamed() {
        let mut shown_tracks = [UByte(0); SONG_TRACKS];
        shown_tracks[2] = UByte(0b00101);
        let info = OctamedMMD0NotationInfo {
            sharp_count: UByte(3),
            flags: UByte(FLATS),
            selected_tracks: [UWord(1), UWord(0xffff), UWord(0), UWord(0xffff), UWord(5)],
            shown_tracks,
            ghosted_tracks: [UByte(0); SONG_TRACKS],
            note_transposes: [Byte(-12); MAX_INSTRUMENTS],
            _pad: UByte(0),
        };
        let notation = OctamedicNotation::from_octamed(&info);
        assert_eq!(
            (notation.sharps, notation.flats, notation.three_four),
            (3, true, false)
        );
        assert_eq!(
            notation.selected_tracks,
            [Some(1), None, Some(0), None, Some(5)]
        );
        assert_eq!(notation.shown_tracks[2], [true, false, true, false, false]);
        assert_eq!(notation.instrument_transposes[0], -12);
    }
}
