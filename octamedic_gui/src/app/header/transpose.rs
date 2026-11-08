use octamedic_core::data::{instrument::InstrumentId, note::OctamedicNote};
use octamedic_util::base32;
use rustuition::{
    font::Font,
    gadget::Gadget,
    gadgets::{button::ContentArea, slider::SliderDirection},
    painter::{Align, Painter, Pen},
    rect::Rect,
};

use crate::{
    app::{
        fonts,
        header::labels::{Picture, Words, draw_words, picture, placed, placed_with_picture},
        main::block_editor::note_text,
        state::AppState,
        transpose::Affect,
    },
    framework::{
        event::Event,
        widgets::{self, Button, SteppedSlider, Textbox, ToggleGroup},
    },
};

// from the panel's top left, measured from OctaMED
const AFFECT_BUTTONS: [(Affect, Rect, Font, Words); 5] = [
    (
        Affect::Song,
        Rect::new(60, 0, 50, 11),
        fonts::LABEL,
        &[(10, "SONG")],
    ),
    (
        Affect::Block,
        Rect::new(110, 0, 52, 11),
        fonts::LABEL,
        &[(8, "BLOCK")],
    ),
    (
        Affect::Track,
        Rect::new(162, 0, 50, 11),
        fonts::LABEL,
        &[(6, "TRACK")],
    ),
    (
        Affect::SelectedTracks,
        Rect::new(212, 0, 56, 11),
        fonts::NARROW_LABEL,
        &[(4, "SEL"), (24, "."), (27, "TRKS")],
    ),
    (
        Affect::Range,
        Rect::new(268, 0, 54, 11),
        fonts::LABEL,
        &[(7, "RANGE")],
    ),
];
const ALL: Rect = Rect::new(370, 0, 34, 11);
const CURR: Rect = Rect::new(404, 0, 44, 11);

const NOTE_FROM: Rect = Rect::new(168, 11, 28, 11);
const NOTE_CHANGE: Rect = Rect::new(196, 11, 20, 11);
const NOTE_EXCHANGE: Rect = Rect::new(216, 11, 20, 11);
const NOTE_TO: Rect = Rect::new(236, 11, 28, 11);
const HALF_STEP_UP: Rect = Rect::new(264, 11, 44, 11);
const HALF_STEP_DOWN: Rect = Rect::new(308, 11, 44, 11);
const OCTAVE_UP: Rect = Rect::new(352, 11, 48, 11);
const OCTAVE_DOWN: Rect = Rect::new(400, 11, 48, 11);

const DEL: Rect = Rect::new(2, 22, 34, 11);
const INSTRUMENT_FROM: Rect = Rect::new(36, 22, 20, 11);
const INSTRUMENT_CHANGE: Rect = Rect::new(56, 22, 20, 11);
const INSTRUMENT_EXCHANGE: Rect = Rect::new(76, 22, 20, 11);
const INSTRUMENT_TO: Rect = Rect::new(96, 22, 20, 11);
const PLAY_TRANSPOSE: Rect = Rect::new(241, 22, 176, 11);
const PLAY_TRANSPOSE_VALUE: Rect = Rect::new(417, 22, 31, 11);
const PLAY_TRANSPOSE_ARROW: [i32; 2] = [8, 7];
const PLAY_TRANSPOSE_ARROW_WIDTH: i32 = 22;
const PLAY_TRANSPOSE_HANDLE: i32 = 6;
const PLAY_TRANSPOSE_LIMIT: i8 = 12;

const TEXT: Rect = Rect::new(40, 33, 408, 11);
const TEXT_LENGTH: usize = 69;

const LABELS: [([i32; 2], Words); 6] = [
    ([12, 3], &[(0, "AFFECT"), (44, ":")]),
    ([329, 3], &[(0, "INSTR"), (37, ":")]),
    ([5, 15], &[(0, "INSTR"), (37, ":")]),
    ([132, 14], &[(0, "NO"), (16, "TE"), (31, ":")]),
    ([128, 25], &[(0, "PLAY"), (37, "TRANSPOSE"), (109, ":")]),
    ([5, 36], &[(0, "TEXT"), (31, ":")]),
];

