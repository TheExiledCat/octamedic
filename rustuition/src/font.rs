use std::{collections::HashMap, fmt, sync::LazyLock};

use ab_glyph::{Font as _, FontRef, PxScale, point};

use crate::painter::{CHAR_HEIGHT, CHAR_WIDTH};

// kickstart 2 topaz
const TOPAZ_TTF: &[u8] = include_bytes!("../assets/fonts/AmigaTopazUnicodeRus.ttf");

const MAX_GLYPH_WIDTH: usize = u32::BITS as usize;

const LEFT_PIXEL: u32 = 1 << (u32::BITS - 1);

/// A handle to a [`BitmapFont`], cheap to copy around. Make your own from a static:
///
/// ```
/// use std::sync::LazyLock;
/// use rustuition::font::{BitmapFont, Font};
///
/// static WIDE: LazyLock<BitmapFont> =
///     LazyLock::new(|| BitmapFont::new("wide", 8).with_fallback(Font::TOPAZ, |g| g.widened()));
/// pub const WIDE_FONT: Font = Font::new(|| &WIDE);
/// ```
#[derive(Clone, Copy)]
pub struct Font(fn() -> &'static BitmapFont);

impl Font {
    /// Kickstart 2's Topaz 8, the Amiga's default font.
    pub const TOPAZ: Self = Self::new(|| &TOPAZ);

    /// Makes a handle from a function returning the font, usually `|| &MY_FONT`.
    pub const fn new(font: fn() -> &'static BitmapFont) -> Self {
        return Self(font);
    }

    /// How many lines tall its glyphs are.
    pub fn height(self) -> i32 {
        return self.bitmap().height;
    }

    /// The glyph for `c`, or `None` if the font doesn't have it.
    pub fn glyph(self, c: char) -> Option<&'static Glyph> {
        return self.bitmap().glyph(c);
    }

    /// How far `c` moves along, bearing included.
    pub fn advance(self, c: char) -> i32 {
        let font = self.bitmap();
        let width = self.glyph(c).map_or(font.space, |glyph| glyph.width);
        return width + 2 * font.bearing;
    }

    /// Blank pixels either side of every glyph.
    pub fn bearing(self) -> i32 {
        return self.bitmap().bearing;
    }

    /// Width of `text` in pixels.
    pub fn width(self, text: &str) -> i32 {
        return text.chars().map(|c| self.advance(c)).sum();
    }

    /// Blank pixels left and right of `c`'s ink, bearing included, `None` if it has no ink.
    pub fn blank_sides(self, c: char) -> Option<(i32, i32)> {
        let ink = self.glyph(c)?.ink();
        if ink == 0 {
            return None;
        }
        let left = self.bearing() + ink.leading_zeros() as i32;
        let ink_end = self.bearing() + (u32::BITS - ink.trailing_zeros()) as i32;
        return Some((left, self.advance(c) - ink_end));
    }

    /// `(start, length)` runs of ink across `text` on one glyph line.
    pub fn ink(self, text: &str, line: usize) -> impl Iterator<Item = (i32, i32)> + '_ {
        let mut advance = 0;
        return text.chars().flat_map(move |c| {
            let x = advance + self.bearing();
            advance += self.advance(c);
            let bits = self
                .glyph(c)
                .and_then(|glyph| glyph.lines.get(line).copied());
            return runs(bits.unwrap_or(0)).map(move |(start, width)| (x + start, width));
        });
    }

    /// The font this is a handle to.
    pub fn bitmap(self) -> &'static BitmapFont {
        return (self.0)();
    }
}

impl Default for Font {
    fn default() -> Self {
        return Self::TOPAZ;
    }
}

impl PartialEq for Font {
    fn eq(&self, other: &Self) -> bool {
        return std::ptr::eq(self.bitmap(), other.bitmap());
    }
}

impl Eq for Font {}

impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f.debug_tuple("Font").field(&self.bitmap().name).finish();
    }
}

/// One character's pixels, a `u32` per line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub width: i32,
    // msb is the leftmost pixel
    pub lines: Vec<u32>,
}

