//! Filters run over a finished [`Framebuffer`], after everything's been painted.

use crate::target::Framebuffer;

/// Changes a finished frame, like scanlines over it. Closures taking `&mut Framebuffer` are
/// filters too, and a list of filters runs them in order.
pub trait Filter {
    fn apply(&self, frame: &mut Framebuffer);

    /// Moves on a frame, for filters that animate. Call it once a frame.
    fn advance_frame(&mut self) {}
}

/// One of a filter's settings, a number from `min` to `max` in steps of `step`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Param {
    pub name: &'static str,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub default: f32,
}

impl Param {
    pub fn clamp(&self, value: f32) -> f32 {
        return value.clamp(self.min, self.max);
    }
}

/// A filter's settings, by index, so they can be edited without knowing which filter it is.
pub trait FilterParams {
    fn params(&self) -> &'static [Param];

    fn param(&self, index: usize) -> f32;

    /// Clamped to what the param takes. Indexes past the end are ignored.
    fn set_param(&mut self, index: usize, value: f32);
}

/// A filter whose settings can be edited, for keeping a list of different filters.
pub trait AdjustableFilter: Filter + FilterParams {}

impl<T: Filter + FilterParams> AdjustableFilter for T {}

impl<F: Fn(&mut Framebuffer)> Filter for F {
    fn apply(&self, frame: &mut Framebuffer) {
        self(frame);
    }
}

impl Filter for [&dyn Filter] {
    fn apply(&self, frame: &mut Framebuffer) {
        for filter in self {
            filter.apply(frame);
        }
    }
}

const SCANLINES_PARAMS: &[Param] = &[
    Param {
        name: "Strength",
        min: 0.0,
        max: 1.0,
        step: 0.05,
        default: 0.3,
    },
    Param {
        name: "Spacing",
        min: 1.0,
        max: 8.0,
        step: 1.0,
        default: 2.0,
    },
];

const CHROMATIC_ABERRATION_PARAMS: &[Param] = &[Param {
    name: "Offset",
    min: 0.0,
    max: 8.0,
    step: 1.0,
    default: 1.0,
}];

const VHS_WOBBLE_PARAMS: &[Param] = &[
    Param {
        name: "Amount",
        min: 0.0,
        max: 8.0,
        step: 0.5,
        default: 1.5,
    },
    Param {
        name: "Wavelength",
        min: 4.0,
        max: 256.0,
        step: 4.0,
        default: 64.0,
    },
    Param {
        name: "Speed",
        min: 0.0,
        max: 4.0,
        step: 0.25,
        default: 0.5,
    },
    Param {
        name: "Jitter",
        min: 0.0,
        max: 4.0,
        step: 0.25,
        default: 0.5,
    },
];

/// Darkens every `spacing`th line, starting with the second, like the gaps between a CRT's lines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scanlines {
    /// How much darker, 0.0 for not at all, 1.0 for black.
    pub strength: f32,
    pub spacing: i32,
}

impl Default for Scanlines {
    fn default() -> Self {
        return Self {
            strength: SCANLINES_PARAMS[0].default,
            spacing: SCANLINES_PARAMS[1].default as i32,
        };
    }
}

impl FilterParams for Scanlines {
    fn params(&self) -> &'static [Param] {
        return SCANLINES_PARAMS;
    }

    fn param(&self, index: usize) -> f32 {
        return match index {
            0 => self.strength,
            1 => self.spacing as f32,
            _ => 0.0,
        };
    }

    fn set_param(&mut self, index: usize, value: f32) {
        let Some(param) = SCANLINES_PARAMS.get(index) else {
            return;
        };
        let value = param.clamp(value);
        match index {
            0 => self.strength = value,
            _ => self.spacing = value.round() as i32,
        }
    }
}

impl Filter for Scanlines {
    fn apply(&self, frame: &mut Framebuffer) {
        let spacing = self.spacing.max(1);
        let keep = 1.0 - self.strength.clamp(0.0, 1.0);
        for y in (spacing - 1..frame.height()).step_by(spacing as usize) {
            let Some(row) = frame.row_mut(y) else {
                continue;
            };
            for pixel in row {
                *pixel = pixel.map(|channel| (channel as f32 * keep) as u8);
            }
        }
    }
}

/// Moves the red channel `offset` pixels left and the blue one right, like a lens that doesn't
/// focus every colour in the same place. Edges repeat the last pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChromaticAberration {
    pub offset: i32,
}

