use std::time::{Duration, Instant};

// how long after the last resize event the window counts as let go of
const SETTLE: Duration = Duration::from_millis(150);
// near enough, so a window manager that rounds the size differently isn't snapped forever
const SLACK: f32 = 2.0;

/// Keeps a windowed window the screen's shape, so the picture fills it with no bars. winit can't
/// hold a window to an aspect ratio while it's dragged, so it's put back in shape once the
/// resizing stops.
#[derive(Clone, Copy, Debug)]
pub struct WindowShape {
    size: [f32; 2],
    // the size before the resizing that's going on started, to tell which side was dragged
    drag_start: Option<[f32; 2]>,
    resized_at: Option<Instant>,
}

impl WindowShape {
    pub fn new(size: [f32; 2]) -> Self {
        return Self {
            size,
            drag_start: None,
            resized_at: None,
        };
    }

    /// Call on every resize with the new size, and when the screen changes shape.
    pub fn resized(&mut self, now: Instant, size: [f32; 2]) {
        if self.drag_start.is_none() {
            self.drag_start = Some(self.size);
        }
        self.size = size;
        self.resized_at = Some(now);
    }

    /// The size to give the window once a resize has settled, if it isn't in shape. `shape` is the
    /// screen's, `limit` the most there's room for, and a maximised or fullscreen window is left
    /// as the system made it.
    pub fn settle(
        &mut self,
        now: Instant,
        shape: [f32; 2],
        limit: Option<[f32; 2]>,
        free: bool,
    ) -> Option<[f32; 2]> {
        let resized_at = self.resized_at?;
        if now.duration_since(resized_at) < SETTLE {
            return None;
        }
        self.resized_at = None;
        let start = self.drag_start.take()?;
        if !free {
            return None;
        }
        let target = snapped(shape, start, self.size, limit);
        let [width, height] = self.size;
        let off = (target[0] - width).abs() > SLACK || (target[1] - height).abs() > SLACK;
        return off.then_some(target);
    }
}

// keeps the side that was dragged and fits the other to it. dragging a corner moves both, then
// the one that changed more wins. then it shrinks to fit `limit`
fn snapped(
    [shape_width, shape_height]: [f32; 2],
    [start_width, start_height]: [f32; 2],
    [width, height]: [f32; 2],
    limit: Option<[f32; 2]>,
) -> [f32; 2] {
    let ratio = shape_width / shape_height;
    let widened = (width - start_width).abs();
    let heightened = (height - start_height).abs();
    let follow_width = match (widened >= 1.0, heightened >= 1.0) {
        (true, false) => true,
        (false, true) => false,
        _ => widened / start_width.max(1.0) > heightened / start_height.max(1.0),
    };
    let [mut width, mut height] = if follow_width {
        [width, width / ratio]
    } else {
        [height * ratio, height]
    };
    if let Some([limit_width, limit_height]) = limit {
        let fit = (limit_width / width).min(limit_height / height).min(1.0);
        width *= fit;
        height *= fit;
    }
    return [width.round(), height.round()];
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPE: [f32; 2] = [640.0, 512.0];
    const START: [f32; 2] = [1280.0, 1024.0];

    #[test]
    fn keeps_the_side_that_was_dragged() {
        // only the width, however little
        assert_eq!(
            snapped(SHAPE, START, [1300.0, 1024.0], None),
            [1300.0, 1040.0]
        );
        // only the height
        assert_eq!(
            snapped(SHAPE, START, [1280.0, 800.0], None),
            [1000.0, 800.0]
        );
        // a corner, the height changing more
        assert_eq!(
            snapped(SHAPE, START, [1290.0, 800.0], None),
            [1000.0, 800.0]
        );
    }

    #[test]
    fn stays_inside_the_limit() {
        let snapped = snapped(SHAPE, START, [1900.0, 1024.0], Some([1920.0, 1000.0]));
        assert_eq!(snapped, [1250.0, 1000.0]);
    }

    #[test]
    fn waits_for_the_resizing_to_stop() {
        let start = Instant::now();
        let mut shape = WindowShape::new(START);
        assert_eq!(shape.settle(start, SHAPE, None, true), None);
        // a drag in steps is measured from where it started
        shape.resized(start, [1400.0, 1024.0]);
        shape.resized(start, [1500.0, 1024.0]);
        assert_eq!(shape.settle(start, SHAPE, None, true), None);
        let later = start + SETTLE;
        assert_eq!(
            shape.settle(later, SHAPE, None, true),
            Some([1500.0, 1200.0])
        );
        // the resize that puts it in shape changes nothing, a pixel out or not
        shape.resized(later, [1500.0, 1199.0]);
        assert_eq!(shape.settle(later + SETTLE, SHAPE, None, true), None);
    }

    #[test]
    fn leaves_maximised_windows_alone() {
        let start = Instant::now();
        let mut shape = WindowShape::new(START);
        shape.resized(start, [1920.0, 1040.0]);
        assert_eq!(shape.settle(start + SETTLE, SHAPE, None, false), None);
        // and coming back from it starts from there
        shape.resized(start, [1280.0, 1000.0]);
        let settled = shape.settle(start + SETTLE, SHAPE, None, true);
        assert_eq!(settled, Some([1280.0, 1024.0]));
    }
}
