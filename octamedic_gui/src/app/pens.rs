use rustuition::palette::Pen;

// OctaMED's colours past rustuition's background, shine, shadow and fill
pub const RED: Pen = Pen(4);
pub const ROSE: Pen = Pen(5);
pub const PINK: Pen = Pen(6);
pub const BLUSH: Pen = Pen(7);

// OctaMED highlights by flipping the lowest bitplane, which swaps each pen for its neighbour:
// black and light grey, the band's greys, pink and blush, red and rose
pub fn highlight(pen: Pen) -> Pen {
    return Pen(pen.0 ^ 1);
}
