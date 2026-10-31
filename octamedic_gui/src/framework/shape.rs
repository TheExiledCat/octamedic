/// Which way a triangle points.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Which sides of a rect get a border.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sides {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

impl Sides {
    pub const ALL: Sides = Sides {
        top: true,
        right: true,
        bottom: true,
        left: true,
    };
    pub const NONE: Sides = Sides {
        top: false,
        right: false,
        bottom: false,
        left: false,
    };
}

/// A row of a shape: from `.0` up to but not including `.1`, across from its left edge.
pub type Span = (i32, i32);

/// A shape's rows, top to bottom; `None` where a row has nothing in it.
pub type Spans = Vec<Option<Span>>;

fn span(left: f32, right: f32) -> Option<Span> {
    let span = (left.round() as i32, right.round() as i32);
    return (span.0 < span.1).then_some(span);
}

/// How far row `y` of `h` is from the middle, from 0 there to 1 at the top and bottom edges.
fn from_middle(y: i32, h: i32) -> f32 {
    let half = h as f32 / 2.0;
    return ((y as f32 + 0.5 - half) / half).abs();
}

/// A triangle filling `w` by `h`, pointing at the middle of one side from the far side. When `w`
/// is a multiple of `h` it widens by the same whole number of pixels every row, so its edges
/// step evenly.
pub fn triangle(w: i32, h: i32, direction: Direction) -> Spans {
    // pointing sideways, the rows widen from the top and bottom edges in to the middle
    let half_rows = (h + 1) / 2;
    return (0..h)
        .map(|y| {
            let span = match direction {
                Direction::Up | Direction::Down => {
                    let from_tip = if direction == Direction::Up {
                        y
                    } else {
                        h - 1 - y
                    };
                    let width = w * (from_tip + 1) / h;
                    let left = (w - width) / 2;
                    (left, left + width)
                }
                Direction::Right => (0, w * (y.min(h - 1 - y) + 1) / half_rows),
                Direction::Left => (w - w * (y.min(h - 1 - y) + 1) / half_rows, w),
            };
            (span.0 < span.1).then_some(span)
        })
        .collect();
}

/// An ellipse filling `w` by `h`; a circle on screen when `w` is twice `h`, for the tall pixels.
pub fn ellipse(w: i32, h: i32) -> Spans {
    let w_f = w as f32;
    return (0..h)
        .map(|y| {
            let across = (1.0 - from_middle(y, h).powi(2)).max(0.0).sqrt();
            let half = w_f / 2.0 * across;
            span(w_f / 2.0 - half, w_f / 2.0 + half)
        })
        .collect();
}

/// What's left of a shape inside a border `thickness` thick, as `Border` is:
/// the pixels with the whole shape around them that far in every direction. Only right for
/// shapes without dents, which all of these are.
pub fn inside(spans: &Spans, thickness: i32) -> Spans {
    let rows = spans.len() as i32;
    let side = 2 * thickness;
    return (0..rows)
        .map(|y| {
            let mut inner = (i32::MIN, i32::MAX);
            for near in y - thickness..=y + thickness {
                let row = (0..rows).contains(&near).then(|| spans[near as usize]);
                let (left, right) = row.flatten()?;
                inner = (inner.0.max(left + side), inner.1.min(right - side));
            }
            (inner.0 < inner.1).then_some(inner)
        })
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draw(spans: &Spans, w: i32) -> Vec<String> {
        return spans
            .iter()
            .map(|span| {
                (0..w)
                    .map(|x| match span {
                        Some((left, right)) if (*left..*right).contains(&x) => '#',
                        _ => '.',
                    })
                    .collect()
            })
            .collect();
    }

    #[test]
    fn triangles_point_where_they_are_told() {
        assert_eq!(
            draw(&triangle(8, 4, Direction::Up), 8),
            ["...##...", "..####..", ".######.", "########"]
        );
        assert_eq!(
            draw(&triangle(8, 4, Direction::Down), 8),
            ["########", ".######.", "..####..", "...##..."]
        );
        assert_eq!(
            draw(&triangle(8, 4, Direction::Right), 8),
            ["####....", "########", "########", "####...."]
        );
        assert_eq!(
            draw(&triangle(8, 4, Direction::Left), 8),
            ["....####", "########", "########", "....####"]
        );
    }

    #[test]
    fn an_ellipse_is_round_and_fills_its_middle() {
        assert_eq!(
            draw(&ellipse(12, 6), 12),
            [
                "...######...",
                ".##########.",
                "############",
                "############",
                ".##########.",
                "...######...",
            ]
        );
    }

    #[test]
    fn the_inside_is_inset_twice_as_far_across_as_down() {
        let spans = vec![Some((0, 10)); 4];
        assert_eq!(
            draw(&inside(&spans, 1), 10),
            ["..........", "..######..", "..######..", ".........."]
        );
    }

    #[test]
    fn a_shape_too_small_for_its_border_has_no_inside() {
        let spans = vec![Some((0, 4)); 3];
        assert!(inside(&spans, 1).iter().all(Option::is_none));
    }
}
