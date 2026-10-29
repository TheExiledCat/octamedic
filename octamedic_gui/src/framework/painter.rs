use ggez::graphics::{Canvas, Color, DrawParam, Image, PxScale, Quad, Text, TextFragment};
use octamedic_core::data::palette::OctamedColorPallete;

use crate::framework::rect::Rect;

pub const TEXT_FONT: &str = "Topaz";

/// Topaz is 8×8 in screen pixels.
pub const CHAR_WIDTH: i32 = 8;

pub const CHAR_HEIGHT: i32 = 8;

/// Double-width characters, which the block editor uses for blocks of up to 4 tracks.
pub const WIDE_CHAR_WIDTH: i32 = 16;

/// A palette index. What it looks like depends on the song's palette.
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

/// Draws onto the screen in screen pixels, with the palette's pens.
pub struct Painter<'a> {
    canvas: &'a mut Canvas,
    pallete: &'a OctamedColorPallete,
}

impl<'a> Painter<'a> {
    pub fn new(canvas: &'a mut Canvas, pallete: &'a OctamedColorPallete) -> Self {
        return Self { canvas, pallete };
    }

    pub fn color(&self, pen: Pen) -> Color {
        let [r, g, b] = self.pallete.rgb(pen.0);
        return Color::from_rgb(r, g, b);
    }

    pub fn fill(&mut self, rect: Rect, pen: Pen) {
        let color = self.color(pen);
        self.canvas
            .draw(&Quad, DrawParam::new().dest_rect(rect.into()).color(color));
    }

    pub fn text(&mut self, x: i32, y: i32, text: &str, pen: Pen) {
        self.text_sized(x, y, text, pen, CHAR_WIDTH);
    }

    /// Text with `char_width` pixel wide characters, `CHAR_WIDTH` or `WIDE_CHAR_WIDTH`.
    pub fn text_sized(&mut self, x: i32, y: i32, text: &str, pen: Pen, char_width: i32) {
        // the font's glyphs are half as wide as its em is tall, for the screen's tall pixels, so
        // it's scaled twice as much across
        let scale = PxScale {
            x: (char_width * 2) as f32,
            y: CHAR_HEIGHT as f32,
        };
        let fragment = TextFragment::new(text)
            .font(TEXT_FONT)
            .scale(scale)
            .color(self.color(pen));
        self.canvas.draw(
            &Text::new(fragment),
            DrawParam::new().dest([x as f32, y as f32]),
        );
    }

    /// `text` in the middle of `rect`, rounded down to whole pixels.
    pub fn text_centered(&mut self, rect: Rect, text: &str, pen: Pen) {
        let x = centered_text_x(rect, text);
        let y = rect.y + (rect.h - CHAR_HEIGHT) / 2;
        self.text(x, y, text, pen);
    }

    /// A one pixel frame just inside `rect`.
    pub fn outline(&mut self, rect: Rect, pen: Pen) {
        let Rect { x, y, w, h } = rect;
        self.fill(Rect::new(x, y, w, 1), pen);
        self.fill(Rect::new(x, y + h - 1, w, 1), pen);
        self.fill(Rect::new(x, y, 1, h), pen);
        self.fill(Rect::new(x + w - 1, y, 1, h), pen);
    }

    /// OctaMED's frame around panels and gadgets: light along the top and right, shadow along the
    /// left and bottom. The sides are 2 pixels wide, so they look as thick as the 1 line tall
    /// top and bottom on the tall pixels.
    pub fn frame(&mut self, rect: Rect) {
        let Rect { x, y, w, h } = rect;
        self.fill(Rect::new(x, y, w, 1), Pen::LIGHT);
        self.fill(Rect::new(x, y + h - 1, w, 1), Pen::SHADOW);
        self.fill(Rect::new(x, y, 2, h), Pen::SHADOW);
        self.fill(Rect::new(x + w - 2, y, 2, h), Pen::LIGHT);
    }

    pub fn image(&mut self, image: &Image, x: i32, y: i32, scale: [f32; 2]) {
        self.canvas.draw(
            image,
            DrawParam::new().dest([x as f32, y as f32]).scale(scale),
        );
    }
}

pub fn text_width(text: &str) -> i32 {
    return text.chars().count() as i32 * CHAR_WIDTH;
}

/// What's left inside `Painter::frame`'s edges.
pub fn inside_frame(rect: Rect) -> Rect {
    return Rect::new(rect.x + 2, rect.y + 1, rect.w - 4, rect.h - 2);
}

/// Where `text_centered` starts `text` across `rect`.
pub fn centered_text_x(rect: Rect, text: &str) -> i32 {
    return rect.x + (rect.w - text_width(text)) / 2;
}
