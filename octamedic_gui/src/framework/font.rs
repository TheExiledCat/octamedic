/*This file is here because OctaMED uses some unspecified font that i couldnt find in any libraries.
so i went pixel by pixel and made a glyph renderer that can read a text file just for this purpose.
*/

use std::{collections::HashMap, sync::LazyLock};

use ab_glyph::{Font as _, FontRef, OutlineCurve, Point};

use crate::framework::painter::{CHAR_HEIGHT, CHAR_WIDTH};

/// Topaz as OctaMED shows it, Kickstart 2's.
pub const TOPAZ_TTF: &[u8] = include_bytes!("../../assets/fonts/AmigaTopazUnicodeRus.ttf");

/// Which font text is drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Font {
    /// The Amiga's Topaz, for the block editor.
    #[default]
    Topaz,
    /// Topaz with OctaMED's own slashes, for numbers and names: the plain `/` between a song's
    /// number and the count, and the bolder `⁄` between a position and a length. Everything else
    /// is drawn in Topaz.
    Interface,
    /// Square capitals, 6 lines tall at the top of the character, for gadget labels in the style
    /// of OctaMED's panels. Lowercase letters are drawn as capitals.
    Panel,
    /// Short capitals, 5 lines tall, for the status row's flags like `CHRD`.
    Small,
    /// The title bar's title, 9 lines tall, from OctaMED's title artwork.
    Title,
    /// The title bar's author, small capitals 5 lines tall, from OctaMED's title artwork.
    Author,
    /// The block editor's 16 pixel wide characters, for blocks of up to 4 tracks: OctaMED's own
    /// where `editor.txt` has them, like the separate dashes of an empty note, and Topaz twice as
    /// wide for the rest.
    Editor,
}

/// OctaMED's bolder slash between a position and a length, like `0001⁄0022`, which
/// `Font::Interface` draws. It's the fraction slash, so it doesn't need typing.
pub const POSITION_SLASH: char = '\u{2044}';

/// The widest glyph a font file can hold.
const MAX_GLYPH_WIDTH: usize = u32::BITS as usize;

/// A glyph line's leftmost pixel; the pixels go right from the highest bit.
const LEFT_PIXEL: u32 = 1 << (u32::BITS - 1);

const LEFT_TWO_PIXELS: u32 = 0b11 << (u32::BITS - 2);

// the glyph files' fonts: lines tall, blank pixels each side of a glyph, and a space's width
const PANEL_HEIGHT: usize = 6;
const PANEL_BEARING: i32 = 1;
const PANEL_SPACE: i32 = 4;
const SMALL_HEIGHT: usize = 5;
const SMALL_SPACE: i32 = 4;
const TITLE_HEIGHT: usize = 9;
const TITLE_SPACE: i32 = 13;
const AUTHOR_HEIGHT: usize = 5;
const AUTHOR_SPACE: i32 = 7;

/// A glyph: how wide it is, and its lines top to bottom, the highest bit leftmost.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub width: i32,
    pub lines: Vec<u32>,
}

/// A font drawn pixel by pixel, from a glyph file (see `parse_glyphs`) or Topaz's TTF.
struct BitmapFont {
    glyphs: HashMap<char, Glyph>,
    /// Blank pixels either side of every glyph.
    bearing: i32,
    /// How far a space moves the text along, if the font has no glyph for it.
    space: i32,
    /// Whether lowercase letters are drawn with the capitals' glyphs.
    capitals_only: bool,
}

impl BitmapFont {
    fn load(name: &str, text: &str, height: usize, bearing: i32, space: i32) -> Self {
        let glyphs = parse_glyphs(text, height)
            .unwrap_or_else(|error| panic!("assets/fonts/{name}: {error}"));
        return Self {
            glyphs: glyphs.into_iter().collect(),
            bearing,
            space,
            capitals_only: false,
        };
    }

    fn glyph(&self, c: char) -> Option<&Glyph> {
        let c = if self.capitals_only {
            c.to_ascii_uppercase()
        } else {
            c
        };
        return self.glyphs.get(&c);
    }
}

/// Topaz's TTF is a bitmap font in disguise: every pixel is a square of the outline. So it's read
/// back into pixels once, and drawn like the glyph files, without blurring.
static TOPAZ: LazyLock<BitmapFont> = LazyLock::new(|| BitmapFont {
    glyphs: rasterize(TOPAZ_TTF).unwrap_or_else(|| panic!("Topaz's TTF couldn't be read")),
    bearing: 0,
    space: CHAR_WIDTH,
    capitals_only: false,
});

