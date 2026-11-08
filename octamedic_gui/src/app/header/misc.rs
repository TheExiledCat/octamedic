use mmd::mmd::module::PALETTE_COLORS;
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
        header::labels::{Words, draw_words, placed, placed_at},
        state::{AppState, screen_palette},
    },
    framework::{
        event::Event,
        widgets::{Button, Slider, ToggleGroup, symbols::stacked_text},
    },
};

// from the panel's top left
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
const COLOR_HANDLE_LENGTH: i32 = 9;

// from the sections' top left. none of them do anything yet. the cramped ones are in the narrow
// font, like on the real thing
const PREFS_BUTTONS: [(Font, Words, Rect); 10] = [
    (
        fonts::NARROW_LABEL,
        &[(4, "P"), (10, "T"), (16, "K"), (22, "EY")],
        Rect::new(0, 0, 37, 11),
    ),
    (fonts::LABEL, &[(5, "ADD")], Rect::new(82, 0, 32, 11)),
    (fonts::LABEL, &[(5, "REM")], Rect::new(114, 0, 32, 11)),
    (fonts::LABEL, &[(7, "OPEN")], Rect::new(60, 11, 43, 11)),
    (
        fonts::NARROW_LABEL,
        &[(6, "CLOSE")],
        Rect::new(103, 11, 43, 11),
    ),
    (
        fonts::NARROW_LABEL,
        &[(6, "LOADGFX")],
        Rect::new(0, 22, 57, 11),
    ),
    (
        fonts::LABEL,
        &[(4, "MOUSE"), (45, "2")],
        Rect::new(57, 22, 54, 11),
    ),
    (
        fonts::NARROW_LABEL,
        &[(6, "SAVE"), (38, "PREFS")],
        Rect::new(0, 33, 76, 11),
    ),
    (
        fonts::NARROW_LABEL,
        &[(5, "NO1"), (26, "6")],
        Rect::new(76, 33, 35, 11),
    ),
    (
        fonts::NARROW_LABEL,
        &[(5, "DEP2")],
        Rect::new(111, 33, 35, 11),
    ),
];
const HOST_TO_BUFFER: Rect = Rect::new(0, 11, 37, 11);
// each with its top
const PREFS_LABELS: [(i32, Words); 2] = [(3, &[(39, "PATHS:")]), (14, &[(39, "WB"), (56, ":")])];
const CLEAR_ALL: Rect = Rect::new(0, 0, 118, 11);
const CLEAR_SONG: Rect = Rect::new(0, 11, 118, 11);
const QUIT: Rect = Rect::new(0, 22, 118, 11);
const CONFIRM: Rect = Rect::new(0, 33, 118, 12);
// which of danger_buttons is picked, before CONFIRM does it. CONFIRM is the last
const CLEAR_ALL_CHOICE: usize = 0;
const CLEAR_SONG_CHOICE: usize = 1;
const QUIT_CHOICE: usize = 2;

// the picked colour's sunk in frame, with the pens' top bitplane flipped the way OctaMED does it
const SELECTED_TOP: Pen = Pen(6);
const SELECTED_BOTTOM: Pen = Pen(5);

const QUIT_LETTERS: Words = &[(41, "Q"), (52, "U"), (63, "I"), (70, "T")];
const CONFIRM_WORDS: Words = &[(30, "CONFIRM")];

// within the button's frame
const HOST_TO_BUFFER_PARTS: [i32; 3] = [3, 14, 22];
const HOST_TO_BUFFER_Y: i32 = 2;
const ARROW_SIZE: [i32; 2] = [6, 5];
// CONFIRM's arrows: black rows 2 wider each line down, with a shine down their right side
// from the left and right
const CONFIRM_ARROW_INSETS: [i32; 2] = [6, 4];
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
    danger_buttons: Vec<(Button, Rect)>,
    selected_color: usize,
    // the level last set on each slider, so a drag shows up as a change
    levels: [u8; Channel::ALL.len()],
    // the palette when the panel was first shown, which UNDO restores
    undo_palette: Option<IntuitionColorPalette>,
    danger_button_selected: Option<usize>,
}