impl Default for ChromaticAberration {
    fn default() -> Self {
        return Self {
            offset: CHROMATIC_ABERRATION_PARAMS[0].default as i32,
        };
    }
}

impl FilterParams for ChromaticAberration {
    fn params(&self) -> &'static [Param] {
        return CHROMATIC_ABERRATION_PARAMS;
    }

    fn param(&self, index: usize) -> f32 {
        return if index == 0 { self.offset as f32 } else { 0.0 };
    }

    fn set_param(&mut self, index: usize, value: f32) {
        if let Some(param) = CHROMATIC_ABERRATION_PARAMS.get(index) {
            self.offset = param.clamp(value).round() as i32;
        }
    }
}

impl Filter for ChromaticAberration {
    fn apply(&self, frame: &mut Framebuffer) {
        for y in 0..frame.height() {
            let Some(row) = frame.row_mut(y) else {
                continue;
            };
            let original = row.to_vec();
            let last = original.len() as i32 - 1;
            let at = |x: i32| original[x.clamp(0, last) as usize];
            for (x, pixel) in row.iter_mut().enumerate() {
                let x = x as i32;
                pixel[0] = at(x + self.offset)[0];
                pixel[2] = at(x - self.offset)[2];
            }
        }
    }
}

/// Shifts each line sideways by a wave that rolls down the screen, plus some jitter, like a
/// worn tape. [`Filter::advance_frame`] moves `frame` on, which makes it roll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VhsWobble {
    /// The furthest a line moves, in pixels.
    pub amount: f32,
    /// Lines per wave.
    pub wavelength: f32,
    /// Lines the wave moves down each frame.
    pub speed: f32,
    /// Extra random movement per line, in pixels.
    pub jitter: f32,
    pub frame: u32,
}

impl Default for VhsWobble {
    fn default() -> Self {
        return Self {
            amount: VHS_WOBBLE_PARAMS[0].default,
            wavelength: VHS_WOBBLE_PARAMS[1].default,
            speed: VHS_WOBBLE_PARAMS[2].default,
            jitter: VHS_WOBBLE_PARAMS[3].default,
            frame: 0,
        };
    }
}

impl FilterParams for VhsWobble {
    fn params(&self) -> &'static [Param] {
        return VHS_WOBBLE_PARAMS;
    }

    fn param(&self, index: usize) -> f32 {
        return match index {
            0 => self.amount,
            1 => self.wavelength,
            2 => self.speed,
            3 => self.jitter,
            _ => 0.0,
        };
    }

    fn set_param(&mut self, index: usize, value: f32) {
        let Some(param) = VHS_WOBBLE_PARAMS.get(index) else {
            return;
        };
        let value = param.clamp(value);
        match index {
            0 => self.amount = value,
            1 => self.wavelength = value,
            2 => self.speed = value,
            _ => self.jitter = value,
        }
    }
}

impl VhsWobble {
    fn shift(&self, y: i32) -> i32 {
        let phase = (y as f32 + self.frame as f32 * self.speed) / self.wavelength;
        let wave = (phase * std::f32::consts::TAU).sin() * self.amount;
        let noise = noise(y as u32, self.frame) * self.jitter;
        return (wave + noise).round() as i32;
    }
}

impl Filter for VhsWobble {
    fn advance_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    fn apply(&self, frame: &mut Framebuffer) {
        for y in 0..frame.height() {
            let shift = self.shift(y);
            let Some(row) = frame.row_mut(y) else {
                continue;
            };
            let last = row.len() as i32 - 1;
            if shift == 0 || last < 0 {
                continue;
            }
            let original = row.to_vec();
            for (x, pixel) in row.iter_mut().enumerate() {
                *pixel = original[(x as i32 - shift).clamp(0, last) as usize];
            }
        }
    }
}

const COLOR_ADJUST_PARAMS: &[Param] = &[
    Param {
        name: "Hue",
        min: -180.0,
        max: 180.0,
        step: 5.0,
        default: 0.0,
    },
    Param {
        name: "Saturation",
        min: 0.0,
        max: 2.0,
        step: 0.05,
        default: 1.0,
    },
    Param {
        name: "Brightness",
        min: 0.0,
        max: 2.0,
        step: 0.05,
        default: 1.0,
    },
    Param {
        name: "Contrast",
        min: 0.0,
        max: 2.0,
        step: 0.05,
        default: 1.0,
    },
    Param {
        name: "Warmth",
        min: -1.0,
        max: 1.0,
        step: 0.05,
        default: 0.0,
    },
];