/// Every character of a font made of square pixels, `CHAR_HEIGHT` of them to the em and twice as
/// tall as they're wide, for the hires screen. A pixel is set when its middle is inside the
/// glyph's outline.
fn rasterize(ttf: &[u8]) -> Option<HashMap<char, Glyph>> {
    let font = FontRef::try_from_slice(ttf).ok()?;
    let em = font.units_per_em()?;
    let (row, column) = (em / CHAR_HEIGHT as f32, em / (2 * CHAR_HEIGHT) as f32);
    let mut glyphs = HashMap::new();
    for (_, c) in font.codepoint_ids() {
        // the font can map a character in more than one table; this is the one that's used
        let id = font.glyph_id(c);
        let width = (font.h_advance_unscaled(id) / column).round() as usize;
        let edges = font
            .outline(id)
            .map_or(Vec::new(), |outline| edges(&outline.curves));
        let lines = (0..CHAR_HEIGHT)
            .map(|y| {
                let middle_y = em - (y as f32 + 0.5) * row;
                (0..width.min(MAX_GLYPH_WIDTH)).fold(0u32, |line, x| {
                    let middle = Point {
                        x: (x as f32 + 0.5) * column,
                        y: middle_y,
                    };
                    if inside(&edges, middle) {
                        line | (LEFT_PIXEL >> x)
                    } else {
                        line
                    }
                })
            })
            .collect();
        let width = width as i32;
        glyphs.insert(c, Glyph { width, lines });
    }
    return Some(glyphs);
}

/// An outline as straight edges; curves are cut into short ones.
fn edges(curves: &[OutlineCurve]) -> Vec<(Point, Point)> {
    let mut edges = Vec::new();
    for curve in curves {
        match *curve {
            OutlineCurve::Line(a, b) => edges.push((a, b)),
            OutlineCurve::Quad(a, b, c) => cut(&mut edges, |t| {
                let u = 1.0 - t;
                weighted(&[(a, u * u), (b, 2.0 * u * t), (c, t * t)])
            }),
            OutlineCurve::Cubic(a, b, c, d) => cut(&mut edges, |t| {
                let u = 1.0 - t;
                let weights = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                weighted(&[
                    (a, weights[0]),
                    (b, weights[1]),
                    (c, weights[2]),
                    (d, weights[3]),
                ])
            }),
        }
    }
    return edges;
}

/// Adds the curve through `point_at(0)` to `point_at(1)` as short straight edges.
fn cut(edges: &mut Vec<(Point, Point)>, point_at: impl Fn(f32) -> Point) {
    const STEPS: usize = 8;
    let at = |step: usize| point_at(step as f32 / STEPS as f32);
    for step in 0..STEPS {
        edges.push((at(step), at(step + 1)));
    }
}

fn weighted(points: &[(Point, f32)]) -> Point {
    return points
        .iter()
        .fold(Point { x: 0.0, y: 0.0 }, |sum, &(point, weight)| Point {
            x: sum.x + point.x * weight,
            y: sum.y + point.y * weight,
        });
}

/// Whether `point` is inside the outline, by the nonzero rule fonts are filled with: edges
/// crossing a line from it to the right count one way going up and the other going down. Some
/// fonts draw each contour twice, which the even-odd rule would cancel out.
fn inside(edges: &[(Point, Point)], point: Point) -> bool {
    let winding: i32 = edges
        .iter()
        .filter(|(a, b)| {
            (a.y > point.y) != (b.y > point.y)
                && point.x < a.x + (point.y - a.y) / (b.y - a.y) * (b.x - a.x)
        })
        .map(|(a, b)| if b.y > a.y { 1 } else { -1 })
        .sum();
    return winding != 0;
}

static PANEL: LazyLock<BitmapFont> = LazyLock::new(|| BitmapFont {
    capitals_only: true,
    ..BitmapFont::load(
        "panel.txt",
        include_str!("../../assets/fonts/panel.txt"),
        PANEL_HEIGHT,
        PANEL_BEARING,
        PANEL_SPACE,
    )
});

static SMALL: LazyLock<BitmapFont> = LazyLock::new(|| BitmapFont {
    capitals_only: true,
    ..BitmapFont::load(
        "small.txt",
        include_str!("../../assets/fonts/small.txt"),
        SMALL_HEIGHT,
        0,
        SMALL_SPACE,
    )
});

static TITLE: LazyLock<BitmapFont> = LazyLock::new(|| {
    let text = include_str!("../../assets/fonts/title.txt");
    BitmapFont::load("title.txt", text, TITLE_HEIGHT, 0, TITLE_SPACE)
});

