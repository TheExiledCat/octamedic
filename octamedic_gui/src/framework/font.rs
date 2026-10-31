// OctaMED's own fonts aren't available anywhere, so they're drawn by hand in assets/fonts.

use std::{collections::HashMap, sync::LazyLock};

use ab_glyph::{Font as _, FontRef, PxScale, point};

use crate::framework::painter::{CHAR_HEIGHT, CHAR_WIDTH};

/// Kickstart 2's Topaz, as OctaMED shows it.
const TOPAZ_TTF: &[u8] = include_bytes!("../../assets/fonts/AmigaTopazUnicodeRus.ttf");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Font {
    #[default]
    Topaz,
    /// Topaz with OctaMED's own `/` and bolder `⁄`.
    Interface,
    /// Gadget labels.
    Panel,
    /// The status row's flags like `CHRD`.
    Small,
    /// The play panel's labels like `PLAY SONG`.
    Label,
    Title,
    Author,
    /// The block editor's 16 pixel wide characters, for blocks with few tracks. Falls back to
    /// double-width Topaz.
    Editor,
}

/// OctaMED's bolder slash between a position and a length, like `0001⁄0022`.
pub const POSITION_SLASH: char = '\u{2044}';

const MAX_GLYPH_WIDTH: usize = u32::BITS as usize;

const LEFT_PIXEL: u32 = 1 << (u32::BITS - 1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub width: i32,
    /// Highest bit leftmost.
    pub lines: Vec<u32>,
}

impl Font {
    pub fn height(self) -> i32 {
        return self.bitmap().height;
    }

    pub fn glyph(self, c: char) -> Option<&'static Glyph> {
        return self.bitmap().glyph(c);
    }

    pub fn advance(self, c: char) -> i32 {
        let font = self.bitmap();
        let width = self.glyph(c).map_or(font.space, |glyph| glyph.width);
        return width + 2 * font.bearing;
    }

    pub fn bearing(self) -> i32 {
        return self.bitmap().bearing;
    }

    pub fn width(self, text: &str) -> i32 {
        return text.chars().map(|c| self.advance(c)).sum();
    }

    /// Blank pixels either side of `c`'s ink, bearing included; `None` when it has no ink.
    pub fn blank_sides(self, c: char) -> Option<(i32, i32)> {
        let ink = self.glyph(c)?.lines.iter().fold(0, |all, &line| all | line);
        if ink == 0 {
            return None;
        }
        let left = self.bearing() + ink.leading_zeros() as i32;
        let ink_end = self.bearing() + (u32::BITS - ink.trailing_zeros()) as i32;
        return Some((left, self.advance(c) - ink_end));
    }

    /// `(start, length)` of each run of ink in glyph line `line` of `text`.
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

    fn bitmap(self) -> &'static BitmapFont {
        return match self {
            Font::Topaz => &TOPAZ,
            Font::Interface => &INTERFACE,
            Font::Panel => &PANEL,
            Font::Small => &SMALL,
            Font::Label => &LABEL,
            Font::Editor => &EDITOR,
            Font::Title => &TITLE,
            Font::Author => &AUTHOR,
        };
    }
}

/// `(start, length)` of each run of set pixels in a glyph line.
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
    return line & (LEFT_PIXEL >> x) != 0;
}

#[derive(Default)]
struct BitmapFont {
    glyphs: HashMap<char, Glyph>,
    height: i32,
    bearing: i32,
    /// Used when the font has no glyph for a space.
    space: i32,
    capitals_only: bool,
}

impl BitmapFont {
    fn with_glyphs(mut self, name: &str, text: &str) -> Self {
        let glyphs = parse_glyphs(text, self.height as usize)
            .unwrap_or_else(|error| panic!("assets/fonts/{name}: {error}"));
        self.glyphs.extend(glyphs);
        return self;
    }

    fn with_fallback(mut self, fallback: &BitmapFont, convert: fn(&Glyph) -> Glyph) -> Self {
        for (&c, glyph) in &fallback.glyphs {
            self.glyphs.entry(c).or_insert_with(|| convert(glyph));
        }
        return self;
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

/// Rasterized from the TTF so it draws without blurring.
static TOPAZ: LazyLock<BitmapFont> = LazyLock::new(|| BitmapFont {
    glyphs: rasterize(TOPAZ_TTF).unwrap_or_else(|| panic!("Topaz's TTF couldn't be read")),
    height: CHAR_HEIGHT,
    space: CHAR_WIDTH,
    ..Default::default()
});

/// Reads back the pixels of a bitmap font in TTF form. Topaz's TTF is the 8 line font at double
/// height with smoothing half lines between, so each line takes the upper half line.
fn rasterize(ttf: &[u8]) -> Option<HashMap<char, Glyph>> {
    let font = FontRef::try_from_slice(ttf).ok()?;
    let em = font.units_per_em()?;
    let pixel = em / (2 * CHAR_HEIGHT) as f32;
    // `PxScale` is ascent to descent, not the em
    let scale = PxScale::from(font.height_unscaled() / pixel);
    let mut glyphs = HashMap::new();
    for (_, c) in font.codepoint_ids() {
        // a character can be in more than one table; this is the one that's used
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

static PANEL: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont {
        height: 6,
        bearing: 1,
        space: 4,
        capitals_only: true,
        ..Default::default()
    }
    .with_glyphs("panel.txt", include_str!("../../assets/fonts/panel.txt"))
});

static SMALL: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont {
        height: 5,
        space: 4,
        capitals_only: true,
        ..Default::default()
    }
    .with_glyphs("small.txt", include_str!("../../assets/fonts/small.txt"))
});