// how far full warmth moves red up and blue down, out of 1.0
const MAX_WARMTH_SHIFT: f32 = 0.15;

/// Shifts the hue round and sets the saturation, brightness, contrast and warmth, like the
/// knobs on an old monitor. The hue turns in YIQ, the colour space NTSC TVs used, and warmth
/// moves red up and blue down, or the other way when it's below 0. The defaults change nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorAdjust {
    /// Degrees round the colour wheel.
    pub hue: f32,
    /// 0.0 for grey, 1.0 as it was.
    pub saturation: f32,
    pub brightness: f32,
    pub contrast: f32,
    pub warmth: f32,
}

impl Default for ColorAdjust {
    fn default() -> Self {
        return Self {
            hue: COLOR_ADJUST_PARAMS[0].default,
            saturation: COLOR_ADJUST_PARAMS[1].default,
            brightness: COLOR_ADJUST_PARAMS[2].default,
            contrast: COLOR_ADJUST_PARAMS[3].default,
            warmth: COLOR_ADJUST_PARAMS[4].default,
        };
    }
}

impl ColorAdjust {
    fn adjust(&self, [r, g, b]: [f32; 3], (sin, cos): (f32, f32)) -> [f32; 3] {
        let y = 0.299 * r + 0.587 * g + 0.114 * b;
        let i = 0.596 * r - 0.274 * g - 0.322 * b;
        let q = 0.211 * r - 0.523 * g + 0.312 * b;
        let (i, q) = (
            (i * cos - q * sin) * self.saturation,
            (i * sin + q * cos) * self.saturation,
        );
        let rgb = [
            y + 0.956 * i + 0.621 * q,
            y - 0.272 * i - 0.647 * q,
            y - 1.106 * i + 1.703 * q,
        ];
        let warmth = self.warmth * MAX_WARMTH_SHIFT;
        let shifts = [warmth, 0.0, -warmth];
        return std::array::from_fn(|channel| {
            let contrasted = (rgb[channel] - 0.5) * self.contrast + 0.5;
            contrasted * self.brightness + shifts[channel]
        });
    }
}

impl Filter for ColorAdjust {
    fn apply(&self, frame: &mut Framebuffer) {
        let turn = self.hue.to_radians().sin_cos();
        for pixel in frame.pixels_mut() {
            let rgb = pixel.map(|channel| channel as f32 / u8::MAX as f32);
            let adjusted = self.adjust(rgb, turn);
            *pixel =
                adjusted.map(|channel| (channel.clamp(0.0, 1.0) * u8::MAX as f32).round() as u8);
        }
    }
}

impl FilterParams for ColorAdjust {
    fn params(&self) -> &'static [Param] {
        return COLOR_ADJUST_PARAMS;
    }

    fn param(&self, index: usize) -> f32 {
        return match index {
            0 => self.hue,
            1 => self.saturation,
            2 => self.brightness,
            3 => self.contrast,
            4 => self.warmth,
            _ => 0.0,
        };
    }

    fn set_param(&mut self, index: usize, value: f32) {
        let Some(param) = COLOR_ADJUST_PARAMS.get(index) else {
            return;
        };
        let value = param.clamp(value);
        match index {
            0 => self.hue = value,
            1 => self.saturation = value,
            2 => self.brightness = value,
            3 => self.contrast = value,
            _ => self.warmth = value,
        }
    }
}

const CURVATURE_PARAMS: &[Param] = &[Param {
    name: "Amount",
    min: 0.0,
    max: 0.25,
    step: 0.01,
    default: 0.04,
}];

/// Bulges the picture out like the curved glass of a CRT, the corners pulled in furthest. What
/// falls outside the tube is black.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Curvature {
    /// How far the corners are pulled in, 0.0 for a flat screen.
    pub amount: f32,
}

impl Default for Curvature {
    fn default() -> Self {
        return Self {
            amount: CURVATURE_PARAMS[0].default,
        };
    }
}

impl FilterParams for Curvature {
    fn params(&self) -> &'static [Param] {
        return CURVATURE_PARAMS;
    }

    fn param(&self, index: usize) -> f32 {
        return if index == 0 { self.amount } else { 0.0 };
    }

    fn set_param(&mut self, index: usize, value: f32) {
        if let Some(param) = CURVATURE_PARAMS.get(index) {
            self.amount = param.clamp(value);
        }
    }
}

