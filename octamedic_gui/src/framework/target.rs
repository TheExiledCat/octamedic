use ggez::graphics::{Canvas, DrawParam, GraphicsContext, InstanceArray};

use crate::framework::rect::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// 0 is fully transparent, `OPAQUE` is fully solid.
    pub a: u8,
}

impl Color {
    pub const OPAQUE: u8 = u8::MAX;

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        return Self {
            r,
            g,
            b,
            a: Self::OPAQUE,
        };
    }
}

/// Something a `Painter` can draw on, such as a ggez canvas or a framebuffer. Coordinates are in
/// screen pixels.
pub trait RenderTarget {
    /// Fills `rect` with `color`. A color with an alpha below `Color::OPAQUE` is blended with
    /// what's already there. Pixels outside the clip rect are left alone.
    fn fill(&mut self, rect: Rect, color: Color);

    /// The area drawing is currently limited to.
    fn get_clip(&self) -> Rect;

    /// Limits drawing to the part of `rect` on the target. Returns false if none of it is, in
    /// which case the clip is not changed.
    fn set_clip(&mut self, rect: Rect) -> bool;
}

/// A ggez canvas whose fills are drawn together, in one instanced draw for each clip rect. The
/// canvas doesn't batch its own draws, and a frame's text alone is thousands of fills, one for
/// each run of a glyph's pixels. What's left is drawn when this is dropped.
pub struct BatchedCanvas<'a> {
    gfx: &'a GraphicsContext,
    canvas: &'a mut Canvas,
    /// The canvas's image, which a clip rect is cut to.
    bounds: Rect,
    fills: InstanceArray,
}

impl<'a> BatchedCanvas<'a> {
    pub fn new(gfx: &'a GraphicsContext, canvas: &'a mut Canvas, bounds: Rect) -> Self {
        let fills = InstanceArray::new(gfx, None);
        return Self {
            gfx,
            canvas,
            bounds,
            fills,
        };
    }

    /// Draws the fills so far, in the clip rect they were made in.
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
    fn fill(&mut self, rect: Rect, color: Color) {
        let color = ggez::graphics::Color::from_rgba(color.r, color.g, color.b, color.a);
        self.fills
            .push(DrawParam::new().dest_rect(rect.into()).color(color));
    }

    fn get_clip(&self) -> Rect {
        let rect = self.canvas.scissor_rect();
        return Rect::new(rect.x as i32, rect.y as i32, rect.w as i32, rect.h as i32);
    }

    fn set_clip(&mut self, rect: Rect) -> bool {
        // ggez turns a rect that starts left of or above the canvas into one far past its end,
        // and refuses it
        let rect = rect.intersection(self.bounds);
        if rect.w == 0 || rect.h == 0 {
            return false;
        }
        self.flush();
        return self.canvas.set_scissor_rect(rect.into()).is_ok();
    }
}