impl Misc {
    pub fn new() -> Self {
        let steps = CHANNEL_MAX as u16 + 1;
        return Self {
            sliders: std::array::from_fn(|_| {
                Slider::new(SliderDirection::Horizontal, false, steps)
                    .fixed_handle(COLOR_HANDLE_LENGTH)
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
                .map(|(font, words, rect)| (unhooked(Button::new(placed(font, words))), rect))
                .chain([(unhooked(Button::new(host_to_buffer)), HOST_TO_BUFFER)])
                .collect(),
            danger_buttons: vec![
                (
                    Button::new(placed(fonts::LABEL, &[(28, "CLEAR"), (71, "ALL")])),
                    CLEAR_ALL,
                ),
                (
                    Button::new("CLEAR SONG ONLY").font(fonts::LABEL),
                    CLEAR_SONG,
                ),
                (
                    Button::new(placed_at(fonts::QUIT_LABEL, 2, QUIT_LETTERS)),
                    QUIT,
                ),
                (Button::new(confirm), CONFIRM),
            ],
            selected_color: 0,
            levels: [0; Channel::ALL.len()],
            undo_palette: None,
            danger_button_selected: None,
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
    fn danger(&mut self, state: &mut AppState) {
        let Some(choice) = self.danger_button_selected.take() else {
            return;
        };
        for (button, _) in &mut self.danger_buttons {
            button.set_toggled(false);
        }
        let action: fn(&mut AppState) = match choice {
            CLEAR_ALL_CHOICE => AppState::clear_all,
            CLEAR_SONG_CHOICE => AppState::clear_song,
            QUIT_CHOICE => AppState::request_exit,
            _ => return,
        };
        state.confirm_if_unsaved(action);
    }
}

impl Default for Misc {
    fn default() -> Self {
        return Self::new();
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
        let (confirm, choices) = self
            .danger_buttons
            .split_last_mut()
            .expect("CONFIRM and its choices");
        let mut choice_buttons =
            ToggleGroup::new(choices.iter_mut().enumerate().map(|(choice, (button, _))| {
                return (button, choice);
            }));
        if let Some((choice, _)) = choice_buttons.take_click() {
            self.danger_button_selected = Some(choice);
        }
        choice_buttons.sync(self.danger_button_selected);
        if confirm.0.clicked().is_some() {
            self.danger(state);
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
                .chain(&mut self.danger_buttons)
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
            painter.text_in_font(
                row.x + x,
                row.y + y,
                letter,
                Pen::BACKGROUND,
                fonts::COLOR_LETTERS,
            );
            self.sliders[channel].render(state, SLIDER.within(row), painter);
            self.values[channel].render(state, VALUE.within(row), painter);
        }
        for pen in 0..PALETTE_COLORS {
            let color_box = Self::color_box(area, pen);
            self.color_buttons[pen].render(state, color_box, painter);
            if pen == self.selected_color {
                painter.bevel(color_box, 1, SELECTED_TOP, SELECTED_BOTTOM);
            }
        }
        self.res.render(state, RES.within(area), painter);
        self.undo.render(state, UNDO.within(area), painter);

        let prefs = PREFS_SECTION.within(area);
        for (button, rect) in &mut self.prefs {
            button.render(state, rect.within(prefs), painter);
        }
        for (top, words) in PREFS_LABELS {
            let at = [prefs.x, prefs.y + top];
            draw_words(painter, fonts::LABEL, at, words, Pen::BACKGROUND);
        }
        let project = PROJECT_SECTION.within(area);
        for (button, rect) in &mut self.danger_buttons {
            button.render(state, rect.within(project), painter);
        }
    }
}

fn unhooked(button: Button) -> Button {
    return button.enabled(false);
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
    placed(fonts::CONFIRM_LABEL, CONFIRM_WORDS)(area, pen, painter);
    let inside = area.inside;
    let [left_inset, right_inset] = CONFIRM_ARROW_INSETS;
    let lefts = [
        inside.x + left_inset,
        inside.right() - right_inset - CONFIRM_ARROW_WIDTH - CONFIRM_ARROW_SHINE,
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

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};

    use super::*;
    use crate::app::prompt::Prompt;

    fn confirm_quit(state: &mut AppState) {
        let mut misc = Misc::new();
        misc.danger_button_selected = Some(QUIT_CHOICE);
        misc.danger(state);
        assert_eq!(misc.danger_button_selected, None);
    }

    #[test]
    fn saved_goes_ahead_without_asking() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        confirm_quit(&mut state);
        assert!(state.exit_requested);
        assert!(state.prompt.is_none());
    }

    #[test]
    fn unsaved_asks_first() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        state.set_song_name("changed");
        confirm_quit(&mut state);
        assert!(!state.exit_requested);
        assert_eq!(
            state.prompt.as_ref().map(Prompt::text),
            Some("Song has unsaved changes (Y/N)")
        );
    }
}