static TITLE: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont {
        height: 9,
        space: 13,
        ..Default::default()
    }
    .with_glyphs("title.txt", include_str!("../../assets/fonts/title.txt"))
});

static AUTHOR: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont {
        height: 5,
        space: 7,
        capitals_only: true,
        ..Default::default()
    }
    .with_glyphs("author.txt", include_str!("../../assets/fonts/author.txt"))
});

static EDITOR: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont {
        height: CHAR_HEIGHT,
        space: 2 * CHAR_WIDTH,
        ..Default::default()
    }
    .with_glyphs("editor.txt", include_str!("../../assets/fonts/editor.txt"))
    .with_fallback(&TOPAZ, widened)
});

static LABEL: LazyLock<BitmapFont> = LazyLock::new(|| {
    let mut font = BitmapFont {
        height: 5,
        space: 6,
        capitals_only: true,
        ..Default::default()
    };
    for (&c, glyph) in &SMALL.glyphs {
        let (left, right) = label_margins(c);
        font.glyphs
            .insert(c, with_margins(&trimmed(glyph), left, right));
    }
    font
});

/// Matches OctaMED's spacing in `CONT. SONG`, `VOL:` and `E1`.
fn label_margins(c: char) -> (u32, u32) {
    return match c {
        '.' => (1, 0),
        ':' => (0, 1),
        '0'..='9' => (3, 1),
        _ => (1, 1),
    };
}

fn with_margins(glyph: &Glyph, left: u32, right: u32) -> Glyph {
    return Glyph {
        width: glyph.width + (left + right) as i32,
        lines: glyph.lines.iter().map(|&line| line >> left).collect(),
    };
}

fn trimmed(glyph: &Glyph) -> Glyph {
    let ink = glyph.lines.iter().fold(0, |all, &line| all | line);
    if ink == 0 {
        return glyph.clone();
    }
    let left = ink.leading_zeros();
    return Glyph {
        width: (u32::BITS - ink.trailing_zeros() - left) as i32,
        lines: glyph.lines.iter().map(|&line| line << left).collect(),
    };
}

fn widened(glyph: &Glyph) -> Glyph {
    let lines = glyph
        .lines
        .iter()
        .map(|&line| {
            let mut wide = 0;
            for x in (0..u8::BITS).filter(|&x| is_set(line, x)) {
                wide |= (LEFT_PIXEL >> (2 * x)) | (LEFT_PIXEL >> (2 * x + 1));
            }
            wide
        })
        .collect();
    return Glyph {
        width: glyph.width * 2,
        lines,
    };
}

static INTERFACE: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont {
        height: CHAR_HEIGHT,
        space: CHAR_WIDTH,
        ..Default::default()
    }
    .with_glyphs(
        "interface.txt",
        include_str!("../../assets/fonts/interface.txt"),
    )
    .with_fallback(&TOPAZ, Glyph::clone)
});

/// Each glyph is its character on a line, then `height` lines of `#` and `.`, then an empty line.
fn parse_glyphs(text: &str, height: usize) -> Result<Vec<(char, Glyph)>, String> {
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

    #[test]
    fn glyph_rows_become_bits_from_the_left() {
        let a = Font::Panel.glyph('a').unwrap();
        assert_eq!(a.width, 8);
        assert_eq!(a.lines[0], 0b0001_1000 << 24);
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
        assert!(parse_glyphs(title, 9).is_ok());
        let author = include_str!("../../assets/fonts/author.txt");
        assert!(parse_glyphs(author, 5).is_ok());
        let small = include_str!("../../assets/fonts/small.txt");
        assert_eq!(parse_glyphs(small, 5).unwrap().len(), 33);
    }

    #[test]
    fn labels_are_the_small_capitals_2_pixels_apart() {
        let l = Font::Label.glyph('l').unwrap();
        assert_eq!((l.width, l.lines[4]), (7, 0b11111 << 26));
        // P, A and Y are 6 wide and L 5, and a space puts 6 more pixels between words
        assert_eq!(Font::Label.width("PLAY"), 6 + 5 + 6 + 6 + 4 * 2);
        assert_eq!(Font::Label.width(" "), 6);
        // further from a letter before a digit, as in E1
        assert_eq!(Font::Label.blank_sides('1'), Some((3, 1)));
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
                    .map(|x| if is_set(*line, x) { '#' } else { '.' })
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
