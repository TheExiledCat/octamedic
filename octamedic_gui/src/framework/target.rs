use ggez::graphics::{Canvas, DrawParam, GraphicsContext, InstanceArray};

use crate::framework::rect::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
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

pub trait RenderTarget {
    fn fill(&mut self, rect: Rect, color: Color);

    fn get_clip(&self) -> Rect;

    // false if rect is entirely off the target, and the clip stays as it was
    fn set_clip(&mut self, rect: Rect) -> bool;
}

// ggez won't batch fills itself and the text alone is thousands of them a frame, so this does
// one instanced draw per clip rect. flushes on drop
pub struct BatchedCanvas<'a> {
    gfx: &'a GraphicsContext,
    canvas: &'a mut Canvas,
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
        // ggez moves a negative start to 0 without shrinking the rect, and rejects empty ones
        let rect = rect.intersection(self.bounds);
        if rect.w == 0 || rect.h == 0 {
            return false;
        }
        self.flush();
        return self.canvas.set_scissor_rect(rect.into()).is_ok();
    }
}
