use std::borrow::Cow;

use crate::framework::{font::Font, painter::Pen};

/// A run of text drawn the same way.
#[derive(Clone, Debug, PartialEq)]
pub struct Span<'a> {
    pub text: Cow<'a, str>,
    pub font: Font,
    pub bold: bool,
    /// Drawn over copies of itself in the fill and shadow pens, a line and a pixel further down
    /// and right each, as the title is.
    pub shadowed: bool,
    /// `None` draws it in the pen the line is drawn in, so it inverts with a button.
    pub pen: Option<Pen>,
}

impl<'a> Span<'a> {
    pub fn new(text: impl Into<Cow<'a, str>>) -> Self {
        return Self {
            text: text.into(),
            font: Font::default(),
            bold: false,
            shadowed: false,
            pen: None,
        };
    }

    pub fn font(mut self, font: Font) -> Self {
        self.font = font;
        return self;
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        return self;
    }

    pub fn shadowed(mut self) -> Self {
        self.shadowed = true;
        return self;
    }

    pub fn pen(mut self, pen: Pen) -> Self {
        self.pen = Some(pen);
        return self;
    }

    pub fn width(&self) -> i32 {
        return self.font.width(&self.text);
    }

    /// The same span with other text.
    fn with_text<'b>(&self, text: impl Into<Cow<'b, str>>) -> Span<'b> {
        return Span {
            text: text.into(),
            font: self.font,
            bold: self.bold,
            shadowed: self.shadowed,
            pen: self.pen,
        };
    }
}

impl<'a> From<&'a str> for Span<'a> {
    fn from(text: &'a str) -> Self {
        return Span::new(text);
    }
}

impl From<String> for Span<'static> {
    fn from(text: String) -> Self {
        return Span::new(text);
    }
}

/// Spans side by side, each in its own font and style.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line<'a>(pub Vec<Span<'a>>);

impl<'a> Line<'a> {
    pub fn width(&self) -> i32 {
        return self.0.iter().map(Span::width).sum();
    }

    /// The tallest of its fonts.
    pub fn height(&self) -> i32 {
        return self
            .0
            .iter()
            .map(|span| span.font.height())
            .max()
            .unwrap_or(0);
    }

    /// The characters, with the font each is drawn in.
    fn chars(&self) -> impl Iterator<Item = (char, Font)> + '_ {
        return self
            .0
            .iter()
            .flat_map(|span| span.text.chars().map(move |c| (c, span.font)));
    }

    /// As much of the line as fits in `width` pixels, in whole characters. The fonts leave the
    /// last column of a character blank, so the last one may hang a pixel past the end.
    pub fn fit(&self, width: i32) -> Line<'_> {
        let mut used = 0;
        let mut spans = Vec::new();
        for span in &self.0 {
            for (end, c) in span.text.char_indices() {
                used += span.font.advance(c);
                if used - 1 > width {
                    if end > 0 {
                        spans.push(span.with_text(&span.text[..end]));
                    }
                    return Line(spans);
                }
            }
            spans.push(span.with_text(span.text.as_ref()));
        }
        return Line(spans);
    }

    /// The character `offset` pixels into the line, counting across all its spans, if it's over
    /// one.
    pub fn index_at(&self, offset: i32) -> Option<usize> {
        if offset < 0 {
            return None;
        }
        let mut end = 0;
        for (index, (c, font)) in self.chars().enumerate() {
            end += font.advance(c);
            if offset < end {
                return Some(index);
            }
        }
        return None;
    }
}

impl<'a> From<Span<'a>> for Line<'a> {
    fn from(span: Span<'a>) -> Self {
        return Line(vec![span]);
    }
}

impl<'a> From<Vec<Span<'a>>> for Line<'a> {
    fn from(spans: Vec<Span<'a>>) -> Self {
        return Line(spans);
    }
}

impl<'a> From<&'a str> for Line<'a> {
    fn from(text: &'a str) -> Self {
        return Line(vec![Span::new(text)]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mixed() -> Line<'static> {
        return Line(vec![
            Span::new("FI").font(Font::Panel),
            Span::new("01").font(Font::Interface),
        ]);
    }

    #[test]
    fn a_line_is_as_wide_as_its_spans() {
        assert_eq!(mixed().width(), 16 + 16);
    }

    #[test]
    fn fitting_cuts_across_spans() {
        assert_eq!(mixed().fit(32), mixed());
        let line = mixed();
        let cut = line.fit(24);
        assert_eq!(cut.0.len(), 2);
        assert_eq!(cut.0[1].text, "0");
        assert_eq!(cut.0[1].font, Font::Interface);
        assert_eq!(mixed().fit(14).0, [Span::new("F").font(Font::Panel)]);
    }

    #[test]
    fn a_character_may_hang_a_pixel_past_the_end() {
        let topaz = Line::from("Speed/Beat");
        assert_eq!(topaz.fit(40).0[0].text, "Speed");
        assert_eq!(Line::from("Speed").fit(39).0[0].text, "Speed");
        assert_eq!(Line::from("Speed").fit(38).0[0].text, "Spee");
        assert!(Line::from("Speed").fit(-8).0.is_empty());
    }

    #[test]
    fn indexes_count_across_spans() {
        assert_eq!(mixed().index_at(0), Some(0));
        assert_eq!(mixed().index_at(10), Some(1));
        assert_eq!(mixed().index_at(16), Some(2));
        assert_eq!(mixed().index_at(31), Some(3));
        assert_eq!(mixed().index_at(32), None);
        assert_eq!(mixed().index_at(-1), None);
    }
}