impl Glyph {
    /// Every line or'd together, to see which columns have any ink.
    pub fn ink(&self) -> u32 {
        return self.lines.iter().fold(0, |all, &line| all | line);
    }

    /// Adds blank columns on the left and right.
    pub fn with_margins(&self, left: u32, right: u32) -> Glyph {
        return Glyph {
            width: self.width + (left + right) as i32,
            lines: self.lines.iter().map(|&line| line >> left).collect(),
        };
    }

    /// Just the ink, without blank columns either side.
    pub fn trimmed(&self) -> Glyph {
        let ink = self.ink();
        if ink == 0 {
            return self.clone();
        }
        let left = ink.leading_zeros();
        return Glyph {
            width: (u32::BITS - ink.trailing_zeros() - left) as i32,
            lines: self.lines.iter().map(|&line| line << left).collect(),
        };
    }

    /// Every pixel twice as wide, the first 16 columns of it.
    pub fn widened(&self) -> Glyph {
        let lines = self
            .lines
            .iter()
            .map(|&line| {
                let mut wide = 0;
                for x in (0..u16::BITS).filter(|&x| is_set(line, x)) {
                    wide |= (LEFT_PIXEL >> (2 * x)) | (LEFT_PIXEL >> (2 * x + 1));
                }
                wide
            })
            .collect();
        return Glyph {
            width: (self.width * 2).min(MAX_GLYPH_WIDTH as i32),
            lines,
        };
    }

    /// Whether the pixel at `x` on line `y` is ink.
    pub fn is_set(&self, x: u32, y: usize) -> bool {
        return self.lines.get(y).is_some_and(|&line| is_set(line, x));
    }
}

/// `(start, length)` runs of ink on one glyph line.
pub fn runs(line: u32) -> impl Iterator<Item = (i32, i32)> {
    let mut x = 0;
    let width = MAX_GLYPH_WIDTH as u32;
    return std::iter::from_fn(move || {
        while x < width && !is_set(line, x) {
            x += 1;
        }
        if x == width {
            return None;
        }
        let start = x;
        while x < width && is_set(line, x) {
            x += 1;
        }
        return Some((start as i32, (x - start) as i32));
    });
}

fn is_set(line: u32, x: u32) -> bool {
    return x < u32::BITS && line & (LEFT_PIXEL >> x) != 0;
}

/// One colour glyphs, up to 32 pixels wide. Build one with [`BitmapFont::new`] and the `with_`
/// methods, then hand it out as a [`Font`].
#[derive(Debug, Default)]
pub struct BitmapFont {
    name: String,
    glyphs: HashMap<char, Glyph>,
    height: i32,
    bearing: i32,
    // advance for missing glyphs, mostly space
    space: i32,
    capitals_only: bool,
}

impl BitmapFont {
    /// An empty font with glyphs `height` lines tall.
    pub fn new(name: impl Into<String>, height: i32) -> Self {
        return Self {
            name: name.into(),
            height,
            ..Default::default()
        };
    }

    /// Sets the blank pixels either side of every glyph.
    pub fn bearing(mut self, bearing: i32) -> Self {
        self.bearing = bearing;
        return self;
    }

    /// Sets how wide a char with no glyph is.
    pub fn space(mut self, space: i32) -> Self {
        self.space = space;
        return self;
    }

    /// Looks lowercase up as uppercase, for fonts with only capitals.
    pub fn capitals_only(mut self) -> Self {
        self.capitals_only = true;
        return self;
    }

    /// Adds glyphs in the text format [`parse_glyphs`] reads.
    ///
    /// # Panics
    ///
    /// If they don't parse, since fonts are usually baked in with `include_str!`.
    pub fn with_glyphs(mut self, text: &str) -> Self {
        let glyphs = parse_glyphs(text, self.height as usize)
            .unwrap_or_else(|error| panic!("font {}: {error}", self.name));
        self.glyphs.extend(glyphs);
        return self;
    }

    /// Adds or replaces the glyph for `c`.
    pub fn with_glyph(mut self, c: char, glyph: Glyph) -> Self {
        self.glyphs.insert(c, glyph);
        return self;
    }

