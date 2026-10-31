use ggez::graphics::Rect as WindowRect;

use crate::framework::rect::{Point, Rect};

// PAL hires
pub const SCREEN_WIDTH: i32 = 640;

pub const SCREEN_HEIGHT: i32 = 256;

pub const SCREEN: Rect = Rect::new(0, 0, SCREEN_WIDTH, SCREEN_HEIGHT);

// non-interlaced hires pixels are twice as tall as wide
pub const PIXEL_ASPECT: f32 = 2.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScaleMode {
    // falls back to Fit if the window is too small for 1x
    #[default]
    Integer,
    Fit,
    Stretch,
}

pub fn viewport(window: [f32; 2], mode: ScaleMode) -> WindowRect {
    let [width, height] = window;
    if mode == ScaleMode::Stretch {
        return WindowRect::new(0.0, 0.0, width, height);
    }
    let screen_width = SCREEN_WIDTH as f32;
    let shown_height = SCREEN_HEIGHT as f32 * PIXEL_ASPECT;
    let fit = (width / screen_width).min(height / shown_height);
    let scale = match mode {
        ScaleMode::Integer if fit >= 1.0 => fit.floor(),
        _ => fit,
    };
    let (w, h) = (screen_width * scale, shown_height * scale);
    return WindowRect::new(
        ((width - w) / 2.0).round(),
        ((height - h) / 2.0).round(),
        w,
        h,
    );
}

pub fn to_screen(viewport: WindowRect, point: [f32; 2]) -> Point {
    let x = (point[0] - viewport.x) / viewport.w * SCREEN_WIDTH as f32;
    let y = (point[1] - viewport.y) / viewport.h * SCREEN_HEIGHT as f32;
    return [
        (x.floor() as i32).clamp(0, SCREEN_WIDTH - 1),
        (y.floor() as i32).clamp(0, SCREEN_HEIGHT - 1),
    ];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_scaling() {
        let viewport = viewport([1920.0, 1080.0], ScaleMode::Integer);
        assert_eq!(viewport, WindowRect::new(320.0, 28.0, 1280.0, 1024.0));
    }

    #[test]
    fn integer_small_window() {
        let viewport = viewport([320.0, 512.0], ScaleMode::Integer);
        assert_eq!((viewport.w, viewport.h), (320.0, 256.0));
    }

    #[test]
    fn fit_keeps_aspect() {
        let viewport = viewport([1920.0, 1080.0], ScaleMode::Fit);
        assert_eq!((viewport.w, viewport.h), (1350.0, 1080.0));
    }

    #[test]
    fn stretch() {
        let viewport = viewport([1920.0, 1080.0], ScaleMode::Stretch);
        assert_eq!(viewport, WindowRect::new(0.0, 0.0, 1920.0, 1080.0));
    }

    #[test]
    fn window_to_screen() {
        let viewport = viewport([1280.0, 1024.0], ScaleMode::Integer);
        assert_eq!(to_screen(viewport, [0.0, 0.0]), [0, 0]);
        // 2x4 window pixels per screen pixel at this size
        assert_eq!(to_screen(viewport, [21.0, 43.0]), [10, 10]);
        assert_eq!(to_screen(viewport, [5000.0, -10.0]), [639, 0]);
    }
}
