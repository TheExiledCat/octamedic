pub use crate::palette::Pen;
use crate::{
    font::{self, Font},
    palette::IntuitionColorPalette,
    rect::Rect,
    shape::{self, Direction, Sides, Span, Spans},
    target::{RenderTarget, Rgba},
    text::Line,
};

/// Topaz's width in pixels.
pub const CHAR_WIDTH: i32 = 8;

/// Topaz's height in lines.
pub const CHAR_HEIGHT: i32 = 8;

/// Draws shapes and text onto a [`RenderTarget`] with pens from a palette.
pub struct Painter<'a> {
    target: &'a mut dyn RenderTarget,
    palette: &'a IntuitionColorPalette,
}

impl<'a> Painter<'a> {
    /// Draws onto `target` with the colours of `palette`.
    pub fn new(target: &'a mut dyn RenderTarget, palette: &'a IntuitionColorPalette) -> Self {
        return Self { target, palette };
    }

    /// The colour `pen` has in the palette.
    pub fn color(&self, pen: Pen) -> Rgba {
        return self.palette.get(pen).to_rgba();
    }

    /// Runs `draw` with drawing clipped to `rect`.
    // draw isn't called at all if rect is empty or off screen
    pub fn clipped(&mut self, rect: Rect, draw: impl FnOnce(&mut Self)) {
        let previous = self.target.get_clip();
        if self.target.set_clip(rect) {
            draw(self);
            self.target.set_clip(previous);
        }
    }

    /// Fills `rect` with `pen`.
    pub fn fill(&mut self, rect: Rect, pen: Pen) {
        let color = self.color(pen);
        self.target.fill(rect, color);
    }

    /// Fills `rect` with a colour from outside the palette, like the copper gives.
    pub fn fill_color(&mut self, rect: Rect, color: Rgba) {
        self.target.fill(rect, color);
    }

    /// Fills `rect` with `pen` at `opacity`, from 0 to 1.
    pub fn fill_translucent(&mut self, rect: Rect, pen: Pen, opacity: f32) {
        let mut color = self.color(pen);
        color.a = (opacity.clamp(0.0, 1.0) * Rgba::OPAQUE as f32).round() as u8;
        self.target.fill(rect, color);
    }

    /// Draws `text` in Topaz with its top left at `x`, `y`.
    pub fn text(&mut self, x: i32, y: i32, text: &str, pen: Pen) {
        self.text_in_font(x, y, text, pen, Font::TOPAZ);
    }

    /// Draws `text` bold the Amiga way.
    // amiga bold, same text again 1px right. fits since glyphs leave the last column blank
    pub fn text_bold(&mut self, x: i32, y: i32, text: &str, pen: Pen, font: Font) {
        self.text_in_font(x, y, text, pen, font);
        self.text_in_font(x + 1, y, text, pen, font);
    }

    /// Draws a styled line, spans without a pen of their own in `pen`.
    pub fn line(&mut self, x: i32, y: i32, line: &Line, pen: Pen) {
        let mut span_x = x;
        for span in &line.0 {
            let pen = span.pen.unwrap_or(pen);
            if span.shadowed {
                self.text_in_font(span_x + 2, y + 2, &span.text, Pen::SHADOW, span.font);
                self.text_in_font(span_x + 1, y + 1, &span.text, Pen::FILL, span.font);
            }
            if span.bold {
                self.text_bold(span_x, y, &span.text, pen, span.font);
            } else {
                self.text_in_font(span_x, y, &span.text, pen, span.font);
            }
            span_x += span.width();
        }
    }

    /// Draws `text` in `font`.
    pub fn text_in_font(&mut self, x: i32, y: i32, text: &str, pen: Pen, font: Font) {
        self.glyphs(x, y, text, pen, font, [1.0, 1.0]);
    }

    /// Draws Topaz `text` scaled, rounded to whole pixels.
    pub fn text_scaled(&mut self, x: i32, y: i32, text: &str, pen: Pen, scale: [f32; 2]) {
        self.glyphs(x, y, text, pen, Font::TOPAZ, scale);
    }

    /// Draws a line cut to fit `rect` and lined up in it.
    pub fn line_in(&mut self, rect: Rect, line: &Line, pen: Pen, align: Align) {
        let line = line.fit(rect.w);
        let x = line_x(rect, &line, align);
        self.line(x, text_y(rect, line.height()), &line, pen);
    }

