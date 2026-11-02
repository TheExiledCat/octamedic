use octamed::mmd::module::PALETTE_COLORS;
use octamedic_core::data::palette::OctamedColorPalette;
use rustuition::{
    font::Font,
    gadget::Gadget,
    gadgets::{button::ContentArea, slider::SliderDirection},
    painter::{Align, Painter, Pen},
    palette::{CHANNEL_MAX, Channel, IntuitionColorPalette},
    rect::Rect,
    shape::Direction,
};

use crate::{
    app::{
        fonts,
        state::{AppState, screen_palette},
    },
    framework::{
        event::Event,
        widgets::{Button, Slider, symbols::stacked_text},
    },
};

// from the panel's top left
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

// from the sections' top left. none of them do anything yet. the cramped ones are in the narrow
// font, like on the real thing
const PREFS_BUTTONS: [(&str, Rect, Font); 10] = [
    ("PTKEY", Rect::new(0, 0, 37, 11), fonts::NARROW_LABEL),
    ("ADD", Rect::new(82, 0, 32, 11), fonts::LABEL),
    ("REM", Rect::new(114, 0, 32, 11), fonts::LABEL),
    ("OPEN", Rect::new(60, 11, 43, 11), fonts::LABEL),
    ("CLOSE", Rect::new(103, 11, 43, 11), fonts::LABEL),
    ("LOADGFX", Rect::new(0, 22, 57, 11), fonts::NARROW_LABEL),
    ("MOUSE2", Rect::new(57, 22, 54, 11), fonts::NARROW_LABEL),
    ("SAVE PREFS", Rect::new(0, 33, 76, 11), fonts::NARROW_LABEL),
    ("NO16", Rect::new(76, 33, 35, 11), fonts::NARROW_LABEL),
    ("DEP2", Rect::new(111, 33, 35, 11), fonts::NARROW_LABEL),
];
const HOST_TO_BUFFER: Rect = Rect::new(0, 11, 37, 11);
const PREFS_LABELS: [(&str, [i32; 2]); 2] = [("PATHS:", [39, 3]), ("WB:", [39, 14])];
const CLEAR_ALL: Rect = Rect::new(0, 0, 118, 11);
const CLEAR_SONG: Rect = Rect::new(0, 11, 118, 11);
const QUIT: Rect = Rect::new(0, 22, 118, 11);
const CONFIRM: Rect = Rect::new(0, 33, 118, 12);

// within the button's frame
const HOST_TO_BUFFER_PARTS: [i32; 3] = [4, 14, 23];
const HOST_TO_BUFFER_Y: i32 = 2;
const ARROW_SIZE: [i32; 2] = [6, 5];
// CONFIRM's arrows: black rows 2 wider each line down, with a shine down their right side
const CONFIRM_ARROW_INSET: i32 = 4;
const CONFIRM_ARROW_ROWS: i32 = 4;
const CONFIRM_ARROW_WIDTH: i32 = 14;
const CONFIRM_ARROW_SHINE: i32 = 2;

const RES_LINES: [(&str, [i32; 2]); 3] = [("R", [7, 1]), ("E", [7, 7]), ("S", [7, 13])];
const UNDO_LINES: [(&str, [i32; 2]); 2] = [("UN", [2, 4]), ("DO", [2, 10])];

pub struct Misc {
    sliders: [Slider; Channel::ALL.len()],
    values: [Button; Channel::ALL.len()],
    color_buttons: [Button; PALETTE_COLORS],
    res: Button,
    undo: Button,
    // not hooked up yet, each with where it goes in its section
    prefs: Vec<(Button, Rect)>,
    project: Vec<(Button, Rect)>,
    selected_color: usize,
    // the level last set on each slider, so a drag shows up as a change
    levels: [u8; Channel::ALL.len()],
    // the palette when the panel was first shown, which UNDO restores
    undo_palette: Option<IntuitionColorPalette>,
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
                    .font(fonts::INTERFACE)
                    .align(Align::Left)
                    .padding(0)
            }),
            color_buttons: std::array::from_fn(|pen| {
                Button::new(move |area: ContentArea, _, painter: &mut Painter| {
                    painter.fill(area.inside, Pen(pen));
                })
            }),
            res: Button::new(stacked_text(&RES_LINES)),
            undo: Button::new(stacked_text(&UNDO_LINES)),
            prefs: PREFS_BUTTONS
                .into_iter()
                .map(|(label, rect, font)| (label_button(label, font), rect))
                .chain([(unhooked(Button::new(host_to_buffer)), HOST_TO_BUFFER)])
                .collect(),
            project: vec![
                (label_button("CLEAR ALL", fonts::LABEL), CLEAR_ALL),
                (label_button("CLEAR SONG ONLY", fonts::LABEL), CLEAR_SONG),
                (unhooked(Button::new("QUIT").font(fonts::PANEL)), QUIT),
                (unhooked(Button::new(confirm)), CONFIRM),
            ],
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