const CHANGE_ARROW: Picture = Picture {
    at: [4, 3],
    rows: &[
        "......##....",
        "......####..",
        "############",
        "......####..",
        "......##....",
    ],
};
const EXCHANGE_ARROW: Picture = Picture {
    at: [3, 3],
    rows: &[
        "....##..##....",
        "..####..####..",
        "##############",
        "..####..####..",
        "....##..##....",
    ],
};
const HALF_STEP_UP_ARROW: Picture = Picture {
    at: [29, 2],
    rows: &[
        "...##...", "..####..", ".######.", "########", "...##...", "...##...",
    ],
};

const HALF_STEP_DOWN_ARROW: Picture = Picture {
    at: [29, 2],
    rows: &[
        "...##...", "...##...", "########", ".######.", "..####..", "...##...",
    ],
};

const OCTAVE_UP_ARROW: Picture = Picture {
    at: [33, 2],
    rows: &[
        "...##...", "..####..", ".######.", "########", "...##...", "...##...",
    ],
};

const OCTAVE_DOWN_ARROW: Picture = Picture {
    at: [33, 2],
    rows: &[
        "...##...", "...##...", "########", ".######.", "..####..", "...##...",
    ],
};

pub struct Transpose {
    affect: Affect,
    current_only: bool,
    notes: [OctamedicNote; 2],
    instruments: [InstrumentId; 2],
    affect_buttons: [Button; 5],
    all: Button,
    curr: Button,
    note_boxes: [Button; 2],
    note_change: Button,
    note_exchange: Button,
    steps: [(Button, i16, Rect); 4],
    delete: Button,
    instrument_boxes: [Button; 2],
    instrument_change: Button,
    instrument_exchange: Button,
    play_transpose: SteppedSlider,
    play_transpose_value: Button,
    text: Textbox,
}

impl Transpose {
    pub fn new() -> Self {
        let label = |words: Words| Button::new(placed(fonts::LABEL, words));
        let number_box = |text: &'static str| Button::new(text).font(fonts::INTERFACE).sunken();
        return Self {
            affect: Affect::default(),
            current_only: false,
            notes: [OctamedicNote::new(); 2],
            instruments: [InstrumentId(0); 2],
            affect_buttons: AFFECT_BUTTONS
                .map(|(_, _, font, words)| Button::new(placed(font, words))),
            all: label(&[(8, "ALL")]),
            curr: label(&[(8, "CURR")]),
            note_boxes: std::array::from_fn(|_| Button::new("---").font(fonts::INTERFACE)),
            note_change: Button::new(picture(&CHANGE_ARROW)),
            note_exchange: Button::new(picture(&EXCHANGE_ARROW)),
            steps: [
                (Button::new(half_step(&HALF_STEP_UP_ARROW)), 1, HALF_STEP_UP),
                (
                    Button::new(half_step(&HALF_STEP_DOWN_ARROW)),
                    -1,
                    HALF_STEP_DOWN,
                ),
                (Button::new(octave(&OCTAVE_UP_ARROW)), 12, OCTAVE_UP),
                (Button::new(octave(&OCTAVE_DOWN_ARROW)), -12, OCTAVE_DOWN),
            ],
            delete: label(&[(7, "DEL")]),
            instrument_boxes: std::array::from_fn(|_| number_box("00")),
            instrument_change: Button::new(picture(&CHANGE_ARROW)),
            instrument_exchange: Button::new(picture(&EXCHANGE_ARROW)),
            play_transpose: SteppedSlider::new(
                SliderDirection::Horizontal,
                0,
                2 * PLAY_TRANSPOSE_LIMIT as u16,
                PLAY_TRANSPOSE_ARROW,
                PLAY_TRANSPOSE_ARROW_WIDTH,
            )
            .solid()
            .fixed_handle(PLAY_TRANSPOSE_HANDLE),
            play_transpose_value: number_box("00").align(Align::Right),
            text: widgets::textbox()
                .font(fonts::INTERFACE)
                .max_length(TEXT_LENGTH)
                .sunken()
                .on_update(|state, text| state.set_song_text(text)),
        };
    }