impl Filter for Curvature {
    fn apply(&self, frame: &mut Framebuffer) {
        if self.amount <= 0.0 {
            return;
        }
        let (width, height) = (frame.width(), frame.height());
        let original = frame.clone();
        for y in 0..height {
            let Some(row) = frame.row_mut(y) else {
                continue;
            };
            let dy = centred(y, height);
            for (x, pixel) in row.iter_mut().enumerate() {
                let dx = centred(x as i32, width);
                // each way bends more the further out it is the other way, so the edges bow
                let from_x = uncentred(dx * (1.0 + self.amount * dy * dy), width);
                let from_y = uncentred(dy * (1.0 + self.amount * dx * dx), height);
                *pixel = original.pixel(from_x, from_y).unwrap_or([0; 3]);
            }
        }
    }
}

const VIGNETTE_PARAMS: &[Param] = &[
    Param {
        name: "Strength",
        min: 0.0,
        max: 1.0,
        step: 0.05,
        default: 0.6,
    },
    Param {
        name: "Size",
        min: 0.0,
        max: 1.0,
        step: 0.05,
        default: 0.3,
    },
];

/// Darkens towards the edges, like an old tube.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vignette {
    /// How dark the edges get, 1.0 for black.
    pub strength: f32,
    /// How much of the middle stays as it is, 0.0 for none, 1.0 for right out to the edges.
    pub size: f32,
}

impl Default for Vignette {
    fn default() -> Self {
        return Self {
            strength: VIGNETTE_PARAMS[0].default,
            size: VIGNETTE_PARAMS[1].default,
        };
    }
}

impl FilterParams for Vignette {
    fn params(&self) -> &'static [Param] {
        return VIGNETTE_PARAMS;
    }

    fn param(&self, index: usize) -> f32 {
        return match index {
            0 => self.strength,
            1 => self.size,
            _ => 0.0,
        };
    }

    fn set_param(&mut self, index: usize, value: f32) {
        let Some(param) = VIGNETTE_PARAMS.get(index) else {
            return;
        };
        let value = param.clamp(value);
        match index {
            0 => self.strength = value,
            _ => self.size = value,
        }
    }
}

impl Vignette {
    // what's left of a pixel `distance` from the middle, where 1.0 is the middle of an edge.
    // smoothstep, so the dark comes in gently and levels off at the edges
    fn keep(&self, distance: f32) -> f32 {
        let fade = ((distance - self.size) / (1.0 - self.size).max(f32::EPSILON)).clamp(0.0, 1.0);
        return 1.0 - self.strength * fade * fade * (3.0 - 2.0 * fade);
    }
}

impl Filter for Vignette {
    fn apply(&self, frame: &mut Framebuffer) {
        let (width, height) = (frame.width(), frame.height());
        for y in 0..height {
            let dy = centred(y, height);
            let Some(row) = frame.row_mut(y) else {
                continue;
            };
            for (x, pixel) in row.iter_mut().enumerate() {
                let dx = centred(x as i32, width);
                let keep = self.keep((dx * dx + dy * dy).sqrt());
                *pixel = pixel.map(|channel| (channel as f32 * keep) as u8);
            }
        }
    }
}

const NOISE_PARAMS: &[Param] = &[Param {
    name: "Amount",
    min: 0.0,
    max: 0.5,
    step: 0.02,
    default: 0.08,
}];

/// Static: every pixel goes a random bit lighter or darker, different each frame.
/// [`Filter::advance_frame`] moves `frame` on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Noise {
    /// The most a pixel changes, out of 1.0.
    pub amount: f32,
    pub frame: u32,
}

impl Default for Noise {
    fn default() -> Self {
        return Self {
            amount: NOISE_PARAMS[0].default,
            frame: 0,
        };
    }
}

impl FilterParams for Noise {
    fn params(&self) -> &'static [Param] {
        return NOISE_PARAMS;
    }

    fn param(&self, index: usize) -> f32 {
        return if index == 0 { self.amount } else { 0.0 };
    }

    fn set_param(&mut self, index: usize, value: f32) {
        if let Some(param) = NOISE_PARAMS.get(index) {
            self.amount = param.clamp(value);
        }
    }
}

