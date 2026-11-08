#[cfg(feature = "ggez")]
pub use batched_canvas::BatchedCanvas;

use crate::rect::{Point, Rect};

/// An 8-bit RGBA colour, what render targets fill with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    /// Alpha for fully opaque.
    pub const OPAQUE: u8 = u8::MAX;

    /// An opaque colour.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        return Self {
            r,
            g,
            b,
            a: Self::OPAQUE,
        };
    }
}

/// Something a [`Painter`](crate::painter::Painter) can draw on. It only needs to fill and clip rects.
pub trait RenderTarget {
    /// Fills `rect` within the clip, blending by alpha.
    fn fill(&mut self, rect: Rect, color: Rgba);

    /// The rect drawing is clipped to.
    fn get_clip(&self) -> Rect;

    /// Clips drawing to `rect`. False if it's entirely off the target, and the clip stays as it was.
    fn set_clip(&mut self, rect: Rect) -> bool;
}

/// RGB pixels in memory, row by row, for when you want the pixels yourself rather than a window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Framebuffer {
    width: i32,
    height: i32,
    pixels: Vec<[u8; 3]>,
    clip: Rect,
}

impl Framebuffer {
    /// A black framebuffer `width` by `height`.
    pub fn new(width: i32, height: i32) -> Self {
        let (width, height) = (width.max(0), height.max(0));
        return Self {
            width,
            height,
            pixels: vec![[0; 3]; (width * height) as usize],
            clip: Rect::new(0, 0, width, height),
        };
    }

    /// Its width in pixels.
    pub fn width(&self) -> i32 {
        return self.width;
    }

    /// Its height in pixels.
    pub fn height(&self) -> i32 {
        return self.height;
    }

    /// The whole framebuffer as a rect.
    pub fn bounds(&self) -> Rect {
        return Rect::new(0, 0, self.width, self.height);
    }

    /// The pixels row by row from the top left.
    pub fn pixels(&self) -> &[[u8; 3]] {
        return &self.pixels;
    }

    /// The pixels to change directly, row by row from the top left.
    pub fn pixels_mut(&mut self) -> &mut [[u8; 3]] {
        return &mut self.pixels;
    }

    /// Row `y`'s pixels to change directly, or `None` off the edge.
    pub fn row_mut(&mut self, y: i32) -> Option<&mut [[u8; 3]]> {
        if y < 0 || y >= self.height {
            return None;
        }
        let start = (y * self.width) as usize;
        return Some(&mut self.pixels[start..start + self.width as usize]);
    }

    /// The pixels as bytes: r, g, b, r, g, b...
    pub fn as_bytes(&self) -> &[u8] {
        return self.pixels.as_flattened();
    }

    /// A copy `factor` times as wide and tall, each pixel a `factor` by `factor` block, so filters
    /// that move pixels around have finer ones to work with.
    pub fn scaled(&self, factor: i32) -> Self {
        let factor = factor.max(1);
        let mut scaled = Self::new(self.width * factor, self.height * factor);
        for (y, row) in scaled
            .pixels
            .chunks_mut(scaled.width.max(1) as usize)
            .enumerate()
        {
            let from = (y as i32 / factor * self.width) as usize;
            for (x, pixel) in row.iter_mut().enumerate() {
                *pixel = self.pixels[from + x / factor as usize];
            }
        }
        return scaled;
    }

    /// The pixels as r, g, b, a bytes, every alpha opaque, the way GPU textures usually take them.
    pub fn to_rgba(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(self.pixels.len() * 4);
        for &[r, g, b] in &self.pixels {
            rgba.extend_from_slice(&[r, g, b, u8::MAX]);
        }
        return rgba;
    }

    /// The pixel at `x`, `y`, or `None` off the edge.
    pub fn pixel(&self, x: i32, y: i32) -> Option<[u8; 3]> {
        if !self.bounds().contains(Point::new(x, y)) {
            return None;
        }
        return Some(self.pixels[(y * self.width + x) as usize]);
    }

    /// Fills all of it with `color`, ignoring the clip and alpha.
    pub fn clear(&mut self, color: Rgba) {
        self.pixels.fill([color.r, color.g, color.b]);
    }
}

impl RenderTarget for Framebuffer {
    fn fill(&mut self, rect: Rect, color: Rgba) {
        let rect = rect.intersection(self.clip);
        if rect.w == 0 {
            return;
        }
        for y in rect.y..rect.bottom() {
            let row = (y * self.width) as usize;
            let pixels = &mut self.pixels[row + rect.x as usize..row + rect.right() as usize];
            for pixel in pixels {
                *pixel = blend(*pixel, color);
            }
        }
    }

    fn get_clip(&self) -> Rect {
        return self.clip;
    }

    fn set_clip(&mut self, rect: Rect) -> bool {
        let rect = rect.intersection(self.bounds());
        if rect.w == 0 || rect.h == 0 {
            return false;
        }
        self.clip = rect;
        return true;
    }
}

fn blend(under: [u8; 3], color: Rgba) -> [u8; 3] {
    let alpha = color.a as u16;
    let mix = |under: u8, over: u8| {
        let mixed = over as u16 * alpha + under as u16 * (u8::MAX as u16 - alpha);
        (mixed / u8::MAX as u16) as u8
    };
    return [
        mix(under[0], color.r),
        mix(under[1], color.g),
        mix(under[2], color.b),
    ];
}

