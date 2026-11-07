use rustuition::{
    font::{Font, Glyph},
    gadgets::button::ContentArea,
    painter::{Painter, Pen, inside_frame},
    rect::Rect,
};

/// OctaMED drew some panels' labels by hand rather than centring them: each word with where its
/// first pixel is from the button's left.
pub type Words = &'static [(i32, &'static str)];

// the labels' top from the button's
const LABEL_Y: i32 = 3;

/// A button label of `words` in `font`, at the label row.
pub fn placed(font: Font, words: Words) -> impl Fn(ContentArea, Pen, &mut Painter) {
    return move |area, pen, painter| {
        let [left, top] = button_origin(area);
        draw_words(painter, font, [left, top + LABEL_Y], words, pen);
    };
}

/// Each of `words` in `font`, its first pixel `x` along from `left`.
pub fn draw_words(
    painter: &mut Painter,
    font: Font,
    [left, top]: [i32; 2],
    words: Words,
    pen: Pen,
) {
    for &(x, word) in words {
        let bearing = word
            .chars()
            .next()
            .and_then(|c| font.glyph(c))
            .map_or(0, blank_columns);
        painter.text_in_font(left + x - bearing, top, word, pen, font);
    }
}

// a glyph's empty columns on its left
fn blank_columns(glyph: &Glyph) -> i32 {
    let lit = glyph.lines.iter().filter(|&&line| line != 0);
    return lit
        .map(|line| line.leading_zeros() as i32)
        .min()
        .unwrap_or(0);
}

/// A picture drawn a pixel per '#', from where its top left is in the button.
pub struct Picture {
    pub at: [i32; 2],
    pub rows: &'static [&'static str],
}

/// A button label of `picture`.
pub fn picture(picture: &'static Picture) -> impl Fn(ContentArea, Pen, &mut Painter) {
    return move |area, pen, painter| {
        let [left, top] = button_origin(area);
        let [x, y] = picture.at;
        draw_pixels(painter, [left + x, top + y], picture.rows, pen);
    };
}

/// A pixel for each '#' in `rows`, from `left` and `top`.
pub fn draw_pixels(painter: &mut Painter, [left, top]: [i32; 2], rows: &[&str], pen: Pen) {
    for (row, line) in rows.iter().enumerate() {
        for (column, pixel) in line.bytes().enumerate() {
            if pixel == b'#' {
                let pixel = Rect::new(left + column as i32, top + row as i32, 1, 1);
                painter.fill(pixel, pen);
            }
        }
    }
}

// the button's own top left, outside its frame
fn button_origin(area: ContentArea) -> [i32; 2] {
    let frame = inside_frame(Rect::new(0, 0, 0, 0), 1);
    return [area.inside.x - frame.x, area.inside.y - frame.y];
}
