use ggez::input::keyboard::KeyCode;
use octamedic_core::data::instrument::InstrumentId;

use crate::app::state::{AppState, MAX_INSTRUMENT};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeypadMap {
    #[default]
    TrackOnOff,
    SelectInstrument,
}

// the Amiga keypad's ( and ) have no key on a PC keypad, so what they do on the real thing is left
// out: tracks 0 and 1 and instruments 1 and 2 without MAP2, tracks A and B with it. MAP2 has no
// keys for tracks E and F on the real thing either
const TRACKS: [(KeyCode, usize); 14] = [
    (KeyCode::NumpadDivide, 0x2),
    (KeyCode::NumpadMultiply, 0x3),
    (KeyCode::Numpad7, 0x4),
    (KeyCode::Numpad8, 0x5),
    (KeyCode::Numpad9, 0x6),
    (KeyCode::NumpadSubtract, 0x7),
    (KeyCode::Numpad4, 0x8),
    (KeyCode::Numpad5, 0x9),
    (KeyCode::Numpad6, 0xA),
    (KeyCode::NumpadAdd, 0xB),
    (KeyCode::Numpad1, 0xC),
    (KeyCode::Numpad2, 0xD),
    (KeyCode::Numpad3, 0xE),
    (KeyCode::NumpadDecimal, 0xF),
];
const TRACKS_MAP2: [(KeyCode, usize); 12] = [
    (KeyCode::NumpadDivide, 0xC),
    (KeyCode::NumpadMultiply, 0xD),
    (KeyCode::Numpad0, 0x0),
    (KeyCode::Numpad1, 0x1),
    (KeyCode::Numpad2, 0x2),
    (KeyCode::Numpad3, 0x3),
    (KeyCode::Numpad4, 0x4),
    (KeyCode::Numpad5, 0x5),
    (KeyCode::Numpad6, 0x6),
    (KeyCode::Numpad7, 0x7),
    (KeyCode::Numpad8, 0x8),
    (KeyCode::Numpad9, 0x9),
];
const DIGITS: [KeyCode; 10] = [
    KeyCode::Numpad0,
    KeyCode::Numpad1,
    KeyCode::Numpad2,
    KeyCode::Numpad3,
    KeyCode::Numpad4,
    KeyCode::Numpad5,
    KeyCode::Numpad6,
    KeyCode::Numpad7,
    KeyCode::Numpad8,
    KeyCode::Numpad9,
];
const TENS: u8 = 10;

pub fn is_keypad(key: KeyCode) -> bool {
    let others = [
        KeyCode::NumpadDecimal,
        KeyCode::NumpadAdd,
        KeyCode::NumpadSubtract,
        KeyCode::NumpadMultiply,
        KeyCode::NumpadDivide,
    ];
    return DIGITS.contains(&key) || others.contains(&key);
}

impl AppState {
    /// The keypad on its own, as the EDIT panel's KEYPAD and MAP2 say. Returns whether it did
    /// anything with `key`.
    pub fn keypad_key(&mut self, key: KeyCode) -> bool {
        return match (self.keypad_map, self.keypad_map2) {
            (KeypadMap::TrackOnOff, false) => self.keypad_track(&TRACKS, key),
            (KeypadMap::TrackOnOff, true) => self.keypad_track_map2(key),
            (KeypadMap::SelectInstrument, false) => self.keypad_instrument(key),
            (KeypadMap::SelectInstrument, true) => self.keypad_instrument_map2(key),
        };
    }

    fn keypad_track(&mut self, map: &[(KeyCode, usize)], key: KeyCode) -> bool {
        let Some(&(_, track)) = map.iter().find(|(code, _)| *code == key) else {
            return false;
        };
        self.toggle_track_mute(track);
        return true;
    }

