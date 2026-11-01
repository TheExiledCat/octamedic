use crate::rect::{Point, Rect};

/// A screen's size in its own pixels, and how tall those are against their width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub width: i32,
    pub height: i32,
    pub pixel_aspect: f32,
}

impl Screen {
    /// 640x256 non-interlaced hires, pixels twice as tall as wide.
    pub const PAL_HIRES: Self = Self::new(640, 256, 2.0);
    /// 640x200 non-interlaced hires.
    pub const NTSC_HIRES: Self = Self::new(640, 200, 2.0);
    /// 320x256 lores, square pixels.
    pub const PAL_LORES: Self = Self::new(320, 256, 1.0);
    /// 320x200 lores.
    pub const NTSC_LORES: Self = Self::new(320, 200, 1.0);

    /// A screen `width` by `height` with pixels `pixel_aspect` times as tall as wide.
    pub const fn new(width: i32, height: i32, pixel_aspect: f32) -> Self {
        return Self {
            width,
            height,
            pixel_aspect,
        };
    }

    /// The whole screen as a rect.
    pub const fn rect(self) -> Rect {
        return Rect::new(0, 0, self.width, self.height);
    }

    /// The window size that shows it at `scale` times.
    pub fn window_size(self, scale: f32) -> [f32; 2] {
        let [width, height] = self.shown_size();
        return [width * scale, height * scale];
    }

    /// Where it goes in a window this size.
    pub fn viewport(self, window: [f32; 2], mode: ScaleMode) -> Viewport {
        let [width, height] = window;
        if mode == ScaleMode::Stretch {
            return Viewport::new(0.0, 0.0, width, height);
        }
        let [screen_width, shown_height] = self.shown_size();
        let fit = (width / screen_width).min(height / shown_height);
        let scale = match mode {
            ScaleMode::Integer if fit >= 1.0 => fit.floor(),
            _ => fit,
        };
        let (w, h) = (screen_width * scale, shown_height * scale);
        return Viewport::new(
            ((width - w) / 2.0).round(),
            ((height - h) / 2.0).round(),
            w,
            h,
        );
    }

    /// The screen pixel under a window position, kept on the screen.
    pub fn to_screen(self, viewport: Viewport, point: [f32; 2]) -> Point {
        let x = (point[0] - viewport.x) / viewport.w * self.width as f32;
        let y = (point[1] - viewport.y) / viewport.h * self.height as f32;
        return [
            (x.floor() as i32).clamp(0, self.width - 1),
            (y.floor() as i32).clamp(0, self.height - 1),
        ];
    }

    fn shown_size(self) -> [f32; 2] {
        return [self.width as f32, self.height as f32 * self.pixel_aspect];
    }
}

/// How the screen is scaled into the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScaleMode {
    // falls back to Fit if the window is too small for 1x
    #[default]
    Integer,
    Fit,
    Stretch,
}

/// Where the screen goes in the window, in window pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewport {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Viewport {
    /// A viewport at `x`, `y`, `w` by `h`.
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        return Self { x, y, w, h };
    }

    /// Whether a window position is on the screen.
    pub fn contains(self, point: [f32; 2]) -> bool {
        let [x, y] = point;
        return x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h;
    }
}

#[cfg(feature = "ggez")]
impl From<Viewport> for ggez::graphics::Rect {
    fn from(viewport: Viewport) -> Self {
        return Self::new(viewport.x, viewport.y, viewport.w, viewport.h);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCREEN: Screen = Screen::PAL_HIRES;

    #[test]
    fn integer_scaling() {
        let viewport = SCREEN.viewport([1920.0, 1080.0], ScaleMode::Integer);
        assert_eq!(viewport, Viewport::new(320.0, 28.0, 1280.0, 1024.0));
    }

    #[test]
    fn integer_small_window() {
        let viewport = SCREEN.viewport([320.0, 512.0], ScaleMode::Integer);
        assert_eq!((viewport.w, viewport.h), (320.0, 256.0));
    }

    #[test]
    fn fit_keeps_aspect() {
        let viewport = SCREEN.viewport([1920.0, 1080.0], ScaleMode::Fit);
        assert_eq!((viewport.w, viewport.h), (1350.0, 1080.0));
    }

    #[test]
    fn stretch() {
        let viewport = SCREEN.viewport([1920.0, 1080.0], ScaleMode::Stretch);
        assert_eq!(viewport, Viewport::new(0.0, 0.0, 1920.0, 1080.0));
    }

    #[test]
    fn window_to_screen() {
        let viewport = SCREEN.viewport([1280.0, 1024.0], ScaleMode::Integer);
        assert_eq!(SCREEN.to_screen(viewport, [0.0, 0.0]), [0, 0]);
        // 2x4 window pixels per screen pixel at this size
        assert_eq!(SCREEN.to_screen(viewport, [21.0, 43.0]), [10, 10]);
        assert_eq!(SCREEN.to_screen(viewport, [5000.0, -10.0]), [639, 0]);
    }

    #[test]
    fn lores_is_square() {
        assert_eq!(Screen::PAL_LORES.window_size(2.0), [640.0, 512.0]);
    }

    #[test]
    fn viewport_contains() {
        let viewport = Viewport::new(10.0, 10.0, 20.0, 20.0);
        assert!(viewport.contains([10.0, 29.9]));
        assert!(!viewport.contains([30.0, 15.0]));
    }
}
