use octamedic_core::data::palette::OctamedColorPalette;

use crate::framework::{
    font::{self, Font},
    rect::Rect,
    shape::{self, Direction, Sides, Span, Spans},
    target::{Color, RenderTarget},
    text::Line,
};

/// Topaz's character size.
pub const CHAR_WIDTH: i32 = 8;

pub const CHAR_HEIGHT: i32 = 8;

/// An index into the song's palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pen(pub usize);

impl Pen {
    pub const BLACK: Self = Self(0);
    pub const LIGHT: Self = Self(1);
    pub const SHADOW: Self = Self(2);
    pub const FILL: Self = Self(3);
    pub const RED: Self = Self(4);
    pub const ROSE: Self = Self(5);
    pub const PINK: Self = Self(6);
    pub const BLUSH: Self = Self(7);
}

pub struct Painter<'a> {
    target: &'a mut dyn RenderTarget,
    palette: &'a OctamedColorPalette,
}

impl<'a> Painter<'a> {
    pub fn new(target: &'a mut dyn RenderTarget, palette: &'a OctamedColorPalette) -> Self {
        return Self { target, palette };
    }

    pub fn color(&self, pen: Pen) -> Color {
        let [r, g, b] = self.palette.rgb(pen.0);
        return Color::rgb(r, g, b);
    }

    /// Skips `draw` entirely when `rect` is empty or off the screen.
    pub fn clipped(&mut self, rect: Rect, draw: impl FnOnce(&mut Self)) {
        let previous = self.target.get_clip();
        if self.target.set_clip(rect) {
            draw(self);
            self.target.set_clip(previous);
        }
    }

    pub fn fill(&mut self, rect: Rect, pen: Pen) {
        let color = self.color(pen);
        self.target.fill(rect, color);
    }

    // for colours outside the palette, which OctaMED gets from the copper
    pub fn fill_color(&mut self, rect: Rect, color: Color) {
        self.target.fill(rect, color);
    }

    pub fn fill_translucent(&mut self, rect: Rect, pen: Pen, opacity: f32) {
        let mut color = self.color(pen);
        color.a = (opacity.clamp(0.0, 1.0) * Color::OPAQUE as f32).round() as u8;
        self.target.fill(rect, color);
    }

    pub fn text(&mut self, x: i32, y: i32, text: &str, pen: Pen) {
        self.text_in_font(x, y, text, pen, Font::Topaz);
    }

    /// Amiga bold: the text again one pixel right. It fits the same space, since glyphs leave
    /// their last column blank.
    pub fn text_bold(&mut self, x: i32, y: i32, text: &str, pen: Pen, font: Font) {
        self.text_in_font(x, y, text, pen, font);
        self.text_in_font(x + 1, y, text, pen, font);
    }

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

    pub fn text_in_font(&mut self, x: i32, y: i32, text: &str, pen: Pen, font: Font) {
        self.glyphs(x, y, text, pen, font, [1.0, 1.0]);
    }

    pub fn text_scaled(&mut self, x: i32, y: i32, text: &str, pen: Pen, scale: [f32; 2]) {
        self.glyphs(x, y, text, pen, Font::Topaz, scale);
    }

    pub fn line_in(&mut self, rect: Rect, line: &Line, pen: Pen, align: Align) {
        let line = line.fit(rect.w);
        let x = line_x(rect, &line, align);
        self.line(x, text_y(rect, line.height()), &line, pen);
    }

    /// `#` is the light pen, `f` fill, `s` shadow; anything else is transparent.
    pub fn picture(&mut self, rows: &[&str], x: i32, y: i32) {
        for (line, row) in rows.iter().enumerate() {
            for (column, pixel) in row.chars().enumerate() {
                let pen = match pixel {
                    '#' => Pen::LIGHT,
                    'f' => Pen::FILL,
                    's' => Pen::SHADOW,
                    _ => continue,
                };
                self.fill(Rect::new(x + column as i32, y + line as i32, 1, 1), pen);
            }
        }
    }

    /// A checkerboard aligned to the screen, not the rect, so neighbouring rects line up.
    pub fn dotted_rect(&mut self, rect: Rect, dots: Pen, gaps: Pen) {
        self.fill(rect, gaps);
        for y in rect.y..rect.bottom() {
            let first = rect.x + (rect.x + y).rem_euclid(2);
            for x in (first..rect.right()).step_by(2) {
                self.fill(Rect::new(x, y, 1, 1), dots);
            }
        }
    }

    pub fn outline(&mut self, rect: Rect, pen: Pen) {
        let Rect { x, y, w, h } = rect;
        self.fill(Rect::new(x, y, w, 1), pen);
        self.fill(Rect::new(x, y + h - 1, w, 1), pen);
        self.fill(Rect::new(x, y, 1, h), pen);
        self.fill(Rect::new(x + w - 1, y, 1, h), pen);
    }

    /// OctaMED's bevel: light top and right, shadow left and bottom. The sides are twice as
    /// wide as `thickness` for the tall pixels, and the corners meet on a slant as in OctaMED.
    /// Returns the inside.
    pub fn frame(&mut self, rect: Rect, thickness: i32) -> Rect {
        return self.bevel(rect, thickness, Pen::LIGHT, Pen::SHADOW);
    }

    /// Used around the status row's gauge boxes.
    pub fn frame_inverted(&mut self, rect: Rect, thickness: i32) -> Rect {
        return self.bevel(rect, thickness, Pen::SHADOW, Pen::LIGHT);
    }

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

    pub fn circle(&mut self, rect: Rect, fill: Option<Pen>, border: Option<Border>) {
        self.shape(rect, &shape::ellipse(rect.w, rect.h), fill, border);
    }

    /// Rounds to whole pixels so glyphs stay sharp at any scale.
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

pub fn text_width(text: &str) -> i32 {
    return text.chars().count() as i32 * CHAR_WIDTH;
}

pub fn inside_frame(rect: Rect, thickness: i32) -> Rect {
    return rect.inset(2 * thickness, thickness);
}

/// Twice as wide as `thickness` at the sides, for the tall pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Border {
    pub pen: Pen,
    pub thickness: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    Left,
    #[default]
    Center,
    Right,
}

/// Pinned to the top when taller than `rect`, as fonts leave their bottom line blank.
pub fn text_y(rect: Rect, height: i32) -> i32 {
    return rect.y + ((rect.h - height) / 2).max(0);
}

/// Centring goes by the ink, not the blank pixels fonts leave around it, which can be uneven.
pub fn line_x(rect: Rect, line: &Line, align: Align) -> i32 {
    if align != Align::Center {
        return aligned_text_x(rect, line.width(), align);
    }
    let (left, right) = line.blank_sides();
    return aligned_text_x(rect, line.width() - left - right, align) - left;
}

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
    fn text_lines_up_with_the_rect() {
        let rect = Rect::new(10, 0, 37, 8);
        assert_eq!(aligned_text_x(rect, 16, Align::Left), 10);
        assert_eq!(aligned_text_x(rect, 16, Align::Center), 20);
        assert_eq!(aligned_text_x(rect, 16, Align::Right), 31);
    }
}