impl Filter for Noise {
    fn advance_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    fn apply(&self, frame: &mut Framebuffer) {
        let most = self.amount * u8::MAX as f32;
        for (index, pixel) in frame.pixels_mut().iter_mut().enumerate() {
            let change = noise(index as u32, self.frame) * most;
            *pixel = pixel.map(|channel| (channel as f32 + change).clamp(0.0, 255.0) as u8);
        }
    }
}

// -1.0 at the left or top edge to 1.0 at the right or bottom, through a pixel's middle
fn centred(position: i32, size: i32) -> f32 {
    return (position as f32 + 0.5) / size.max(1) as f32 * 2.0 - 1.0;
}

// the pixel at `centred` position `d`
fn uncentred(d: f32, size: i32) -> i32 {
    return ((d + 1.0) * 0.5 * size as f32).floor() as i32;
}

// large odd numbers that spread nearby inputs far apart: the golden ratio as a u32, and two
// mixing constants from MurmurHash3
const GOLDEN_RATIO: u32 = 0x9E37_79B9;
const MURMUR_MIX_1: u32 = 0x85EB_CA6B;
const MURMUR_MIX_2: u32 = 0x2C1B_3C6D;
const FIRST_SHIFT: u32 = 15;
const SECOND_SHIFT: u32 = 12;

