use std::time::{Duration, Instant};

use ggez::input::keyboard::KeyCode;

/// Holds back the system's key repeats until a key's been held for the machine's repeat delay.
/// After that they come at the system's rate.
#[derive(Default)]
pub struct KeyRepeat {
    held: Option<(KeyCode, Instant)>,
}

impl KeyRepeat {
    pub fn pressed(&mut self, key: KeyCode, now: Instant) {
        self.held = Some((key, now));
    }

    pub fn released(&mut self, key: KeyCode) {
        if self.held.is_some_and(|(held, _)| held == key) {
            self.held = None;
        }
    }

    pub fn allows_repeat(&self, key: KeyCode, now: Instant, delay: Duration) -> bool {
        return self
            .held
            .is_some_and(|(held, since)| held == key && now.duration_since(since) >= delay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DELAY: Duration = Duration::from_millis(400);

    #[test]
    fn repeats_only_after_the_delay() {
        let start = Instant::now();
        let mut repeat = KeyRepeat::default();
        repeat.pressed(KeyCode::Z, start);
        assert!(!repeat.allows_repeat(KeyCode::Z, start + Duration::from_millis(100), DELAY));
        assert!(repeat.allows_repeat(KeyCode::Z, start + DELAY, DELAY));
    }

    #[test]
    fn only_the_held_key() {
        let start = Instant::now();
        let mut repeat = KeyRepeat::default();
        repeat.pressed(KeyCode::Z, start);
        assert!(!repeat.allows_repeat(KeyCode::X, start + DELAY, DELAY));
        repeat.pressed(KeyCode::X, start + DELAY);
        assert!(!repeat.allows_repeat(KeyCode::X, start + DELAY, DELAY));
    }

    #[test]
    fn released_keys_dont_repeat() {
        let start = Instant::now();
        let mut repeat = KeyRepeat::default();
        repeat.pressed(KeyCode::Z, start);
        repeat.released(KeyCode::X);
        assert!(repeat.allows_repeat(KeyCode::Z, start + DELAY, DELAY));
        repeat.released(KeyCode::Z);
        assert!(!repeat.allows_repeat(KeyCode::Z, start + DELAY, DELAY));
    }
}