    fn update_choices(&mut self) {
        let affects = self.affect_buttons.iter_mut().zip(AFFECT_BUTTONS);
        let mut affect =
            ToggleGroup::new(affects.map(|(button, (affect, _, _, _))| (button, affect)));
        if let Some((picked, _)) = affect.take_click() {
            self.affect = picked;
        }
        affect.sync(Some(self.affect));
        let mut instruments = ToggleGroup::new([(&mut self.all, false), (&mut self.curr, true)]);
        if let Some((picked, _)) = instruments.take_click() {
            self.current_only = picked;
        }
        instruments.sync(Some(self.current_only));
    }

    fn update_notes(&mut self, state: &mut AppState) {
        for (note, note_box) in self.notes.iter().zip(&mut self.note_boxes) {
            note_box.set_text(note_text(note.value));
            let held = note_box.is_pressed();
            note_box.set_toggled(held);
        }
        let (affect, current_only) = (self.affect, self.current_only);
        if self.note_change.clicked().is_some() {
            state.change_notes(affect, current_only, self.notes, false);
        }
        if self.note_exchange.clicked().is_some() {
            state.change_notes(affect, current_only, self.notes, true);
        }
        for (button, semitones, _) in &mut self.steps {
            if button.clicked().is_some() {
                state.transpose_notes(affect, current_only, *semitones);
            }
        }
    }

    fn update_instruments(&mut self, state: &mut AppState) {
        for (index, instrument_box) in self.instrument_boxes.iter_mut().enumerate() {
            if instrument_box.clicked().is_some() {
                self.instruments[index] = state.active_instrument;
            }
            instrument_box.set_text(base32::format(self.instruments[index].0 as u32, 2, '0'));
        }
        if self.delete.clicked().is_some() {
            state.delete_instrument_notes(self.affect, self.instruments[0]);
        }
        if self.instrument_change.clicked().is_some() {
            state.change_instruments(self.affect, self.instruments, false);
        }
        if self.instrument_exchange.clicked().is_some() {
            state.change_instruments(self.affect, self.instruments, true);
        }
    }

    fn update_play_transpose(&mut self, state: &mut AppState) {
        let current = state
            .song()
            .global_transpose
            .clamp(-PLAY_TRANSPOSE_LIMIT, PLAY_TRANSPOSE_LIMIT);
        let position = (current + PLAY_TRANSPOSE_LIMIT) as u16;
        if let Some(slid) = self.play_transpose.update(state, position) {
            state.set_play_transpose(slid as i8 - PLAY_TRANSPOSE_LIMIT);
        }
        let semitones = state.song().global_transpose;
        self.play_transpose_value
            .set_text(format!("{semitones:02}"));
    }

    fn pick_note(&mut self, state: &AppState, event: &Event) -> bool {
        let Event::Text { character, .. } = *event else {
            return false;
        };
        let Some(held) = self.note_boxes.iter().position(Button::is_pressed) else {
            return false;
        };
        let Some(note) = state.key_to_note(character) else {
            return false;
        };
        self.notes[held] = note;
        return true;
    }