    /// Adds every glyph of `fallback` this font doesn't have yet, through `convert`.
    pub fn with_fallback(mut self, fallback: Font, convert: impl Fn(&Glyph) -> Glyph) -> Self {
        for (&c, glyph) in &fallback.bitmap().glyphs {
            self.glyphs.entry(c).or_insert_with(|| convert(glyph));
        }
        return self;
    }

    /// The name it was made with, for errors and debugging.
    pub fn name(&self) -> &str {
        return &self.name;
    }

    /// Every glyph in the font, in no particular order.
    pub fn glyphs(&self) -> impl Iterator<Item = (char, &Glyph)> {
        return self.glyphs.iter().map(|(&c, glyph)| (c, glyph));
    }

    /// The glyph for `c`, uppercased first if the font is capitals only.
    pub fn glyph(&self, c: char) -> Option<&Glyph> {
        let c = if self.capitals_only {
            c.to_ascii_uppercase()
        } else {
            c
        };
        return self.glyphs.get(&c);
    }
}

// rasterized from the ttf up front so it doesn't blur
static TOPAZ: LazyLock<BitmapFont> = LazyLock::new(|| BitmapFont {
    name: "Topaz".to_owned(),
    glyphs: rasterize(TOPAZ_TTF).unwrap_or_else(|| panic!("Topaz's TTF couldn't be read")),
    height: CHAR_HEIGHT,
    space: CHAR_WIDTH,
    ..Default::default()
});

// the topaz ttf is the 8 line bitmap font at double height with smoothing half lines in between,
// so take the upper half of each line to get the real pixels back
fn rasterize(ttf: &[u8]) -> Option<HashMap<char, Glyph>> {
    let font = FontRef::try_from_slice(ttf).ok()?;
    let em = font.units_per_em()?;
    let pixel = em / (2 * CHAR_HEIGHT) as f32;
    // `PxScale` is ascent to descent, not the em
    let scale = PxScale::from(font.height_unscaled() / pixel);
    let mut glyphs = HashMap::new();
    for (_, c) in font.codepoint_ids() {
        // chars can be in more than one cmap table, glyph_id gives the one that's used
        let id = font.glyph_id(c);
        let width = (font.h_advance_unscaled(id) / pixel).round() as i32;
        let mut lines = vec![0; CHAR_HEIGHT as usize];
        let glyph = id.with_scale_and_position(scale, point(0.0, em / pixel));
        if let Some(outline) = font.outline_glyph(glyph) {
            let corner = outline.px_bounds().min;
            outline.draw(|x, y, coverage| {
                let (x, y) = (corner.x as i32 + x as i32, corner.y as i32 + y as i32);
                let shown = (0..width.min(MAX_GLYPH_WIDTH as i32)).contains(&x)
                    && (0..2 * CHAR_HEIGHT).contains(&y)
                    && y % 2 == 0;
                if shown && coverage >= 0.5 {
                    lines[(y / 2) as usize] |= LEFT_PIXEL >> x;
                }
            });
        }
        glyphs.insert(c, Glyph { width, lines });
    }
    return Some(glyphs);
}

/// Reads glyphs drawn in text: the char on its own line, `height` lines of `#` (ink) and `.`
/// (blank), then an empty line.
///
/// ```text
/// A
/// ..####..
/// .##..##.
/// ```
pub fn parse_glyphs(text: &str, height: usize) -> Result<Vec<(char, Glyph)>, String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut glyphs = Vec::new();
    for block in lines.split(|line| line.is_empty()) {
        let Some((header, rows)) = block.split_first() else {
            continue;
        };
        let mut chars = header.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return Err(format!("`{header}` should be a single character"));
        };
        if rows.len() != height {
            return Err(format!(
                "`{c}` should be {height} lines, then an empty line"
            ));
        }
        let width = rows[0].chars().count();
        let mut glyph = Glyph {
            width: width as i32,
            lines: vec![0; height],
        };
        for (y, (row, line)) in rows.iter().zip(&mut glyph.lines).enumerate() {
            if width > MAX_GLYPH_WIDTH || row.chars().count() != width {
                return Err(format!(
                    "`{c}` line {}: `{row}` should be 1 to {MAX_GLYPH_WIDTH} wide, as wide as \
                     the line above",
                    y + 1
                ));
            }
            for (x, pixel) in row.chars().enumerate() {
                match pixel {
                    '#' => *line |= LEFT_PIXEL >> x,
                    '.' => {}
                    _ => {
                        return Err(format!(
                            "`{c}` line {}: `{pixel}` should be `#` or `.`",
                            y + 1
                        ));
                    }
                }
            }
        }
        glyphs.push((c, glyph));
    }
    return Ok(glyphs);
}

