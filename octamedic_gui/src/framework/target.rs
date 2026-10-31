use ggez::graphics::{Canvas, DrawParam, Quad};

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

    /// Limits drawing to `rect`. Returns false if `rect` is entirely outside the target, in which
    /// case the clip is not changed.
    fn set_clip(&mut self, rect: Rect) -> bool;
}

impl RenderTarget for Canvas {
    fn fill(&mut self, rect: Rect, color: Color) {
        let color = ggez::graphics::Color::from_rgba(color.r, color.g, color.b, color.a);
        self.draw(&Quad, DrawParam::new().dest_rect(rect.into()).color(color));
    }

    fn get_clip(&self) -> Rect {
        let rect = self.scissor_rect();
        return Rect::new(rect.x as i32, rect.y as i32, rect.w as i32, rect.h as i32);
    }

    fn set_clip(&mut self, rect: Rect) -> bool {
        return self.set_scissor_rect(rect.into()).is_ok();
    }
}