// hash function, -1.0..1.0, full deterministic
fn noise(index: u32, frame: u32) -> f32 {
    let mut hash = index.wrapping_mul(GOLDEN_RATIO) ^ frame.wrapping_mul(MURMUR_MIX_1);
    hash ^= hash >> FIRST_SHIFT;
    hash = hash.wrapping_mul(MURMUR_MIX_2);
    hash ^= hash >> SECOND_SHIFT;
    return hash as f32 / u32::MAX as f32 * 2.0 - 1.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rect::Rect,
        target::{RenderTarget, Rgba},
    };

    fn grey(width: i32, height: i32) -> Framebuffer {
        let mut frame = Framebuffer::new(width, height);
        frame.clear(Rgba::rgb(100, 100, 100));
        return frame;
    }

    #[test]
    fn scanlines_darken_every_other_line() {
        let mut frame = grey(2, 4);
        Scanlines {
            strength: 0.5,
            spacing: 2,
        }
        .apply(&mut frame);
        assert_eq!(frame.pixel(0, 0), Some([100; 3]));
        assert_eq!(frame.pixel(0, 1), Some([50; 3]));
        assert_eq!(frame.pixel(1, 2), Some([100; 3]));
        assert_eq!(frame.pixel(1, 3), Some([50; 3]));
    }

    #[test]
    fn chromatic_aberration_splits_red_and_blue() {
        let mut frame = Framebuffer::new(5, 1);
        frame.fill(Rect::new(2, 0, 1, 1), Rgba::rgb(255, 255, 255));
        ChromaticAberration { offset: 1 }.apply(&mut frame);
        assert_eq!(frame.pixel(1, 0), Some([255, 0, 0]));
        assert_eq!(frame.pixel(2, 0), Some([0, 255, 0]));
        assert_eq!(frame.pixel(3, 0), Some([0, 0, 255]));
    }

    #[test]
    fn wobble_moves_lines_sideways_and_changes_with_the_frame() {
        let mut stripe = Framebuffer::new(16, 64);
        stripe.fill(Rect::new(8, 0, 1, 64), Rgba::rgb(255, 255, 255));
        let wobble = VhsWobble {
            amount: 3.0,
            jitter: 0.0,
            ..VhsWobble::default()
        };
        let mut first = stripe.clone();
        wobble.apply(&mut first);
        assert_ne!(first, stripe);
        let mut later = stripe.clone();
        VhsWobble {
            frame: 16,
            ..wobble
        }
        .apply(&mut later);
        assert_ne!(later, first);
    }

    #[test]
    fn curvature_pulls_the_corners_in() {
        let mut frame = Framebuffer::new(20, 20);
        frame.fill(Rect::new(0, 0, 20, 20), Rgba::rgb(255, 255, 255));
        Curvature { amount: 0.3 }.apply(&mut frame);
        assert_eq!(frame.pixel(0, 0), Some([0; 3]));
        assert_eq!(frame.pixel(10, 10), Some([255; 3]));
        let mut flat = grey(4, 4);
        Curvature { amount: 0.0 }.apply(&mut flat);
        assert_eq!(flat, grey(4, 4));
    }

    #[test]
    fn vignette_darkens_towards_the_edges() {
        let mut frame = grey(20, 20);
        Vignette {
            strength: 1.0,
            size: 0.0,
        }
        .apply(&mut frame);
        let [corner, edge, between, middle] =
            [(0, 0), (0, 10), (5, 10), (10, 10)].map(|(x, y)| frame.pixel(x, y).unwrap()[0]);
        assert!(corner <= edge && edge < between && between < middle);
        assert!(edge < 10 && middle >= 95);
    }

    #[test]
    fn noise_changes_pixels_by_up_to_the_amount_and_each_frame() {
        let noise = Noise {
            amount: 0.1,
            frame: 0,
        };
        let mut first = grey(16, 16);
        noise.apply(&mut first);
        assert_ne!(first, grey(16, 16));
        assert!(
            first
                .pixels()
                .iter()
                .all(|pixel| (74..=126).contains(&pixel[0]))
        );
        let mut later = grey(16, 16);
        Noise { frame: 1, ..noise }.apply(&mut later);
        assert_ne!(later, first);
    }

    #[test]
    fn params_by_index_are_clamped() {
        let mut filters: Vec<Box<dyn AdjustableFilter>> = vec![
            Box::new(Scanlines::default()),
            Box::new(ChromaticAberration::default()),
            Box::new(VhsWobble::default()),
            Box::new(ColorAdjust::default()),
            Box::new(Curvature::default()),
            Box::new(Vignette::default()),
            Box::new(Noise::default()),
        ];
        for filter in &mut filters {
            for (index, param) in filter.params().iter().enumerate() {
                assert_eq!(filter.param(index), param.default, "{}", param.name);
                filter.set_param(index, param.max + 100.0);
                assert_eq!(filter.param(index), param.max, "{}", param.name);
            }
            filter.set_param(99, 1.0);
        }
    }

    #[test]
    fn advance_frame_moves_the_wobble_on() {
        let mut wobble = VhsWobble::default();
        wobble.advance_frame();
        assert_eq!(wobble.frame, 1);
    }

    fn colors() -> Framebuffer {
        let mut frame = Framebuffer::new(4, 1);
        let colors = [(255, 0, 0), (0, 200, 0), (30, 60, 220), (128, 128, 128)];
        for (x, (r, g, b)) in colors.into_iter().enumerate() {
            frame.fill(Rect::new(x as i32, 0, 1, 1), Rgba::rgb(r, g, b));
        }
        return frame;
    }

    #[test]
    fn color_adjust_defaults_change_nothing() {
        let mut frame = colors();
        ColorAdjust::default().apply(&mut frame);
        for (x, pixel) in colors().pixels().iter().enumerate() {
            let adjusted = frame.pixel(x as i32, 0).unwrap();
            for channel in 0..3 {
                let difference = adjusted[channel].abs_diff(pixel[channel]);
                assert!(difference <= 2, "{adjusted:?} {pixel:?}");
            }
        }
    }

    #[test]
    fn color_adjust_knobs() {
        let adjusted = |adjust: ColorAdjust| {
            let mut frame = colors();
            adjust.apply(&mut frame);
            return frame.pixel(0, 0).unwrap();
        };
        let grey = adjusted(ColorAdjust {
            saturation: 0.0,
            ..ColorAdjust::default()
        });
        assert!(grey[0] == grey[1] && grey[1] == grey[2]);
        let black = adjusted(ColorAdjust {
            brightness: 0.0,
            ..ColorAdjust::default()
        });
        assert_eq!(black, [0, 0, 0]);
        // half way round, red goes to a blue green
        let turned = adjusted(ColorAdjust {
            hue: 180.0,
            ..ColorAdjust::default()
        });
        assert!(turned[0] < turned[1] && turned[0] < turned[2], "{turned:?}");
        let cool = adjusted(ColorAdjust {
            warmth: -1.0,
            saturation: 0.0,
            ..ColorAdjust::default()
        });
        assert!(cool[2] > cool[0], "{cool:?}");
    }

    #[test]
    fn closures_and_lists_are_filters() {
        let invert = |frame: &mut Framebuffer| {
            for pixel in frame.pixels_mut() {
                *pixel = pixel.map(|channel| u8::MAX - channel);
            }
        };
        let scanlines = Scanlines::default();
        let filters: [&dyn Filter; 2] = [&invert, &scanlines];
        let mut frame = grey(1, 2);
        filters.as_slice().apply(&mut frame);
        assert_eq!(frame.pixel(0, 0), Some([155; 3]));
        assert_eq!(frame.pixel(0, 1), Some([108; 3]));
    }
}
