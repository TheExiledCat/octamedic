// OctaMED's own fonts aren't around anywhere so they're redrawn by hand in assets/fonts

use std::sync::LazyLock;

use rustuition::{
    font::{BitmapFont, Font, Glyph},
    painter::{CHAR_HEIGHT, CHAR_WIDTH},
};

// topaz with OctaMED's own slashes
pub const INTERFACE: Font = Font::new(|| &INTERFACE_FONT);

// gadget labels
pub const PANEL: Font = Font::new(|| &PANEL_FONT);

// status row flags, CHRD etc
pub const SMALL: Font = Font::new(|| &SMALL_FONT);

// play panel labels like PLAY SONG
pub const LABEL: Font = Font::new(|| &LABEL_FONT);
// LABEL with 1px between letters, for the ones OctaMED squeezed in like SAVE FORMAT:
pub const NARROW_LABEL: Font = Font::new(|| &NARROW_LABEL_FONT);

pub const TITLE: Font = Font::new(|| &TITLE_FONT);

pub const AUTHOR: Font = Font::new(|| &AUTHOR_FONT);

// 16px wide block editor chars for blocks with few tracks, double width topaz otherwise
pub const EDITOR: Font = Font::new(|| &EDITOR_FONT);

// OctaMED's fatter slash between a position and a length, e.g. 0001/0022
pub const POSITION_SLASH: char = '\u{2044}';

const PANEL_GLYPHS: &str = include_str!("../../assets/fonts/panel.txt");
const SMALL_GLYPHS: &str = include_str!("../../assets/fonts/small.txt");
const TITLE_GLYPHS: &str = include_str!("../../assets/fonts/title.txt");
const AUTHOR_GLYPHS: &str = include_str!("../../assets/fonts/author.txt");
const EDITOR_GLYPHS: &str = include_str!("../../assets/fonts/editor.txt");
const INTERFACE_GLYPHS: &str = include_str!("../../assets/fonts/interface.txt");

static PANEL_FONT: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont::new("panel", 6)
        .bearing(1)
        .space(4)
        .capitals_only()
        .with_glyphs(PANEL_GLYPHS)
});

static SMALL_FONT: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont::new("small", 5)
        .space(4)
        .capitals_only()
        .with_glyphs(SMALL_GLYPHS)
});

static TITLE_FONT: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont::new("title", 9)
        .space(13)
        .with_glyphs(TITLE_GLYPHS)
});

static AUTHOR_FONT: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont::new("author", 5)
        .space(7)
        .capitals_only()
        .with_glyphs(AUTHOR_GLYPHS)
});

static EDITOR_FONT: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont::new("editor", CHAR_HEIGHT)
        .space(2 * CHAR_WIDTH)
        .with_glyphs(EDITOR_GLYPHS)
        .with_fallback(Font::TOPAZ, Glyph::widened)
});

static LABEL_FONT: LazyLock<BitmapFont> = LazyLock::new(|| {
    let font = BitmapFont::new("label", 5).space(6).capitals_only();
    return SMALL.bitmap().glyphs().fold(font, |font, (c, glyph)| {
        let (left, right) = label_margins(c);
        font.with_glyph(c, glyph.trimmed().with_margins(left, right))
    });
});

static NARROW_LABEL_FONT: LazyLock<BitmapFont> = LazyLock::new(|| {
    let font = BitmapFont::new("narrow label", 5).space(5).capitals_only();
    return SMALL.bitmap().glyphs().fold(font, |font, (c, glyph)| {
        let left = if c.is_ascii_digit() { 1 } else { 0 };
        font.with_glyph(c, glyph.trimmed().with_margins(left, 1))
    });
});

static INTERFACE_FONT: LazyLock<BitmapFont> = LazyLock::new(|| {
    BitmapFont::new("interface", CHAR_HEIGHT)
        .space(CHAR_WIDTH)
        .with_glyphs(INTERFACE_GLYPHS)
        .with_fallback(Font::TOPAZ, Glyph::clone)
});

// matched against CONT. SONG, VOL: and E1 on the real thing
fn label_margins(c: char) -> (u32, u32) {
    return match c {
        '.' => (1, 0),
        ':' => (0, 1),
        '0'..='9' => (3, 1),
        _ => (1, 1),
    };
}

#[cfg(test)]
mod tests {
    use rustuition::font::parse_glyphs;

    use super::*;

    #[test]
    fn glyph_bits() {
        let a = PANEL.glyph('a').unwrap();
        assert_eq!(a.width, 8);
        assert_eq!(a.lines[0], 0b0001_1000 << 24);
        assert_eq!(PANEL.glyph('I').unwrap().width, 4);
        assert!(PANEL.glyph('~').is_none());
        assert_eq!(INTERFACE.glyph('/').unwrap().width, 10);
        assert_eq!(INTERFACE.glyph('a'), Font::TOPAZ.glyph('a'));
    }

    #[test]
    fn font_files_parse() {
        assert!(parse_glyphs(PANEL_GLYPHS, 6).unwrap().len() >= 26);
        assert_eq!(parse_glyphs(INTERFACE_GLYPHS, 8).unwrap().len(), 2);
        assert!(parse_glyphs(TITLE_GLYPHS, 9).is_ok());
        assert!(parse_glyphs(AUTHOR_GLYPHS, 5).is_ok());
        assert_eq!(parse_glyphs(SMALL_GLYPHS, 5).unwrap().len(), 42);
        assert!(parse_glyphs(EDITOR_GLYPHS, 8).is_ok());
    }

    #[test]
    fn label_spacing() {
        let l = LABEL.glyph('l').unwrap();
        assert_eq!((l.width, l.lines[4]), (7, 0b11111 << 26));
        // P A Y are 6 wide, L 5, 2 between each
        assert_eq!(LABEL.width("PLAY"), 6 + 5 + 6 + 6 + 4 * 2);
        assert_eq!(LABEL.width(" "), 6);
        // digits sit further from the letter before, see E1
        assert_eq!(LABEL.blank_sides('1'), Some((3, 1)));
    }

    #[test]
    fn narrow_label_spacing() {
        // S A V E are 6 6 6 5 wide, 1 between each
        assert_eq!(NARROW_LABEL.width("SAVE"), 6 + 6 + 6 + 5 + 4);
        // 6 between words, counting E's 1
        assert_eq!(NARROW_LABEL.width("E F") - NARROW_LABEL.width("EF"), 5);
    }

    #[test]
    fn editor_font() {
        let a = EDITOR.glyph('A').unwrap();
        assert_eq!(a.width, 16);
        assert_eq!(a.lines, Font::TOPAZ.glyph('A').unwrap().widened().lines);
        // an empty note's dashes stay apart
        let dash = EDITOR.glyph('-').unwrap();
        assert_eq!(dash.lines[3], 0b0001_1111_1111_1000 << 16);
    }

    #[test]
    fn panel_spacing() {
        assert_eq!(PANEL.width("FI"), 10 + 6);
    }

    #[test]
    fn interface_widths() {
        assert_eq!(INTERFACE.width("0001"), 32);
        assert_eq!(INTERFACE.width("/"), 10);
        assert_eq!(INTERFACE.width(&POSITION_SLASH.to_string()), 12);
        // not in the font, falls back to topaz
        assert_eq!(INTERFACE.width("~ "), 16);
    }
}
