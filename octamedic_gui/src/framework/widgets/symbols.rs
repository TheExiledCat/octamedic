use crate::framework::{
    painter::{Align, Painter, Pen, aligned_text_x},
    rect::Rect,
    widgets::button::{ButtonContent, ContentArea},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blocks {
    pub count: i32,
}

impl Blocks {
    pub fn new(count: i32) -> Self {
        return Self { count };
    }

    const WIDTH: i32 = 8;
    const GAP: i32 = 6;
    const STRIPES: i32 = 3;
}

impl ButtonContent for Blocks {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        let inside = area.inside;
        let width = self.count * Self::WIDTH + (self.count - 1).max(0) * Self::GAP;
        let x = aligned_text_x(inside, width, Align::Center);
        for block in 0..self.count {
            let block_x = x + block * (Self::WIDTH + Self::GAP);
            for stripe in 0..Self::STRIPES {
                let line = Rect::new(block_x, inside.y + 1 + 2 * stripe, Self::WIDTH, 1);
                painter.fill(line, pen);
            }
        }
    }
}

// copied off OctaMED's picture button. each has its spacing built in so a row of them comes
// out spaced the same
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicSymbol {
    Note,
    BassClef,
    Sharp,
    // half note head with a ledger line through it
    HalfNote,
    Flat,
    Natural,
}

impl MusicSymbol {
    #[rustfmt::skip]
    fn rows(self) -> &'static [&'static str] {
        return match self {
            MusicSymbol::Note => &[
                "....###..",
                "....####.",
                "....##.##",
                ".#####..#",
                "######.#.",
                ".####....",
            ],
            MusicSymbol::BassClef => &[
                ".#######....",
                "###...###.##",
                ".###..###...",
                ".....###..##",
                "....###.....",
                "..###.......",
            ],
            MusicSymbol::Sharp => &[
                "......##..",
                "..##..####",
                "..######..",
                "####..##..",
                "..##..####",
                "..######..",
                "####..##..",
                "..##......",
            ],
            MusicSymbol::HalfNote => &[
                "...#####...",
                "..##...##..",
                "###########",
                "..##...##..",
                "...#####...",
            ],
            MusicSymbol::Flat => &[
                "##....",
                "##....",
                "#####.",
                "##..##",
                "##.##.",
                "####..",
            ],
            MusicSymbol::Natural => &[
                "##....",
                "##..##",
                "######",
                "##..##",
                "######",
                "##..##",
                "....##",
            ],
        };
    }

    // blank pixels (before, after)
    fn spacing(self) -> (i32, i32) {
        return match self {
            MusicSymbol::Note => (3, 2),
            MusicSymbol::BassClef => (3, 0),
            MusicSymbol::Sharp => (2, 0),
            MusicSymbol::HalfNote => (1, 1),
            MusicSymbol::Flat | MusicSymbol::Natural => (1, 1),
        };
    }

    fn ink_width(self) -> i32 {
        return self
            .rows()
            .iter()
            .map(|row| row.len() as i32)
            .max()
            .unwrap_or(0);
    }

    fn width(self) -> i32 {
        let (before, after) = self.spacing();
        return before + self.ink_width() + after;
    }

    fn height(self) -> i32 {
        return self.rows().len() as i32;
    }
}

impl ButtonContent for MusicSymbol {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        render_symbols(&[*self], area, pen, painter);
    }
}

impl<const N: usize> ButtonContent for [MusicSymbol; N] {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        render_symbols(self, area, pen, painter);
    }
}

fn draw_rows(rows: &[&str], x: i32, y: i32, pen: Pen, painter: &mut Painter) {
    for (line, row) in rows.iter().enumerate() {
        let mut start = None;
        for (column, c) in row.chars().chain(['.']).enumerate() {
            match (c == '#', start) {
                (true, None) => start = Some(column as i32),
                (false, Some(from)) => {
                    let run = Rect::new(x + from, y + line as i32, column as i32 - from, 1);
                    painter.fill(run, pen);
                    start = None;
                }
                _ => {}
            }
        }
    }
}

fn render_symbols(symbols: &[MusicSymbol], area: ContentArea, pen: Pen, painter: &mut Painter) {
    let width = symbols.iter().map(|symbol| symbol.width()).sum();
    let mut x = aligned_text_x(area.inside, width, area.align);
    for &symbol in symbols {
        let y = area.inside.y + (area.inside.h - symbol.height()) / 2;
        draw_rows(symbol.rows(), x + symbol.spacing().0, y, pen, painter);
        x += symbol.width();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [MusicSymbol; 6] = [
        MusicSymbol::Note,
        MusicSymbol::BassClef,
        MusicSymbol::Sharp,
        MusicSymbol::HalfNote,
        MusicSymbol::Flat,
        MusicSymbol::Natural,
    ];

    #[test]
    fn symbols_fit() {
        for symbol in ALL {
            assert!(symbol.height() <= 8, "{symbol:?}");
        }
    }

    #[test]
    fn picture_row_width() {
        let row = [
            MusicSymbol::Note,
            MusicSymbol::BassClef,
            MusicSymbol::Sharp,
            MusicSymbol::HalfNote,
        ];
        assert_eq!(row.iter().map(|symbol| symbol.width()).sum::<i32>(), 54);
    }
}