    fn buttons(&mut self) -> impl Iterator<Item = (&mut Button, Rect)> {
        let affects = self
            .affect_buttons
            .iter_mut()
            .zip(AFFECT_BUTTONS)
            .map(|(button, (_, rect, _, _))| (button, rect));
        let steps = self
            .steps
            .iter_mut()
            .map(|(button, _, rect)| (button, *rect));
        let [note_from, note_to] = &mut self.note_boxes;
        let [instrument_from, instrument_to] = &mut self.instrument_boxes;
        return affects.chain(steps).chain([
            (&mut self.all, ALL),
            (&mut self.curr, CURR),
            (note_from, NOTE_FROM),
            (&mut self.note_change, NOTE_CHANGE),
            (&mut self.note_exchange, NOTE_EXCHANGE),
            (note_to, NOTE_TO),
            (&mut self.delete, DEL),
            (instrument_from, INSTRUMENT_FROM),
            (&mut self.instrument_change, INSTRUMENT_CHANGE),
            (&mut self.instrument_exchange, INSTRUMENT_EXCHANGE),
            (instrument_to, INSTRUMENT_TO),
            (&mut self.play_transpose_value, PLAY_TRANSPOSE_VALUE),
        ]);
    }
}

fn half_step(arrow: &'static Picture) -> impl Fn(ContentArea, Pen, &mut Painter) {
    return placed_with_picture(fonts::LABEL, &[(7, "1/"), (21, "2")], arrow);
}

fn octave(arrow: &'static Picture) -> impl Fn(ContentArea, Pen, &mut Painter) {
    return placed_with_picture(fonts::LABEL, &[(9, "OCT")], arrow);
}

impl Default for Transpose {
    fn default() -> Self {
        Self::new()
    }
}