    fn keypad_track_map2(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::NumpadDecimal => self.set_all_tracks_muted(true),
            KeyCode::NumpadEnter => self.set_all_tracks_muted(false),
            _ => return self.keypad_track(&TRACKS_MAP2, key),
        }
        return true;
    }

    // TRACKS' keys, each picking the instrument one more than its track
    fn keypad_instrument(&mut self, key: KeyCode) -> bool {
        let Some(&(_, track)) = TRACKS.iter().find(|(code, _)| *code == key) else {
            return false;
        };
        self.set_instrument(track as u8 + 1);
        return true;
    }

    // a digit picks within the instrument's ten, . moves ten on, or back when that's past the end
    fn keypad_instrument_map2(&mut self, key: KeyCode) -> bool {
        let current = self.active_instrument.0;
        if let Some(digit) = DIGITS.iter().position(|&code| code == key) {
            self.set_instrument(current / TENS * TENS + digit as u8);
            return true;
        }
        match key {
            KeyCode::NumpadDecimal if current + TENS <= MAX_INSTRUMENT => {
                self.set_instrument(current + TENS);
            }
            KeyCode::NumpadDecimal => {
                self.set_instrument(current.saturating_sub(TENS));
            }
            KeyCode::NumpadAdd => {
                self.set_instrument(current.saturating_add(1));
            }
            KeyCode::NumpadSubtract => {
                self.set_instrument(current.saturating_sub(1));
            }
            KeyCode::NumpadMultiply => {
                if let Some(nearest) = self.instrument_nearest_cursor() {
                    self.active_instrument = nearest;
                }
            }
            _ => return false,
        }
        return true;
    }

    // the nearest one above the cursor in its track, or below when there's none above
    fn instrument_nearest_cursor(&self) -> Option<InstrumentId> {
        let pattern = self.active_pattern()?;
        let (line, track) = (self.cursor.line, self.cursor.track);
        let at = |line: usize| {
            pattern
                .track(line, track)
                .map(|cell| cell.instrument_id)
                .filter(|instrument| instrument.0 != 0)
        };
        let above = (0..=line).rev().find_map(at);
        return above.or_else(|| (line + 1..pattern.line_count()).find_map(at));
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;

    fn state() -> AppState {
        return AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
    }

    #[test]
    fn track_on_off_toggles_tracks() {
        let mut state = state();
        assert!(state.keypad_key(KeyCode::Numpad7));
        assert!(state.is_track_muted(4));
        state.keypad_key(KeyCode::Numpad7);
        assert!(!state.is_track_muted(4));
        state.keypad_key(KeyCode::NumpadDecimal);
        assert!(state.is_track_muted(0xF));
        assert!(!state.keypad_key(KeyCode::Numpad0));
    }

    #[test]
    fn map2_has_digits_for_tracks_and_all_off_and_on() {
        let mut state = state();
        state.keypad_map2 = true;
        state.keypad_key(KeyCode::Numpad3);
        assert!(state.is_track_muted(3));
        state.keypad_key(KeyCode::NumpadDecimal);
        assert!((0..16).all(|track| state.is_track_muted(track)));
        state.keypad_key(KeyCode::NumpadEnter);
        assert!((0..16).all(|track| !state.is_track_muted(track)));
    }

    #[test]
    fn sel_instr_picks_instruments() {
        let mut state = state();
        state.keypad_map = KeypadMap::SelectInstrument;
        state.keypad_key(KeyCode::NumpadDecimal);
        assert_eq!(state.active_instrument, InstrumentId(16));
        state.keypad_key(KeyCode::Numpad5);
        assert_eq!(state.active_instrument, InstrumentId(10));
        assert!(!state.keypad_key(KeyCode::NumpadEnter));
    }

    #[test]
    fn sel_instr_map2_picks_by_digit_and_ten() {
        let mut state = state();
        state.keypad_map = KeypadMap::SelectInstrument;
        state.keypad_map2 = true;
        state.keypad_key(KeyCode::Numpad7);
        assert_eq!(state.active_instrument, InstrumentId(7));
        state.keypad_key(KeyCode::NumpadDecimal);
        assert_eq!(state.active_instrument, InstrumentId(17));
        state.keypad_key(KeyCode::NumpadAdd);
        assert_eq!(state.active_instrument, InstrumentId(18));
        state.keypad_key(KeyCode::Numpad2);
        assert_eq!(state.active_instrument, InstrumentId(12));
        state.active_pattern_mut().unwrap().lines[0].tracks[0].instrument_id = InstrumentId(5);
        state.cursor.line = 9;
        state.keypad_key(KeyCode::NumpadMultiply);
        assert_eq!(state.active_instrument, InstrumentId(5));
    }
}