#[cfg(test)]
mod tests {
    use super::*;

    static SPACED: LazyLock<BitmapFont> = LazyLock::new(|| {
        BitmapFont::new("spaced", 2)
            .bearing(1)
            .space(4)
            .capitals_only()
            .with_glyphs("I\n.##.\n.##.\n")
    });

    const SPACED_FONT: Font = Font::new(|| &SPACED);

    #[test]
    fn bad_glyphs() {
        let rows = "#.\n#.\n#.\n#.\n#.\n";
        assert!(parse_glyphs(&format!("AB\n{rows}#.\n"), 6).is_err());
        assert!(parse_glyphs(&format!("A\n{rows}#..\n"), 6).is_err());
        assert!(parse_glyphs(&format!("A\n{rows}#x\n"), 6).is_err());
        assert!(parse_glyphs(&format!("A\n{rows}"), 6).is_err());
        let too_wide = format!("A\n{}\n", "#".repeat(MAX_GLYPH_WIDTH + 1));
        assert!(parse_glyphs(&too_wide, 1).is_err());
        let two = parse_glyphs(&format!("A\n{rows}#.\n\nB\n{rows}.#\n"), 6).unwrap();
        assert_eq!(two[1].0, 'B');
        assert_eq!(two[1].1.lines[5], 0x4000_0000);
    }

    #[test]
    fn topaz_pixels() {
        let one = Font::TOPAZ.glyph('1').unwrap();
        assert_eq!(one.width, 8);
        let rows: Vec<String> = (0..8)
            .map(|y| {
                (0..8)
                    .map(|x| if one.is_set(x, y) { '#' } else { '.' })
                    .collect()
            })
            .collect();
        // with a flag and no base
        assert_eq!(
            rows,
            [
                "...##...", "..###...", ".####...", "...##...", "...##...", "...##...", "...##...",
                "........"
            ]
        );
    }

    #[test]
    fn glyph_runs() {
        assert_eq!(
            runs(0b1100_0110 << 24).collect::<Vec<_>>(),
            [(0, 2), (5, 2)]
        );
        assert_eq!(runs(0).count(), 0);
        assert_eq!(runs(u32::MAX).collect::<Vec<_>>(), [(0, 32)]);
    }

    #[test]
    fn spacing() {
        assert_eq!(SPACED_FONT.width("i"), 6);
        assert_eq!(SPACED_FONT.width("I~"), 6 + 6);
        assert_eq!(SPACED_FONT.blank_sides('I'), Some((2, 2)));
        assert_eq!(SPACED_FONT.blank_sides(' '), None);
    }

    #[test]
    fn fonts_compare_by_bitmap() {
        assert_eq!(SPACED_FONT, Font::new(|| &SPACED));
        assert_ne!(SPACED_FONT, Font::TOPAZ);
        assert_eq!(format!("{:?}", Font::TOPAZ), "Font(\"Topaz\")");
    }

    #[test]
    fn glyph_helpers() {
        let glyph = Glyph {
            width: 6,
            lines: vec![0b0011_0000 << 24],
        };
        assert_eq!(
            glyph.trimmed(),
            Glyph {
                width: 2,
                lines: vec![0b11 << 30]
            }
        );
        let margins = glyph.trimmed().with_margins(1, 2);
        assert_eq!((margins.width, margins.lines[0]), (5, 0b11 << 29));
        let wide = glyph.widened();
        assert_eq!((wide.width, wide.lines[0]), (12, 0b1111 << 24));
    }
}
