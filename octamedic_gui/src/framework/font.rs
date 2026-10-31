// OctaMED's panel, title and editor fonts aren't in any font library, so they're drawn pixel by
// pixel in assets/fonts and read here, along with Topaz from its TTF.

use std::{collections::HashMap, sync::LazyLock};

use ab_glyph::{Font as _, FontRef, PxScale, point};

use crate::framework::painter::{CHAR_HEIGHT, CHAR_WIDTH};

/// Topaz as OctaMED shows it, Kickstart 2's.
const TOPAZ_TTF: &[u8] = include_bytes!("../../assets/fonts/AmigaTopazUnicodeRus.ttf");

/// Which font text is drawn in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Font {
    /// The Amiga's Topaz, for most text, and the block editor's for blocks too wide for `Editor`.
    #[default]
    Topaz,
    /// Topaz with OctaMED's own slashes, for numbers and names: the plain `/` between a song's
    /// number and the count, and the bolder `⁄` between a position and a length. Everything else
    /// is Topaz's.
    Interface,
    /// Square capitals, 6 lines tall at the top of the character, for gadget labels in the style
    /// of OctaMED's panels. Lowercase letters are drawn as capitals.
    Panel,
    /// Short capitals, 5 lines tall, for the status row's flags like `CHRD`.
    Small,
    /// `Small`'s capitals 2 pixels apart, for the play panel's labels like `PLAY SONG`.
    Label,
    /// The title bar's title, 9 lines tall, from OctaMED's title artwork.
    Title,
    /// The title bar's author, small capitals 5 lines tall, from OctaMED's title artwork.
    Author,
    /// The block editor's 16 pixel wide characters, for blocks of few tracks: OctaMED's own
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

/// Whether pixel `x` of a glyph line is set, from the left.
fn is_set(line: u32, x: u32) -> bool {
    return line & (LEFT_PIXEL >> x) != 0;
}

/// A glyph: how wide it is, and its lines top to bottom, the highest bit leftmost.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub width: i32,
    pub lines: Vec<u32>,
}

/// A font drawn pixel by pixel, from a glyph file (see `parse_glyphs`) or Topaz's TTF.
#[derive(Default)]
struct BitmapFont {
    glyphs: HashMap<char, Glyph>,
    /// How many lines tall the glyphs are, from the top of the character.
    height: i32,
    /// Blank pixels either side of every glyph.
    bearing: i32,
    /// How far a space moves the text along, if the font has no glyph for it.
    space: i32,
    /// Whether lowercase letters are drawn with the capitals' glyphs.
    capitals_only: bool,
}

impl BitmapFont {
    /// With the glyphs of `assets/fonts/{name}`, whose text is `text`.
    fn with_glyphs(mut self, name: &str, text: &str) -> Self {
        let glyphs = parse_glyphs(text, self.height as usize)
            .unwrap_or_else(|error| panic!("assets/fonts/{name}: {error}"));
        self.glyphs.extend(glyphs);
        return self;
    }

    /// With `fallback`'s glyphs, as `convert` makes them, for the characters it doesn't have.
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

/// Read back into pixels once from the TTF, and drawn like the glyph files, without blurring.
static TOPAZ: LazyLock<BitmapFont> = LazyLock::new(|| BitmapFont {
    glyphs: rasterize(TOPAZ_TTF).unwrap_or_else(|| panic!("Topaz's TTF couldn't be read")),
    height: CHAR_HEIGHT,
    space: CHAR_WIDTH,
    ..Default::default()
});

/// Every character of a font that's a bitmap font in disguise, each pixel a square of the outline,
/// `2 * CHAR_HEIGHT` of them to the em, as
/// `CHAR_HEIGHT` lines for the hires screen, whose pixels are twice as tall as they're wide.
/// Topaz's TTF is the 8 line font at twice the height, with half lines between them that smooth
/// it, so each line is the upper of its two half lines. At the font's own size every pixel is
/// either inside the outline or out of it; one is set when it's at least half covered.
fn rasterize(ttf: &[u8]) -> Option<HashMap<char, Glyph>> {
    let font = FontRef::try_from_slice(ttf).ok()?;
    let em = font.units_per_em()?;
    let pixel = em / (2 * CHAR_HEIGHT) as f32;
    // a `PxScale` is how many pixels tall the font is from its ascent to its descent
    let scale = PxScale::from(font.height_unscaled() / pixel);
    let mut glyphs = HashMap::new();
    for (_, c) in font.codepoint_ids() {
        // the font can map a character in more than one table; this is the one that's used
        let id = font.glyph_id(c);
        let width = (font.h_advance_unscaled(id) / pixel).round() as i32;
        let mut lines = vec![0; CHAR_HEIGHT as usize];
        // with the baseline a whole em down, the top of the em is the top of the character
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

/// Square capitals, 2 pixels apart.
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

/// Each pixel twice as wide.
/// `Small` with each letter cut to its ink and given `label_margins`.
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

/// Blank pixels before and after a label's character: one each, so letters are 2 apart, but
/// none after a full stop or before a colon, and 3 before a digit, as OctaMED spaces
/// `CONT. SONG`, `VOL:` and `E1`.
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

/// Without the blank columns at its sides.
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

/// Reads glyphs written as their character on a line of its own, then `height` lines of `#`
/// (set) and `.` (clear), all as wide as each other and no wider than `MAX_GLYPH_WIDTH`, with
/// an empty line before the next character.
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

impl Font {
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

    /// How many lines tall the glyphs are, from the top of the character.
    pub fn height(self) -> i32 {
        return self.bitmap().height;
    }

    /// The glyph `c` is drawn with; `None` for characters with no glyph, which are left blank.
    pub fn glyph(self, c: char) -> Option<&'static Glyph> {
        return self.bitmap().glyph(c);
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

    /// Blank pixels before and after the ink in `c`'s character, its bearing included; `None`
    /// when it has none, like a space.
    pub fn blank_sides(self, c: char) -> Option<(i32, i32)> {
        let ink = self.glyph(c)?.lines.iter().fold(0, |all, &line| all | line);
        if ink == 0 {
            return None;
        }
        let left = self.bearing() + ink.leading_zeros() as i32;
        let ink_end = self.bearing() + (u32::BITS - ink.trailing_zeros()) as i32;
        return Some((left, self.advance(c) - ink_end));
    }
}

/// The runs of set pixels in a glyph's line, as where each starts and how long it is.
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
