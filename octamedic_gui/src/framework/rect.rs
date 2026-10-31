pub type Point = [i32; 2];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        return Self { x, y, w, h };
    }

    pub const fn right(self) -> i32 {
        return self.x + self.w;
    }

    pub const fn bottom(self) -> i32 {
        return self.y + self.h;
    }

    /// Excludes the right and bottom edges.
    pub fn contains(self, point: Point) -> bool {
        let [x, y] = point;
        return x >= self.x && x < self.right() && y >= self.y && y < self.bottom();
    }

    pub fn split_top(self, height: i32) -> (Self, Self) {
        let height = height.clamp(0, self.h);
        let top = Self::new(self.x, self.y, self.w, height);
        let rest = Self::new(self.x, self.y + height, self.w, self.h - height);
        return (top, rest);
    }

    pub fn split_left(self, width: i32) -> (Self, Self) {
        let width = width.clamp(0, self.w);
        let left = Self::new(self.x, self.y, width, self.h);
        let rest = Self::new(self.x + width, self.y, self.w - width, self.h);
        return (left, rest);
    }

    pub fn columns(self, widths: &[i32]) -> Vec<Self> {
        let mut x = self.x;
        return widths
            .iter()
            .map(|&width| {
                let column = Self::new(x, self.y, width, self.h);
                x += width;
                column
            })
            .collect();
    }

    pub fn rows(self, heights: &[i32]) -> Vec<Self> {
        let mut y = self.y;
        return heights
            .iter()
            .map(|&height| {
                let row = Self::new(self.x, y, self.w, height);
                y += height;
                row
            })
            .collect();
    }

    /// Leftover pixels widen the first columns.
    pub fn split_columns(self, count: i32) -> Vec<Self> {
        return self.columns(&even_sizes(self.w, count));
    }

    /// Leftover lines go to the first rows.
    pub fn split_rows(self, count: i32) -> Vec<Self> {
        return self.rows(&even_sizes(self.h, count));
    }

    /// Swaps width and height.
    pub fn rotate(self) -> Self {
        return Self::new(self.x, self.y, self.h, self.w);
    }

    pub fn swap_xy(self) -> Self {
        return Self::new(self.y, self.x, self.w, self.h);
    }

    /// Treats this rect as relative to `area`'s top left.
    pub fn within(self, area: Self) -> Self {
        return Self::new(area.x + self.x, area.y + self.y, self.w, self.h);
    }

    pub fn inset(self, x: i32, y: i32) -> Self {
        return Self::new(self.x + x, self.y + y, self.w - 2 * x, self.h - 2 * y);
    }

    pub fn intersection(self, other: Self) -> Self {
        let (x, y) = (self.x.max(other.x), self.y.max(other.y));
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        return Self::new(x, y, (right - x).max(0), (bottom - y).max(0));
    }
}

impl From<Rect> for ggez::graphics::Rect {
    fn from(rect: Rect) -> Self {
        return Self::new(rect.x as f32, rect.y as f32, rect.w as f32, rect.h as f32);
    }
}

fn even_sizes(total: i32, count: i32) -> Vec<i32> {
    if count <= 0 {
        return Vec::new();
    }
    let (size, leftover) = (total / count, total % count);
    return (0..count)
        .map(|index| size + i32::from(index < leftover))
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rects_do_not_contain_their_far_edges() {
        let rect = Rect::new(10, 10, 20, 10);
        assert!(rect.contains([10, 10]));
        assert!(rect.contains([29, 19]));
        assert!(!rect.contains([30, 15]));
        assert!(!rect.contains([15, 20]));
    }

    #[test]
    fn splitting_shares_the_area_between_the_parts() {
        let rect = Rect::new(0, 10, 100, 50);
        assert_eq!(
            rect.split_top(20),
            (Rect::new(0, 10, 100, 20), Rect::new(0, 30, 100, 30))
        );
        assert_eq!(
            rect.split_left(70),
            (Rect::new(0, 10, 70, 50), Rect::new(70, 10, 30, 50))
        );
        assert_eq!(rect.split_top(80).1.h, 0);
    }

    #[test]
    fn splitting_evenly_fills_the_rect() {
        let rect = Rect::new(10, 20, 22, 7);
        let widths: Vec<i32> = rect
            .split_columns(5)
            .iter()
            .map(|column| column.w)
            .collect();
        assert_eq!(widths, [5, 5, 4, 4, 4]);
        assert_eq!(rect.split_columns(5)[4], Rect::new(28, 20, 4, 7));
        assert_eq!(
            rect.split_rows(3),
            [
                Rect::new(10, 20, 22, 3),
                Rect::new(10, 23, 22, 2),
                Rect::new(10, 25, 22, 2),
            ]
        );
        assert!(rect.split_rows(0).is_empty());
    }

    #[test]
    fn rotating_swaps_the_width_and_height() {
        let rect = Rect::new(10, 20, 160, 8);
        assert_eq!(rect.rotate(), Rect::new(10, 20, 8, 160));
        assert_eq!(rect.rotate().rotate(), rect);
    }

    #[test]
    fn swapping_x_and_y_keeps_the_size() {
        let rect = Rect::new(10, 20, 160, 8);
        assert_eq!(rect.swap_xy(), Rect::new(20, 10, 160, 8));
        assert_eq!(rect.swap_xy().rotate(), Rect::new(20, 10, 8, 160));
    }

    #[test]
    fn a_rect_within_an_area_moves_by_its_top_left() {
        let area = Rect::new(10, 18, 448, 45);
        assert_eq!(
            Rect::new(182, 0, 146, 45).within(area),
            Rect::new(192, 18, 146, 45)
        );
    }

    #[test]
    fn an_intersection_is_the_overlap_or_nothing() {
        let screen = Rect::new(0, 0, 640, 256);
        assert_eq!(
            Rect::new(-10, 250, 30, 20).intersection(screen),
            Rect::new(0, 250, 20, 6)
        );
        let off = Rect::new(700, 0, 10, 10).intersection(screen);
        assert_eq!((off.w, off.h), (0, 10));
    }

    #[test]
    fn columns_follow_each_other() {
        let columns = Rect::new(3, 65, 100, 11).columns(&[84, 14]);
        assert_eq!(
            columns,
            vec![Rect::new(3, 65, 84, 11), Rect::new(87, 65, 14, 11)]
        );
    }

    #[test]
    fn rows_follow_each_other() {
        let rows = Rect::new(3, 20, 32, 41).rows(&[1, 8, 8]);
        assert_eq!(
            rows,
            vec![
                Rect::new(3, 20, 32, 1),
                Rect::new(3, 21, 32, 8),
                Rect::new(3, 29, 32, 8),
            ]
        );
    }
}
