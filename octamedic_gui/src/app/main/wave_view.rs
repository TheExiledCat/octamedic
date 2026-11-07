use std::ops::Range;

/// Which part of a sample the sample editor shows. Each zoom level halves what's in view, zooming
/// in and out around the middle of it. Zoom 0 is the whole sample, or the range SHOW RANGE showed,
/// and zooming out from a range doubles it until it's the whole sample again.
///
/// Like OctaMED, zooming in moves the start on by a quarter of the view, rounded down, and zooming
/// out moves it back by as much, so a zoom out undoes a zoom in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WaveView {
    zoom: u32,
    start: usize,
    // how much zoom 0 shows
    base: usize,
    // what the view was worked out for, so another sample starts it over
    length: usize,
}

impl WaveView {
    pub fn visible(&self) -> Range<usize> {
        return self.start..self.start + self.visible_length(self.zoom);
    }

    /// Starts over at the whole sample if it isn't `length` long any more.
    pub fn follow(&mut self, length: usize) {
        if length != self.length {
            *self = Self {
                length,
                base: length,
                ..Self::default()
            };
        }
    }

    /// Keeps the view where it is for a sample that's been cut down or added to, now `length`
    /// long, so what came after the change moves along under it. It only moves when the sample
    /// doesn't reach that far any more.
    pub fn resized(&mut self, length: usize) {
        let visible = self.visible_length(self.zoom);
        if visible > length {
            *self = Self {
                length,
                base: length,
                ..Self::default()
            };
            return;
        }
        self.length = length;
        // zooming out would show more than there is, so zooming out starts from this view
        if self.base > length {
            self.base = visible;
            self.zoom = 0;
        }
        self.start = self.start.min(length - visible);
    }

    pub fn zoom_in(&mut self) {
        let visible = self.visible_length(self.zoom);
        if self.visible_length(self.zoom + 1) > 0 {
            self.zoom += 1;
            self.start += visible / 4;
        }
    }

    pub fn zoom_out(&mut self) {
        if self.zoom > 0 {
            self.zoom -= 1;
        } else if self.base < self.length {
            self.base = (2 * self.base).min(self.length);
        } else {
            return;
        }
        let visible = self.visible_length(self.zoom);
        self.start = self
            .start
            .saturating_sub(visible / 4)
            .min(self.length - visible);
    }

    /// Moves the view half its length towards the start, or the end when `next` is true, as far as
    /// the sample goes.
    pub fn scroll(&mut self, next: bool) {
        let visible = self.visible_length(self.zoom);
        let step = (visible / 2).max(1);
        let start = if next {
            self.start + step
        } else {
            self.start.saturating_sub(step)
        };
        self.start = start.min(self.length - visible);
    }

    pub fn sample_at(&self, column: i32, width: i32) -> usize {
        let visible = self.visible_length(self.zoom);
        let column = column.clamp(0, (width - 1).max(0)) as usize;
        return self.start + column * visible / width.max(1) as usize;
    }

    /// The first column of a waveform `width` wide that sample `index` is drawn in, from 0 for
    /// anything before the view to `width` for anything after it.
    pub fn column_of(&self, index: usize, width: i32) -> i32 {
        let visible = self.visible_length(self.zoom);
        if visible == 0 || index <= self.start {
            return 0;
        }
        let into_view = (index - self.start).min(visible);
        return (into_view * width as usize).div_ceil(visible) as i32;
    }

    pub fn show_all(&mut self) {
        self.show(0..self.length);
    }

    /// Shows just `range`, as far as the sample goes, if there's anything in it.
    pub fn show(&mut self, range: Range<usize>) {
        let range = range.start.min(self.length)..range.end.min(self.length);
        if !range.is_empty() {
            self.zoom = 0;
            self.start = range.start;
            self.base = range.len();
        }
    }

    /// Shows `samples` around the middle of the view, as far as the sample goes.
    pub fn show_around_middle(&mut self, samples: usize) {
        let visible = self.visible();
        let middle = (visible.start + visible.end) / 2;
        let start = middle
            .saturating_sub(samples / 2)
            .min(self.length.saturating_sub(samples));
        self.show(start..start + samples);
    }

