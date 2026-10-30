/// A position in screen pixels.
pub type Point = [i32; 2];

/// An area in screen pixels.
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

    pub fn right(self) -> i32 {
        return self.x + self.w;
    }

    pub fn bottom(self) -> i32 {
        return self.y + self.h;
    }

    /// Without the right and bottom edges, so neighbours don't overlap.
    pub fn contains(self, point: Point) -> bool {
        let [x, y] = point;
        return x >= self.x && x < self.right() && y >= self.y && y < self.bottom();
    }

    /// The top `height` lines, and the rest below them.
    pub fn split_top(self, height: i32) -> (Self, Self) {
        let height = height.clamp(0, self.h);
        let top = Self::new(self.x, self.y, self.w, height);
        let rest = Self::new(self.x, self.y + height, self.w, self.h - height);
        return (top, rest);
    }

    /// The left `width` pixels, and the rest to their right.
    pub fn split_left(self, width: i32) -> (Self, Self) {
        let width = width.clamp(0, self.w);
        let left = Self::new(self.x, self.y, width, self.h);
        let rest = Self::new(self.x + width, self.y, self.w - width, self.h);
        return (left, rest);
    }

    /// Side by side from the left, one per width, as tall as this.
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

    /// Turned a quarter around its top left: as wide as it was tall, and as tall as it was wide.
    pub fn rotate(self) -> Self {
        return Self::new(self.x, self.y, self.h, self.w);
    }

    /// With its x and y swapped; the size stays. With `rotate`, it mirrors the rect across the
    /// diagonal.
    pub fn swap_xy(self) -> Self {
        return Self::new(self.y, self.x, self.w, self.h);
    }

    /// This rect, given from `area`'s top left, placed in `area`.
    pub fn within(self, area: Self) -> Self {
        return Self::new(area.x + self.x, area.y + self.y, self.w, self.h);
    }

    /// Smaller by `x` on the left and right and `y` at the top and bottom.
    pub fn inset(self, x: i32, y: i32) -> Self {
        return Self::new(self.x + x, self.y + y, self.w - 2 * x, self.h - 2 * y);
    }
}

impl From<Rect> for ggez::graphics::Rect {
    fn from(rect: Rect) -> Self {
        return Self::new(rect.x as f32, rect.y as f32, rect.w as f32, rect.h as f32);
    }
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
    fn columns_follow_each_other() {
        let columns = Rect::new(3, 65, 100, 11).columns(&[84, 14]);
        assert_eq!(
            columns,
            vec![Rect::new(3, 65, 84, 11), Rect::new(87, 65, 14, 11)]
        );
    }
}