#[cfg(feature = "ggez")]
mod batched_canvas {
    use ggez::graphics::{Canvas, DrawParam, GraphicsContext, InstanceArray};

    use super::{RenderTarget, Rgba};
    use crate::rect::Rect;

    /// Draws onto a ggez `Canvas`, batching the fills into one draw per clip rect.
    // ggez won't batch fills itself and the text alone is thousands of them a frame, so this does
    // one instanced draw per clip rect. flushes on drop
    pub struct BatchedCanvas<'a> {
        gfx: &'a GraphicsContext,
        canvas: &'a mut Canvas,
        bounds: Rect,
        fills: InstanceArray,
    }

    impl<'a> BatchedCanvas<'a> {
        /// Draws onto `canvas`, never clipping outside `bounds`.
        pub fn new(gfx: &'a GraphicsContext, canvas: &'a mut Canvas, bounds: Rect) -> Self {
            let fills = InstanceArray::new(gfx, None);
            return Self {
                gfx,
                canvas,
                bounds,
                fills,
            };
        }

        fn flush(&mut self) {
            if self.fills.instances().is_empty() {
                return;
            }
            // the canvas only keeps a view of the array, so each batch gets an array of its own
            let fills = std::mem::replace(&mut self.fills, InstanceArray::new(self.gfx, None));
            self.canvas.draw(&fills, DrawParam::new());
        }
    }

    impl Drop for BatchedCanvas<'_> {
        fn drop(&mut self) {
            self.flush();
        }
    }

    impl RenderTarget for BatchedCanvas<'_> {
        fn fill(&mut self, rect: Rect, color: Rgba) {
            let color = ggez::graphics::Color::from_rgba(color.r, color.g, color.b, color.a);
            self.fills
                .push(DrawParam::new().dest_rect(rect.into()).color(color));
        }

        fn get_clip(&self) -> Rect {
            let rect = self.canvas.scissor_rect();
            return Rect::new(rect.x as i32, rect.y as i32, rect.w as i32, rect.h as i32);
        }

        fn set_clip(&mut self, rect: Rect) -> bool {
            // ggez moves a negative start to 0 without shrinking the rect, and rejects empty ones
            let rect = rect.intersection(self.bounds);
            if rect.w == 0 || rect.h == 0 {
                return false;
            }
            self.flush();
            return self.canvas.set_scissor_rect(rect.into()).is_ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_is_opaque() {
        let mut buffer = Framebuffer::new(1, 1);
        buffer.fill(buffer.bounds(), Rgba::rgb(1, 2, 3));
        assert_eq!(buffer.to_rgba(), [1, 2, 3, 255]);
    }

    #[test]
    fn scaled_makes_each_pixel_a_block() {
        let mut buffer = Framebuffer::new(2, 1);
        buffer.fill(Rect::new(1, 0, 1, 1), Rgba::rgb(255, 0, 0));
        let scaled = buffer.scaled(2);
        assert_eq!((scaled.width(), scaled.height()), (4, 2));
        assert_eq!(scaled.pixel(1, 1), Some([0, 0, 0]));
        assert_eq!(scaled.pixel(2, 0), Some([255, 0, 0]));
        assert_eq!(scaled.pixel(3, 1), Some([255, 0, 0]));
    }

    #[test]
    fn framebuffer_fill() {
        let mut buffer = Framebuffer::new(4, 3);
        buffer.fill(Rect::new(1, 1, 10, 1), Rgba::rgb(255, 0, 0));
        assert_eq!(buffer.pixel(0, 1), Some([0, 0, 0]));
        assert_eq!(buffer.pixel(3, 1), Some([255, 0, 0]));
        assert_eq!(buffer.pixel(1, 2), Some([0, 0, 0]));
        assert_eq!(buffer.pixel(4, 1), None);
        assert_eq!(&buffer.as_bytes()[15..18], [255, 0, 0]);
    }

    #[test]
    fn framebuffer_clip() {
        let mut buffer = Framebuffer::new(4, 4);
        assert!(!buffer.set_clip(Rect::new(10, 10, 2, 2)));
        assert_eq!(buffer.get_clip(), buffer.bounds());
        assert!(buffer.set_clip(Rect::new(2, 2, 5, 5)));
        assert_eq!(buffer.get_clip(), Rect::new(2, 2, 2, 2));
        buffer.fill(buffer.bounds(), Rgba::rgb(0, 255, 0));
        assert_eq!(buffer.pixel(1, 1), Some([0, 0, 0]));
        assert_eq!(buffer.pixel(2, 2), Some([0, 255, 0]));
    }

    #[test]
    fn framebuffer_blends() {
        let mut buffer = Framebuffer::new(1, 1);
        buffer.clear(Rgba::rgb(0, 0, 200));
        let half_white = Rgba {
            a: 128,
            ..Rgba::rgb(255, 255, 255)
        };
        buffer.fill(buffer.bounds(), half_white);
        assert_eq!(buffer.pixel(0, 0), Some([128, 128, 227]));
    }
}