    fn visible_length(&self, zoom: u32) -> usize {
        return self.base.checked_shr(zoom).unwrap_or(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(length: usize) -> WaveView {
        let mut view = WaveView::default();
        view.follow(length);
        return view;
    }

    #[test]
    fn zoom_halves_around_the_middle() {
        let mut view = view(1000);
        assert_eq!(view.visible(), 0..1000);
        view.zoom_in();
        assert_eq!(view.visible(), 250..750);
        view.zoom_in();
        assert_eq!(view.visible(), 375..625);
        view.zoom_out();
        assert_eq!(view.visible(), 250..750);
        view.zoom_out();
        view.zoom_out();
        assert_eq!(view.visible(), 0..1000);
    }

    #[test]
    fn lands_where_octamed_does() {
        // measured from OctaMED, 9 zooms into a 36474 sample
        let mut view = view(36474);
        for _ in 0..9 {
            view.zoom_in();
        }
        assert_eq!(view.visible(), 18196..18267);
        for _ in 0..9 {
            view.zoom_out();
        }
        assert_eq!(view.visible(), 0..36474);
    }

    #[test]
    fn scrolls_half_a_view_at_a_time() {
        let mut view = view(1000);
        view.zoom_in();
        view.zoom_in();
        assert_eq!(view.visible(), 375..625);
        view.scroll(false);
        assert_eq!(view.visible(), 250..500);
        view.scroll(false);
        view.scroll(false);
        assert_eq!(view.visible(), 0..250);
        for _ in 0..10 {
            view.scroll(true);
        }
        assert_eq!(view.visible(), 750..1000);
        view.zoom_in();
        assert_eq!(view.visible(), 812..937);
        view.zoom_out();
        view.zoom_out();
        assert_eq!(view.visible(), 500..1000);
        view.show_all();
        view.scroll(true);
        assert_eq!(view.visible(), 0..1000);
    }

    #[test]
    fn columns_to_samples() {
        let mut view = view(1000);
        assert_eq!((view.sample_at(0, 640), view.sample_at(639, 640)), (0, 998));
        view.zoom_in();
        view.zoom_in();
        view.zoom_in();
        view.zoom_in();
        // 62 samples, about 10 columns each
        assert_eq!(view.visible(), 468..530);
        assert_eq!(
            (view.sample_at(0, 640), view.sample_at(15, 640)),
            (468, 469)
        );
        assert_eq!(view.sample_at(639, 640), 529);
    }

    #[test]
    fn samples_to_columns() {
        let mut view = view(1000);
        view.zoom_in();
        view.zoom_in();
        view.zoom_in();
        view.zoom_in();
        assert_eq!(view.visible(), 468..530);
        assert_eq!((view.column_of(100, 640), view.column_of(468, 640)), (0, 0));
        assert_eq!(
            (view.column_of(469, 640), view.column_of(530, 640)),
            (11, 640)
        );
        assert_eq!(view.column_of(900, 640), 640);
        for column in 0..640 {
            let sample = view.sample_at(column, 640);
            assert!(view.column_of(sample, 640) <= column);
            assert!(view.column_of(sample + 1, 640) > column);
        }
    }

    #[test]
    fn showing_a_range() {
        let mut view = view(1000);
        view.show(100..200);
        assert_eq!(view.visible(), 100..200);
        view.zoom_in();
        assert_eq!(view.visible(), 125..175);
        view.zoom_out();
        assert_eq!(view.visible(), 100..200);
        view.zoom_out();
        assert_eq!(view.visible(), 50..250);
        view.zoom_out();
        view.zoom_out();
        view.zoom_out();
        assert_eq!(view.visible(), 0..1000);
        view.show(300..300);
        assert_eq!(view.visible(), 0..1000);
        view.show(900..1200);
        assert_eq!(view.visible(), 900..1000);
    }

    #[test]
    fn a_cut_keeps_the_view() {
        let mut view = view(1000);
        view.zoom_in();
        view.zoom_in();
        assert_eq!(view.visible(), 375..625);
        view.resized(900);
        view.follow(900);
        assert_eq!(view.visible(), 375..625);
        view.resized(500);
        assert_eq!(view.visible(), 250..500);
        view.resized(100);
        assert_eq!(view.visible(), 0..100);
    }

    #[test]
    fn show_all_starts_over() {
        let mut view = view(1000);
        view.zoom_in();
        view.zoom_in();
        view.show_all();
        assert_eq!(view.visible(), 0..1000);
    }

    #[test]
    fn stops_at_one_sample() {
        let mut view = view(4);
        for _ in 0..10 {
            view.zoom_in();
        }
        assert_eq!(view.visible(), 1..2);
    }

    #[test]
    fn another_sample_starts_over() {
        let mut view = view(1000);
        view.zoom_in();
        view.follow(1000);
        assert_eq!(view.visible(), 250..750);
        view.follow(64);
        assert_eq!(view.visible(), 0..64);
        let mut empty = WaveView::default();
        empty.follow(0);
        empty.zoom_in();
        empty.scroll(true);
        assert_eq!(empty.visible(), 0..0);
    }

    #[test]
    fn showing_around_the_middle() {
        let mut view = view(1000);
        view.show_around_middle(640);
        assert_eq!(view.visible(), 180..820);
        view.show_around_middle(100);
        assert_eq!(view.visible(), 450..550);
        for _ in 0..10 {
            view.scroll(true);
        }
        view.show_around_middle(640);
        assert_eq!(view.visible(), 360..1000);
        let mut short = self::view(300);
        short.show_around_middle(640);
        assert_eq!(short.visible(), 0..300);
    }
}
