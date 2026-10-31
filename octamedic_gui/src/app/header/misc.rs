use octamed::mmd::module::PALETTE_COLORS;
use octamedic_core::data::palette::{CHANNEL_MAX, Channel, OctamedColorPalette};

use crate::{
    app::state::AppState,
    framework::{
        event::Event,
        font::Font,
        painter::{Align, Painter, Pen},
        rect::Rect,
        widget::OctamedicWidget,
        widgets::{
            button::{Button, ButtonContent, ContentArea},
            slider::{Slider, SliderDirection},
        },
    },
};

// screenshot measurements, panel's top left
const COLOR_SECTION: Rect = Rect::new(0, 2, 182, 42);
const SLIDERS: Rect = Rect::new(0, 2, 160, 30);
const COLOR_BOXES: Rect = Rect::new(0, 32, 160, 12);
const RES: Rect = Rect::new(160, 2, 22, 21);
const UNDO: Rect = Rect::new(160, 23, 22, 21);
const PREFS_SECTION: Rect = Rect::new(182, 0, 146, 44);
const PROJECT_SECTION: Rect = Rect::new(330, 0, 118, 45);

// within a slider row
const ROW_HEIGHT: i32 = 10;
const LETTER: [i32; 2] = [2, 2];
const SLIDER: Rect = Rect::new(14, 0, 134, ROW_HEIGHT);
const VALUE: Rect = Rect::new(148, 0, 12, ROW_HEIGHT);
const CHANNEL_LETTERS: [&str; 3] = ["R", "G", "B"];

const COLOR_BOX_WIDTH: i32 = 20;

const STACKED_LINE: i32 = 6;

pub struct Misc {
    sliders: [Slider; Channel::ALL.len()],
    values: [Button; Channel::ALL.len()],
    color_buttons: [Button; PALETTE_COLORS],
    res: Button,
    undo: Button,
    selected_color: usize,
    // the level last set on each slider, so a drag shows up as a change
    levels: [u8; Channel::ALL.len()],
    // the palette when the panel was first shown, which UNDO restores
    undo_palette: Option<OctamedColorPalette>,
}

impl Misc {
    pub fn new() -> Self {
        let steps = CHANNEL_MAX as u16 + 1;
        return Self {
            sliders: std::array::from_fn(|_| {
                Slider::new(SliderDirection::Horizontal, false, steps)
            }),
            values: std::array::from_fn(|_| {
                Button::new("0")
                    .font(Font::Interface)
                    .align(Align::Left)
                    .padding(0)
            }),
            color_buttons: std::array::from_fn(|pen| Button::new(Swatch(Pen(pen)))),
            res: Button::new(StackedLabel {
                lines: &["R", "E", "S"],
                offset: [7, 1],
            }),
            undo: Button::new(StackedLabel {
                lines: &["UN", "DO"],
                offset: [2, 4],
            }),
            selected_color: 0,
            levels: [0; Channel::ALL.len()],
            undo_palette: None,
        };
    }

    fn row(area: Rect, channel: usize) -> Rect {
        let sliders = SLIDERS.within(area);
        return Rect::new(
            sliders.x,
            sliders.y + channel as i32 * ROW_HEIGHT,
            sliders.w,
            ROW_HEIGHT,
        );
    }

    fn color_box(area: Rect, pen: usize) -> Rect {
        let boxes = COLOR_BOXES.within(area);
        return Rect::new(
            boxes.x + pen as i32 * COLOR_BOX_WIDTH,
            boxes.y,
            COLOR_BOX_WIDTH,
            boxes.h,
        );
    }
}

impl OctamedicWidget for Misc {
    fn update(&mut self, state: &mut AppState) {
        let undo_palette = *self.undo_palette.get_or_insert(state.palette);
        if self.res.clicked().is_some() {
            state.palette = OctamedColorPalette::DEFAULT;
        }
        if self.undo.clicked().is_some() {
            state.palette = undo_palette;
        }
        for (pen, button) in self.color_buttons.iter_mut().enumerate() {
            if button.clicked().is_some() {
                self.selected_color = pen;
            }
        }
        let pen = self.selected_color;
        for (index, channel) in Channel::ALL.into_iter().enumerate() {
            let dragged = self.sliders[index].value() as u8;
            if dragged != self.levels[index] {
                state.palette.set_channel(pen, channel, dragged);
            }
            let level = state.palette.channel(pen, channel);
            self.sliders[index].set_value(level as u16);
            self.levels[index] = level;
            self.values[index].set_text(format!("{level:X}"));
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.res.handle_event(state, event)
            || self.undo.handle_event(state, event)
            || self
                .color_buttons
                .iter_mut()
                .any(|button| button.handle_event(state, event))
            || self
                .sliders
                .iter_mut()
                .any(|slider| slider.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        for (channel, letter) in CHANNEL_LETTERS.iter().enumerate() {
            let row = Self::row(area, channel);
            let [x, y] = LETTER;
            painter.text_in_font(row.x + x, row.y + y, letter, Pen::BLACK, Font::Panel);
            self.sliders[channel].render(state, SLIDER.within(row), painter);
            self.values[channel].render(state, VALUE.within(row), painter);
        }
        for pen in 0..PALETTE_COLORS {
            let color_box = Self::color_box(area, pen);
            self.color_buttons[pen].render(state, color_box, painter);
            if pen == self.selected_color {
                painter.frame_inverted(color_box, 1);
            }
        }
        self.res.render(state, RES.within(area), painter);
        self.undo.render(state, UNDO.within(area), painter);
    }
}

struct Swatch(Pen);

impl ButtonContent for Swatch {
    fn render(&self, area: ContentArea, _pen: Pen, painter: &mut Painter) {
        painter.fill(area.inside, self.0);
    }
}

struct StackedLabel {
    lines: &'static [&'static str],
    offset: [i32; 2],
}

impl ButtonContent for StackedLabel {
    fn render(&self, area: ContentArea, pen: Pen, painter: &mut Painter) {
        let [x, y] = self.offset;
        for (line, text) in self.lines.iter().enumerate() {
            let line_y = area.inside.y + y + line as i32 * STACKED_LINE;
            painter.text_in_font(area.inside.x + x, line_y, text, pen, Font::Small);
        }
    }
}