impl Gadget<AppState, Event> for Transpose {
    fn update(&mut self, state: &mut AppState) {
        self.update_choices();
        self.update_notes(state);
        self.update_instruments(state);
        self.update_play_transpose(state);
        if !self.text.is_focused() {
            let text = state.project().annotation.clone();
            self.text.set_text(text);
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        if self.pick_note(state, event) {
            return true;
        }
        let typed = self.text.handle_event(state, event);
        return typed
            || self.play_transpose.handle_event(state, event)
            || self
                .buttons()
                .any(|(button, _)| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        for ([x, y], words) in LABELS {
            draw_words(
                painter,
                fonts::LABEL,
                [area.x + x, area.y + y],
                words,
                Pen::BACKGROUND,
            );
        }
        for (button, rect) in self.buttons() {
            button.render(state, rect.within(area), painter);
        }
        self.play_transpose
            .render(state, PLAY_TRANSPOSE.within(area), painter);
        self.text.render(state, TEXT.within(area), painter);
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::{event::MouseButton, rect::Point, target::Framebuffer};

    use super::*;

    const AREA: Rect = Rect::new(0, 0, 448, 45);

    fn click(panel: &mut Transpose, state: &mut AppState, rect: Rect) {
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        panel.render(state, AREA, &mut Painter::new(&mut screen, &state.palette));
        let pos = Point::new(rect.x + 3, rect.y + 3);
        let button = MouseButton::Left;
        panel.handle_event(state, &Event::MouseDown { pos, button });
        panel.handle_event(state, &Event::MouseUp { pos, button });
        panel.update(state);
    }

    fn type_key(panel: &mut Transpose, state: &mut AppState, character: char) {
        let event = Event::Text {
            character,
            mods: ggez::input::keyboard::KeyMods::NONE,
            repeat: false,
        };
        assert!(panel.handle_event(state, &event));
        panel.update(state);
    }

    fn first_note(state: &AppState) -> u8 {
        return state.active_pattern().unwrap().lines[0].tracks[0]
            .note
            .value;
    }

    fn state_with_a_note() -> AppState {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let cell = &mut state.active_pattern_mut().unwrap().lines[0].tracks[0];
        cell.note.value = 13;
        cell.instrument_id = InstrumentId(1);
        return state;
    }

    #[test]
    fn affect_and_instr_pick_one_each() {
        let mut state = state_with_a_note();
        let mut panel = Transpose::new();
        panel.update(&mut state);
        assert!(panel.affect_buttons[0].is_toggled() && panel.all.is_toggled());
        click(&mut panel, &mut state, AFFECT_BUTTONS[2].1);
        click(&mut panel, &mut state, CURR);
        assert_eq!(panel.affect, Affect::Track);
        assert!(panel.current_only && panel.curr.is_toggled() && !panel.all.is_toggled());
        assert!(!panel.affect_buttons[0].is_toggled());
    }

    #[test]
    fn half_steps_and_octaves() {
        let mut state = state_with_a_note();
        let mut panel = Transpose::new();
        click(&mut panel, &mut state, HALF_STEP_UP);
        assert_eq!(first_note(&state), 14);
        click(&mut panel, &mut state, OCTAVE_DOWN);
        assert_eq!(first_note(&state), 2);
    }

    fn hold(panel: &mut Transpose, state: &mut AppState, rect: Rect, down: bool) {
        let pos = Point::new(rect.x + 3, rect.y + 3);
        let button = MouseButton::Left;
        let event = match down {
            true => Event::MouseDown { pos, button },
            false => Event::MouseUp { pos, button },
        };
        panel.handle_event(state, &event);
        panel.update(state);
    }

    #[test]
    fn a_note_box_takes_a_note_key_while_its_held() {
        let mut state = state_with_a_note();
        let mut panel = Transpose::new();
        click(&mut panel, &mut state, Rect::default());
        let from = state.key_to_note('z').unwrap();
        state.active_pattern_mut().unwrap().lines[0].tracks[0].note = from;
        hold(&mut panel, &mut state, NOTE_FROM, true);
        assert!(panel.note_boxes[0].is_toggled());
        type_key(&mut panel, &mut state, 'z');
        hold(&mut panel, &mut state, NOTE_FROM, false);
        assert!(!panel.note_boxes[0].is_toggled());
        click(&mut panel, &mut state, NOTE_TO);
        let typed = Event::Text {
            character: 'x',
            mods: ggez::input::keyboard::KeyMods::NONE,
            repeat: false,
        };
        assert!(!panel.handle_event(&mut state, &typed));
        hold(&mut panel, &mut state, NOTE_TO, true);
        type_key(&mut panel, &mut state, 'x');
        hold(&mut panel, &mut state, NOTE_TO, false);
        assert_eq!(panel.notes, [from, state.key_to_note('x').unwrap()]);
        click(&mut panel, &mut state, NOTE_CHANGE);
        assert_eq!(first_note(&state), state.key_to_note('x').unwrap().value);
    }

    #[test]
    fn instrument_boxes_take_the_current_instrument() {
        let mut state = state_with_a_note();
        let mut panel = Transpose::new();
        click(&mut panel, &mut state, INSTRUMENT_FROM);
        state.active_instrument = InstrumentId(2);
        click(&mut panel, &mut state, INSTRUMENT_TO);
        assert_eq!(panel.instruments, [InstrumentId(1), InstrumentId(2)]);
        click(&mut panel, &mut state, INSTRUMENT_CHANGE);
        let cell = state.active_pattern().unwrap().lines[0].tracks[0];
        assert_eq!(cell.instrument_id, InstrumentId(2));
    }

    #[test]
    fn play_transpose_arrows_change_the_song() {
        let mut state = state_with_a_note();
        let mut panel = Transpose::new();
        click(&mut panel, &mut state, Rect::default());
        let [down, up] = panel
            .play_transpose
            .arrows()
            .clone()
            .map(|arrow| arrow.area());
        click(&mut panel, &mut state, up);
        click(&mut panel, &mut state, up);
        click(&mut panel, &mut state, down);
        assert_eq!(state.song().global_transpose, 1);
    }

    #[test]
    fn text_is_the_song_text() {
        let mut state = state_with_a_note();
        let mut panel = Transpose::new();
        state.project_mut().annotation = "by me".into();
        panel.update(&mut state);
        assert_eq!(panel.text.text(), "by me");
        click(&mut panel, &mut state, TEXT);
        type_key(&mut panel, &mut state, '!');
        assert_eq!(state.project().annotation, "!by me");
    }
}