static AUTHOR: LazyLock<BitmapFont> = LazyLock::new(|| BitmapFont {
    capitals_only: true,
    ..BitmapFont::load(
        "author.txt",
        include_str!("../../assets/fonts/author.txt"),
        AUTHOR_HEIGHT,
        0,
        AUTHOR_SPACE,
    )
});

static EDITOR: LazyLock<BitmapFont> = LazyLock::new(|| {
    let text = include_str!("../../assets/fonts/editor.txt");
    let mut font = BitmapFont::load("editor.txt", text, CHAR_HEIGHT as usize, 0, 2 * CHAR_WIDTH);
    for (&c, glyph) in &TOPAZ.glyphs {
        font.glyphs.entry(c).or_insert_with(|| widened(glyph));
    }
    font
});

fn widened(glyph: &Glyph) -> Glyph {
    let lines = glyph
        .lines
        .iter()
        .map(|&line| {
            (0..u8::BITS).fold(0u32, |wide, x| {
                if line & (LEFT_PIXEL >> x) != 0 {
                    wide | (LEFT_TWO_PIXELS >> (2 * x))
                } else {
                    wide
                }
            })
        })
        .collect();
    return Glyph {
        width: glyph.width * 2,
        lines,
    };
}

static INTERFACE: LazyLock<BitmapFont> = LazyLock::new(|| {
    let text = include_str!("../../assets/fonts/interface.txt");
    BitmapFont::load("interface.txt", text, CHAR_HEIGHT as usize, 0, CHAR_WIDTH)
});

/// Reads glyphs written as their character on a line of its own, then `height` lines of `#`
/// (set) and `.` (clear), all as wide as each other and no wider than `MAX_GLYPH_WIDTH`, with
/// an empty line before the next character.
fn parse_glyphs(text: &str, height: usize) -> Result<Vec<(char, Glyph)>, String> {
    let mut glyphs = Vec::new();
    let mut lines = text
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .peekable();
    while lines.peek().is_some() {
        let header = lines.next().unwrap_or_default();
        if header.is_empty() {
            continue;
        }
        let mut chars = header.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return Err(format!("`{header}` should be a single character"));
        };
        let mut glyph = Glyph {
            width: 0,
            lines: vec![0; height],
        };
        for (y, line) in glyph.lines.iter_mut().enumerate() {
            let row = lines.next().unwrap_or_default();
            let width = row.chars().count();
            if row.is_empty() || width > MAX_GLYPH_WIDTH || (y > 0 && width as i32 != glyph.width) {
                return Err(format!(
                    "`{c}` line {}: `{row}` should be 1 to {MAX_GLYPH_WIDTH} wide, as wide as \
                     the line above",
                    y + 1
                ));
            }
            glyph.width = width as i32;
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
        if lines.next().is_some_and(|line| !line.is_empty()) {
            return Err(format!(
                "`{c}` should be {height} lines, then an empty line"
            ));
        }
        glyphs.push((c, glyph));
    }
    return Ok(glyphs);
}

impl Font {
    fn bitmap(self) -> &'static BitmapFont {
        return match self {
            Font::Topaz => &TOPAZ,
            Font::Interface => &INTERFACE,
            Font::Panel => &PANEL,
            Font::Small => &SMALL,
            Font::Editor => &EDITOR,
            Font::Title => &TITLE,
            Font::Author => &AUTHOR,
        };
    }

    /// How many lines tall the glyphs are, from the top of the character.
    pub fn height(self) -> i32 {
        return match self {
            Font::Topaz | Font::Interface | Font::Editor => CHAR_HEIGHT,
            Font::Panel => PANEL_HEIGHT as i32,
            Font::Small => SMALL_HEIGHT as i32,
            Font::Title => TITLE_HEIGHT as i32,
            Font::Author => AUTHOR_HEIGHT as i32,
        };
    }

    /// The glyph `c` is drawn with, Topaz's for the characters `Interface` doesn't have; `None`
    /// for characters with no glyph, which are left blank.
    pub fn glyph(self, c: char) -> Option<&'static Glyph> {
        let glyph = self.bitmap().glyph(c);
        if glyph.is_none() && self.falls_back_to_topaz() {
            return TOPAZ.glyph(c);
        }
        return glyph;
    }

    /// Whether characters the font doesn't have are drawn in Topaz, rather than left blank.
    pub fn falls_back_to_topaz(self) -> bool {
        return self == Font::Interface;
    }

    pub fn advance(self, c: char) -> i32 {
        let font = self.bitmap();
        let width = self.glyph(c).map_or(font.space, |glyph| glyph.width);
        return width + 2 * font.bearing;
    }

    /// Where a glyph's ink starts in its character.
    pub fn bearing(self) -> i32 {
        return self.bitmap().bearing;
    }

    pub fn width(self, text: &str) -> i32 {
        return text.chars().map(|c| self.advance(c)).sum();
    }
}