    /// Draws a picture from rows of text: `#` shine, `f` fill, `s` shadow, anything else see-through.
    pub fn picture(&mut self, rows: &[&str], x: i32, y: i32) {
        for (line, row) in rows.iter().enumerate() {
            for (column, pixel) in row.chars().enumerate() {
                let pen = match pixel {
                    '#' => Pen::SHINE,
                    'f' => Pen::FILL,
                    's' => Pen::SHADOW,
                    _ => continue,
                };
                self.fill(Rect::new(x + column as i32, y + line as i32, 1, 1), pen);
            }
        }
    }

    /// Fills `rect` with a checkerboard of `dots` and `gaps`.
    // checkerboard lines up with the screen, not the rect, so rects next to each other match
    pub fn dotted_rect(&mut self, rect: Rect, dots: Pen, gaps: Pen) {
        self.fill(rect, gaps);
        for y in rect.y..rect.bottom() {
            let first = rect.x + (rect.x + y).rem_euclid(2);
            for x in (first..rect.right()).step_by(2) {
                self.fill(Rect::new(x, y, 1, 1), dots);
            }
        }
    }

    /// Draws a 1px box just inside `rect`.
    pub fn outline(&mut self, rect: Rect, pen: Pen) {
        let Rect { x, y, w, h } = rect;
        self.fill(Rect::new(x, y, w, 1), pen);
        self.fill(Rect::new(x, y + h - 1, w, 1), pen);
        self.fill(Rect::new(x, y, 1, h), pen);
        self.fill(Rect::new(x + w - 1, y, 1, h), pen);
    }

    /// Draws the raised bevel and returns what's inside it.
    // the bevel, shine top/right and shadow left/bottom. sides are 2x thickness for the tall
    // pixels, corners meet on a slant like the original. returns the inside
    pub fn frame(&mut self, rect: Rect, thickness: i32) -> Rect {
        return self.bevel(rect, thickness, Pen::SHINE, Pen::SHADOW);
    }

    /// Draws the sunk in bevel and returns what's inside it.
    pub fn frame_inverted(&mut self, rect: Rect, thickness: i32) -> Rect {
        return self.bevel(rect, thickness, Pen::SHADOW, Pen::SHINE);
    }

    /// Fills `rect` and draws a border on the chosen `sides`.
    pub fn rect(&mut self, rect: Rect, fill: Option<Pen>, border: Option<Border>, sides: Sides) {
        if let Some(fill) = fill {
            self.fill(rect, fill);
        }
        let Some(Border { pen, thickness }) = border else {
            return;
        };
        let Rect { x, y, w, h } = rect;
        let side = 2 * thickness;
        if sides.top {
            self.fill(Rect::new(x, y, w, thickness), pen);
        }
        if sides.bottom {
            self.fill(Rect::new(x, y + h - thickness, w, thickness), pen);
        }
        if sides.left {
            self.fill(Rect::new(x, y, side, h), pen);
        }
        if sides.right {
            self.fill(Rect::new(x + w - side, y, side, h), pen);
        }
    }

    /// Draws a triangle pointing `direction` in `rect`, filled, bordered or both.
    pub fn triangle(
        &mut self,
        rect: Rect,
        fill: Option<Pen>,
        border: Option<Border>,
        direction: Direction,
    ) {
        self.shape(
            rect,
            &shape::triangle(rect.w, rect.h, direction),
            fill,
            border,
        );
    }

    /// Draws an ellipse filling `rect`, filled, bordered or both.
    pub fn circle(&mut self, rect: Rect, fill: Option<Pen>, border: Option<Border>) {
        self.shape(rect, &shape::ellipse(rect.w, rect.h), fill, border);
    }

