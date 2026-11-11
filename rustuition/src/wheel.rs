/// For handlers that scroll by lines, each keeping its own. Wheel delta to whole lines: 1.0 per
/// line, fraction carried, capped at one line per event since ggez, for one, doesn't separate line
/// deltas (wheels) from pixel deltas (trackpads).
#[derive(Default)]
pub struct WheelLines {
    saved: f32,
}

impl WheelLines {
    /// `y` is [`PointerEvent::wheel`](crate::event::PointerEvent::wheel)'s, positive away from you.
    /// The lines are positive towards the end of a list.
    pub fn lines(&mut self, y: f32) -> i32 {
        // turning back starts over
        if y * self.saved < 0.0 {
            self.saved = 0.0;
        }
        let total = self.saved + y;
        let up = total.trunc().clamp(-1.0, 1.0);
        self.saved = if up == 0.0 { total } else { total.fract() };
        return -(up as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(steps: &[f32]) -> Vec<i32> {
        let mut wheel = WheelLines::default();
        return steps.iter().map(|&y| wheel.lines(y)).collect();
    }

    #[test]
    fn a_notch_is_a_line() {
        assert_eq!(lines(&[-1.0, -1.0, 1.0]), vec![1, 1, -1]);
    }

    #[test]
    fn fractions_add_up() {
        assert_eq!(lines(&[-0.4, -0.4, -0.4]), vec![0, 0, 1]);
    }

    #[test]
    fn at_most_a_line_an_event() {
        assert_eq!(lines(&[-30.0, -12.5]), vec![1, 1]);
    }

    #[test]
    fn whats_left_of_a_big_step_carries_over() {
        // the 0.5 left after the first line makes the 0.5 a whole one
        assert_eq!(lines(&[-1.5, -0.5]), vec![1, 1]);
    }

    #[test]
    fn turning_back_drops_whats_saved() {
        assert_eq!(lines(&[-0.8, 0.5, 0.5]), vec![0, 0, -1]);
    }
}