/// The runs of set pixels in a glyph's line, as where each starts and how long it is.
pub fn runs(line: u32) -> impl Iterator<Item = (i32, i32)> {
    let mut x = 0;
    let width = MAX_GLYPH_WIDTH as i32;
    return std::iter::from_fn(move || {
        while x < width && line & (LEFT_PIXEL >> x) == 0 {
            x += 1;
        }
        if x == width {
            return None;
        }
        let start = x;
        while x < width && line & (LEFT_PIXEL >> x) != 0 {
            x += 1;
        }
        return Some((start, x - start));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_rows_become_bits_from_the_left() {
        let a = Font::Panel.glyph('a').unwrap();
        assert_eq!(a.width, 8);
        assert_eq!(a.lines[0], 0b0111_1110 << 24);
        assert_eq!(Font::Panel.glyph('I').unwrap().width, 4);
        assert!(Font::Panel.glyph('~').is_none());
        assert_eq!(Font::Interface.glyph('/').unwrap().width, 10);
        assert_eq!(Font::Interface.glyph('a'), Font::Topaz.glyph('a'));
    }

    #[test]
    fn the_font_files_parse() {
        let panel = parse_glyphs(include_str!("../../assets/fonts/panel.txt"), 6).unwrap();
        assert!(panel.len() >= 26);
        let text = include_str!("../../assets/fonts/interface.txt");
        assert_eq!(parse_glyphs(text, 8).unwrap().len(), 2);
        let title = include_str!("../../assets/fonts/title.txt");
        assert!(parse_glyphs(title, TITLE_HEIGHT).is_ok());
        let author = include_str!("../../assets/fonts/author.txt");
        assert!(parse_glyphs(author, AUTHOR_HEIGHT).is_ok());
        let small = include_str!("../../assets/fonts/small.txt");
        assert_eq!(parse_glyphs(small, 5).unwrap().len(), 10);
    }

    #[test]
    fn badly_drawn_glyphs_are_rejected() {
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
    fn topaz_is_read_back_into_its_pixels() {
        let one = Font::Topaz.glyph('1').unwrap();
        assert_eq!(one.width, 8);
        let rows: Vec<String> = one
            .lines
            .iter()
            .map(|line| {
                (0..8)
                    .map(|x| {
                        if line & (LEFT_PIXEL >> x) != 0 {
                            '#'
                        } else {
                            '.'
                        }
                    })
                    .collect()
            })
            .collect();
        // OctaMED's 1, with a flag and no base
        assert_eq!(
            rows,
            [
                "...##...", "..###...", ".####...", "...##...", "...##...", "...##...", "...##...",
                "........"
            ]
        );
    }

    #[test]
    fn the_editor_font_is_topaz_twice_as_wide_with_octameds_own_glyphs() {
        let a = Font::Editor.glyph('A').unwrap();
        assert_eq!(a.width, 16);
        assert_eq!(a.lines, widened(Font::Topaz.glyph('A').unwrap()).lines);
        // an empty note's dashes stay apart
        let dash = Font::Editor.glyph('-').unwrap();
        assert_eq!(dash.lines[3], 0b0001_1111_1111_1000 << 16);
    }

    #[test]
    fn lines_split_into_runs() {
        assert_eq!(
            runs(0b1100_0110 << 24).collect::<Vec<_>>(),
            [(0, 2), (5, 2)]
        );
        assert_eq!(runs(0).count(), 0);
        assert_eq!(runs(u32::MAX).collect::<Vec<_>>(), [(0, 32)]);
    }

    #[test]
    fn panel_glyphs_are_2_pixels_apart() {
        assert_eq!(Font::Panel.width("FI"), 10 + 6);
    }

    #[test]
    fn interface_text_is_8_wide_with_a_wider_slash() {
        assert_eq!(Font::Interface.width("0001"), 32);
        assert_eq!(Font::Interface.width("/"), 10);
        // the bolder slash between a position and a length
        assert_eq!(Font::Interface.width(&POSITION_SLASH.to_string()), 12);
        // not in the font, so drawn in Topaz
        assert_eq!(Font::Interface.width("~ "), 16);
    }
}