    // rounds to whole pixels so scaled text stays sharp
    fn glyphs(&mut self, x: i32, y: i32, text: &str, pen: Pen, font: Font, scale: [f32; 2]) {
        let scaled = |pixels: i32, axis: usize| (pixels as f32 * scale[axis]).round() as i32;
        let mut advance = 0;
        for c in text.chars() {
            if let Some(glyph) = font.glyph(c) {
                let ink = advance + font.bearing();
                for (line, &bits) in glyph.lines.iter().enumerate() {
                    let top = scaled(line as i32, 1);
                    let height = scaled(line as i32 + 1, 1) - top;
                    for (start, width) in font::runs(bits) {
                        let left = scaled(ink + start, 0);
                        let run_width = scaled(ink + start + width, 0) - left;
                        self.fill(Rect::new(x + left, y + top, run_width, height), pen);
                    }
                }
            }
            advance += font.advance(c);
        }
    }

    fn bevel(&mut self, rect: Rect, thickness: i32, top: Pen, bottom: Pen) -> Rect {
        let Rect { x, y, w, h } = rect;
        let side = 2 * thickness;
        self.fill(Rect::new(x, y, side, h), bottom);
        self.fill(Rect::new(x + w - side, y, side, h), top);
        for line in 0..thickness {
            let inset = 2 * line + 1;
            self.fill(Rect::new(x + inset, y + line, w - inset, 1), top);
            self.fill(Rect::new(x, y + h - 1 - line, w - inset, 1), bottom);
        }
        return inside_frame(rect, thickness);
    }

    fn shape(&mut self, rect: Rect, spans: &Spans, fill: Option<Pen>, border: Option<Border>) {
        let row = |y: usize, (left, right): Span| {
            Rect::new(rect.x + left, rect.y + y as i32, right - left, 1)
        };
        let Some(Border { pen, thickness }) = border else {
            if let Some(fill) = fill {
                for (y, span) in spans.iter().enumerate() {
                    if let Some(span) = *span {
                        self.fill(row(y, span), fill);
                    }
                }
            }
            return;
        };
        let inside = shape::inside(spans, thickness);
        for (y, (span, inner)) in spans.iter().zip(&inside).enumerate() {
            let Some((left, right)) = *span else {
                continue;
            };
            match *inner {
                Some((inner_left, inner_right)) => {
                    self.fill(row(y, (left, inner_left)), pen);
                    self.fill(row(y, (inner_right, right)), pen);
                    if let Some(fill) = fill {
                        self.fill(row(y, (inner_left, inner_right)), fill);
                    }
                }
                None => self.fill(row(y, (left, right)), pen),
            }
        }
    }
}

/// Width of `text` in Topaz.
pub fn text_width(text: &str) -> i32 {
    return text.chars().count() as i32 * CHAR_WIDTH;
}

/// What's inside a frame of `thickness` drawn around `rect`.
pub fn inside_frame(rect: Rect, thickness: i32) -> Rect {
    return rect.inset(2 * thickness, thickness);
}

/// A border's pen and thickness.
// sides are drawn 2x thickness wide because of the tall pixels
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Border {
    pub pen: Pen,
    pub thickness: i32,
}

/// How something lines up across an area.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    Left,
    #[default]
    Center,
    Right,
}

/// Where text `height` lines tall goes to be centred down `rect`.
// pinned to the top when it doesn't fit, fonts leave the bottom line blank anyway
pub fn text_y(rect: Rect, height: i32) -> i32 {
    return rect.y + ((rect.h - height) / 2).max(0);
}

/// Where `line` goes across `rect` to be lined up with `align`.
// centres the ink, the blank pixels fonts leave either side can be uneven
pub fn line_x(rect: Rect, line: &Line, align: Align) -> i32 {
    if align != Align::Center {
        return aligned_text_x(rect, line.width(), align);
    }
    let (left, right) = line.blank_sides();
    return aligned_text_x(rect, line.width() - left - right, align) - left;
}

/// Where something `width` wide goes across `rect` to be lined up with `align`.
pub fn aligned_text_x(rect: Rect, width: i32, align: Align) -> i32 {
    let spare = rect.w - width;
    return match align {
        Align::Left => rect.x,
        Align::Center => rect.x + spare / 2,
        Align::Right => rect.x + spare,
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_alignment() {
        let rect = Rect::new(10, 0, 37, 8);
        assert_eq!(aligned_text_x(rect, 16, Align::Left), 10);
        assert_eq!(aligned_text_x(rect, 16, Align::Center), 20);
        assert_eq!(aligned_text_x(rect, 16, Align::Right), 31);
    }
}