impl Gadget<AppState, Event> for Misc {
    fn update(&mut self, state: &mut AppState) {
        let undo_palette = self
            .undo_palette
            .get_or_insert_with(|| state.palette.clone());
        if self.res.clicked().is_some() {
            state.palette = screen_palette(&OctamedColorPalette::DEFAULT);
        }
        if self.undo.clicked().is_some() {
            state.palette = undo_palette.clone();
        }
        for (pen, button) in self.color_buttons.iter_mut().enumerate() {
            if button.clicked().is_some() {
                self.selected_color = pen;
            }
        }
        let pen = Pen(self.selected_color);
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
                .prefs
                .iter_mut()
                .chain(&mut self.project)
                .any(|(button, _)| button.handle_event(state, event))
            || self
                .sliders
                .iter_mut()
                .any(|slider| slider.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        for (channel, letter) in CHANNEL_LETTERS.iter().enumerate() {
            let row = Self::row(area, channel);
            let [x, y] = LETTER;
            painter.text_in_font(row.x + x, row.y + y, letter, Pen::BACKGROUND, fonts::PANEL);
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

        let prefs = PREFS_SECTION.within(area);
        for (button, rect) in &mut self.prefs {
            button.render(state, rect.within(prefs), painter);
        }
        for (label, [x, y]) in PREFS_LABELS {
            painter.text_in_font(
                prefs.x + x,
                prefs.y + y,
                label,
                Pen::BACKGROUND,
                fonts::LABEL,
            );
        }
        let project = PROJECT_SECTION.within(area);
        for (button, rect) in &mut self.project {
            button.render(state, rect.within(project), painter);
        }
    }
}

fn unhooked(button: Button) -> Button {
    return button.enabled(false);
}

// no padding at the sides, so the widest still fit
fn label_button(label: &'static str, font: Font) -> Button {
    return unhooked(Button::new(label).font(font).padding_xy(0, 1));
}

// H, an arrow pointing right, B
fn host_to_buffer(area: ContentArea, pen: Pen, painter: &mut Painter) {
    let [h, arrow, b] = HOST_TO_BUFFER_PARTS.map(|x| area.inside.x + x);
    let y = area.inside.y + HOST_TO_BUFFER_Y;
    painter.text_in_font(h, y, "H", pen, fonts::LABEL);
    let [w, height] = ARROW_SIZE;
    painter.triangle(
        Rect::new(arrow, y, w, height),
        Some(pen),
        None,
        Direction::Right,
    );
    painter.text_in_font(b, y, "B", pen, fonts::LABEL);
}

// CONFIRM between two arrows pointing up
fn confirm(area: ContentArea, pen: Pen, painter: &mut Painter) {
    let inside = area.inside;
    let width = fonts::PANEL.width("CONFIRM");
    let text_x = inside.x + (inside.w - width) / 2;
    let text_y = inside.y + (inside.h - fonts::PANEL.height()) / 2;
    painter.text_in_font(text_x, text_y, "CONFIRM", pen, fonts::PANEL);
    let lefts = [
        inside.x + CONFIRM_ARROW_INSET,
        inside.right() - CONFIRM_ARROW_INSET - CONFIRM_ARROW_WIDTH - CONFIRM_ARROW_SHINE,
    ];
    let top = inside.y + (inside.h - CONFIRM_ARROW_ROWS) / 2;
    for left in lefts {
        let centre = left + CONFIRM_ARROW_WIDTH / 2;
        for row in 0..CONFIRM_ARROW_ROWS {
            let half = 1 + 2 * row;
            let y = top + row;
            painter.fill(Rect::new(centre - half, y, 2 * half, 1), pen);
            let shine = Rect::new(centre + half, y, CONFIRM_ARROW_SHINE, 1);
            painter.fill(shine, Pen::SHINE);
        }
    }
}
