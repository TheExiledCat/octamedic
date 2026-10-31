use crate::{
    app::state::AppState,
    framework::{painter::Painter, rect::Rect, target::Color, widget::OctamedicWidget},
};

// one colour per two lines, top down. measured off OctaMED from the 4th on, the first three just
// continue its steps up to red
const VU_COLORS: [u32; 30] = [
    0xFF2211, 0xFF3311, 0xFF4411, 0xFF5511, 0xFF6622, 0xFF7722, 0xFF8822, 0xFF9922, 0xFFAA22,
    0xFFBB22, 0xFFCC22, 0xFFDD33, 0xFFEE33, 0xFFFF33, 0xEEFF33, 0xCCFF22, 0xBBFF22, 0x99FF22,
    0x88FF11, 0x66FF11, 0x44FF11, 0x22FF00, 0x00FF00, 0x00EE00, 0x00DD00, 0x00CC00, 0x00BB00,
    0x00AA00, 0x009900, 0x008800,
];

const VU_EDGES: [u32; 2] = [0xAAAAAA, 0xBBBBBB];

// edges are a tenth of the width
const VU_EDGE_SHARE: i32 = 10;

const MAX_LEVEL: u8 = 128;

// per frame, a full meter empties in about 2/3 s at 50Hz
const VU_FALL: u8 = 4;

#[derive(Clone)]
pub struct Oscilloscope {
    sample_count: usize,
    // oldest first, always sample_count long
    samples: Vec<i8>,
}

impl Oscilloscope {
    pub fn new(sample_count: usize) -> Self {
        return Self {
            sample_count,
            samples: vec![0; sample_count],
        };
    }

    // evenly spaced picks so a long window still fits
    pub fn show(&mut self, window: &[i8]) {
        if window.is_empty() {
            return;
        }
        let count = self.sample_count;
        self.samples = (0..count)
            .map(|i| window[i * window.len() / count])
            .collect();
    }

    // one rect per run of samples on the same row, saves fills
    pub fn runs(&self, area: Rect) -> impl Iterator<Item = Rect> + '_ {
        let count = self.samples.len();
        // spreads the remainder so the runs fill the width exactly
        let x = move |i: usize| area.x + (i * area.w as usize / count) as i32;
        let y = move |i: usize| sample_y(self.samples[i], area);
        let mut start = 0;
        return std::iter::from_fn(move || {
            if start >= count {
                return None;
            }
            let row = y(start);
            let mut end = start + 1;
            while end < count && y(end) == row {
                end += 1;
            }
            let run = Rect::new(x(start), row, x(end) - x(start), 1);
            start = end;
            return Some(run);
        });
    }
}

#[derive(Default)]
pub struct VuMeter {
    level: u8,
}

impl VuMeter {
    // 0..=128, jumps up straight away and falls back slowly
    pub fn set(&mut self, level: u8) {
        self.level = level.min(MAX_LEVEL).max(self.level.saturating_sub(VU_FALL));
    }

    // a silent meter still shows its bottom line
    fn filled(&self, area: Rect) -> i32 {
        return (self.level as i32 * area.h / MAX_LEVEL as i32).max(1);
    }

    // lines of the same colour merged into one rect
    fn bands(&self, area: Rect) -> impl Iterator<Item = (Rect, Color)> {
        let bottom = area.y + area.h;
        let filled = self.filled(area);
        let band = move |y: i32| (((y - area.y + 1) / 2) as usize).min(VU_COLORS.len() - 1);
        let edge = area.w / VU_EDGE_SHARE;
        let mut y = bottom - filled;
        return std::iter::from_fn(move || {
            if y >= bottom {
                return None;
            }
            let color = band(y);
            let mut end = y + 1;
            while end < bottom && band(end) == color {
                end += 1;
            }
            let rect = Rect::new(area.x + edge, y, area.w - 2 * edge, end - y);
            y = end;
            return Some((rect, rgb(VU_COLORS[color])));
        });
    }
}

impl OctamedicWidget for VuMeter {
    fn render(&mut self, _state: &AppState, area: Rect, painter: &mut Painter) {
        if area.h <= 0 {
            return;
        }
        let filled = self.filled(area);
        let top = area.y + area.h - filled;
        let edge = area.w / VU_EDGE_SHARE;
        let [left, right] = VU_EDGES.map(rgb);
        painter.fill_color(Rect::new(area.x, top, edge, filled), left);
        painter.fill_color(Rect::new(area.x + area.w - edge, top, edge, filled), right);
        for (rect, color) in self.bands(area) {
            painter.fill_color(rect, color);
        }
    }
}

fn sample_y(sample: i8, area: Rect) -> i32 {
    let half = area.h / 2;
    let y = area.y + half - sample as i32 * half / 128;
    return y.min(area.y + area.h - 1);
}

fn rgb(color: u32) -> Color {
    let [_, r, g, b] = color.to_be_bytes();
    return Color::rgb(r, g, b);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence() {
        let scope = Oscilloscope::new(16);
        let runs: Vec<Rect> = scope.runs(Rect::new(10, 0, 64, 20)).collect();
        assert_eq!(runs, vec![Rect::new(10, 10, 64, 1)]);
    }

    #[test]
    fn runs_merge() {
        let mut scope = Oscilloscope::new(5);
        scope.show(&[0, 0, 64, 64, 0]);
        let runs: Vec<Rect> = scope.runs(Rect::new(0, 0, 10, 20)).collect();
        assert_eq!(
            runs,
            vec![
                Rect::new(0, 10, 4, 1),
                Rect::new(4, 5, 4, 1),
                Rect::new(8, 10, 2, 1),
            ]
        );
    }

    #[test]
    fn show_window() {
        let mut scope = Oscilloscope::new(4);
        scope.show(&[0, 1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(scope.samples, [0, 2, 4, 6]);
    }

    #[test]
    fn full_meter_bands() {
        let mut meter = VuMeter::default();
        meter.set(MAX_LEVEL);
        let bands: Vec<(Rect, Color)> = meter.bands(Rect::new(0, 60, 20, 60)).collect();
        assert_eq!(bands.len(), VU_COLORS.len());
        assert_eq!(bands[0], (Rect::new(2, 60, 16, 1), rgb(0xFF2211)));
        assert_eq!(bands[3], (Rect::new(2, 65, 16, 2), rgb(0xFF5511)));
        assert_eq!(bands[29], (Rect::new(2, 117, 16, 3), rgb(0x008800)));
    }

    #[test]
    fn meter_falloff() {
        let mut meter = VuMeter::default();
        meter.set(100);
        assert_eq!(meter.level, 100);
        meter.set(0);
        assert_eq!(meter.level, 100 - VU_FALL);
        meter.set(120);
        assert_eq!(meter.level, 120);
    }

    #[test]
    fn meter_fill() {
        let mut meter = VuMeter::default();
        meter.set(64);
        let bands: Vec<(Rect, Color)> = meter.bands(Rect::new(0, 0, 20, 54)).collect();
        assert_eq!(bands[0].0.y, 27);
        let silent: Vec<(Rect, Color)> =
            VuMeter::default().bands(Rect::new(0, 0, 20, 54)).collect();
        assert_eq!(silent.len(), 1);
        assert_eq!(silent[0].0, Rect::new(2, 53, 16, 1));
    }

    #[test]
    fn sample_extremes() {
        let area = Rect::new(0, 4, 8, 9);
        assert_eq!(sample_y(i8::MIN, area), 12);
        assert_eq!(sample_y(i8::MAX, area), 5);
    }
}
