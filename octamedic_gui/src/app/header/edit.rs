use rustuition::{
    gadget::Gadget,
    painter::{Painter, Pen},
    rect::Rect,
};

use crate::{
    app::{
        advance::{CommandAdvance, NoteAdvance},
        fonts,
        header::{
            labels::{Picture, Words, draw_words, picture, placed, placed_at},
            range,
        },
        keypad::KeypadMap,
        state::{AppState, MAX_OCTAVE, SELECTABLE_TRACKS},
    },
    framework::{
        event::Event,
        widgets::{self, Button, Textbox, ToggleGroup, symbols::stacked_text},
    },
};

// from the panel's top left, measured from OctaMED
const TRACK_ON_OFF: Rect = Rect::new(58, 11, 73, 11);
const SELECT_INSTRUMENT: Rect = Rect::new(131, 11, 65, 11);
const MAP2: Rect = Rect::new(196, 11, 36, 11);
const SPACE_DEL: Rect = Rect::new(368, 11, 80, 10);
const EDIT_ON_RING: Rect = Rect::new(368, 21, 80, 12);
const EDIT_ON: Rect = Rect::new(370, 22, 76, 10);
const SPC: Rect = Rect::new(368, 33, 44, 12);
const LINE_ADD: Rect = Rect::new(412, 33, 36, 12);
const LINE_ADD_DIGITS: usize = 2;
const MAX_LINE_ADD: u8 = 16;

const OCTAVE_FIRST: Rect = Rect::new(58, 22, 17, 12);

const SPEAKER: Rect = Rect::new(35, 34, 23, 11);
static NOTE_ADVANCES: [(NoteAdvance, Rect, Picture); 4] = [
    (NoteAdvance::Up, Rect::new(58, 34, 23, 11), UP_ARROW),
    (NoteAdvance::Down, Rect::new(81, 34, 23, 11), DOWN_ARROW),
    (
        NoteAdvance::TrackRight,
        Rect::new(104, 34, 23, 11),
        TRACK_RIGHT_ARROW,
    ),
    (
        NoteAdvance::TrackLeft,
        Rect::new(127, 34, 23, 11),
        TRACK_LEFT_ARROW,
    ),
];
static COMMAND_ADVANCES: [(CommandAdvance, Rect, Picture); 2] = [
    (
        CommandAdvance::Right,
        Rect::new(150, 34, 23, 11),
        RIGHT_ARROW,
    ),
    (CommandAdvance::Left, Rect::new(173, 34, 23, 11), LEFT_ARROW),
];

// the programmable keys, laid out but not hooked up yet
const KEY_CLR: Rect = Rect::new(232, 11, 34, 23);
const KEY_NUMBER: Rect = Rect::new(266, 11, 13, 12);
const KEY_DEFINITION: Rect = Rect::new(292, 11, 76, 12);
const PREVIOUS_KEY: Rect = Rect::new(266, 23, 20, 11);
const NEXT_KEY: Rect = Rect::new(286, 23, 20, 11);
const LEFT_ALT: Rect = Rect::new(306, 23, 31, 11);
const RIGHT_ALT: Rect = Rect::new(337, 23, 31, 11);
const PICK_NOTE: Rect = Rect::new(232, 34, 46, 11);
const PICK_RANGE: Rect = Rect::new(278, 34, 54, 11);
const PICK_BUFFER: Rect = Rect::new(332, 34, 36, 11);

const LABELS: [([i32; 2], Words); 5] = [
    ([1, 3], &[(0, "TRK"), (26, "ON"), (43, ":")]),
    ([6, 14], &[(0, "KEYPAD"), (47, ":")]),
    ([7, 25], &[(0, "OCTAVE"), (46, ":")]),
    ([7, 37], &[(0, "ADV.")]),
    ([201, 37], &[(0, "PICK"), (27, ":")]),
];
const EQUALS: [i32; 2] = [282, 14];
// the T tucks under the end of the L
const LEFT_ALT_LETTERS: Words = &[(3, "L"), (9, "."), (11, "A"), (18, "L"), (22, "T")];
const RIGHT_ALT_LETTERS: Words = &[(3, "R"), (10, "."), (12, "A"), (19, "L"), (23, "T")];
const KEY_CLR_LINES: [(&str, [i32; 2]); 3] = [("C", [12, 2]), ("L", [12, 8]), ("R", [12, 14])];
const OCTAVE_LABELS: [Words; MAX_OCTAVE as usize] = [
    &[(6, "1")],
    &[(6, "2")],
    &[(5, "3")],
    &[(5, "4")],
    &[(6, "5")],
    &[(6, "6")],
    &[(6, "7")],
    &[(6, "8")],
    &[(6, "9")],
];

const SPEAKER_LABEL: Picture = Picture {
    at: [4, 2],
    rows: &[
        ".......###......",
        ".....##.##.##..#",
        "######..##...##.",
        "##..##..##......",
        "######..##.##..#",
        ".....##.##...##.",
        ".......###......",
    ],
};
const UP_ARROW: Picture = Picture {
    at: [5, 4],
    rows: &[
        "......##......",
        "....######....",
        "..##########..",
        "##############",
    ],
};
const DOWN_ARROW: Picture = Picture {
    at: [5, 4],
    rows: &[
        "##############",
        "..##########..",
        "....######....",
        "......##......",
    ],
};
const TRACK_RIGHT_ARROW: Picture = Picture {
    at: [4, 2],
    rows: &[
        "##......##......",
        "####....####....",
        "######..######..",
        "################",
        "######..######..",
        "####....####....",
        "##......##......",
    ],
};
const TRACK_LEFT_ARROW: Picture = Picture {
    at: [3, 2],
    rows: &[
        "......##......##",
        "....####....####",
        "..######..######",
        "################",
        "..######..######",
        "....####....####",
        "......##......##",
    ],
};
const RIGHT_ARROW: Picture = Picture {
    at: [8, 2],
    rows: &[
        "##......", "####....", "######..", "########", "######..", "####....", "##......",
    ],
};
const LEFT_ARROW: Picture = Picture {
    at: [7, 2],
    rows: &[
        "......##", "....####", "..######", "########", "..######", "....####", "......##",
    ],
};
const PREVIOUS_KEY_ARROW: Picture = Picture {
    at: [5, 2],
    ..LEFT_ARROW
};
const NEXT_KEY_ARROW: Picture = Picture {
    at: [7, 2],
    ..RIGHT_ARROW
};

pub struct Edit {
    tracks_on: [Button; SELECTABLE_TRACKS],
    all_on: Button,
    all_off: Button,
    track_on_off: Button,
    select_instrument: Button,
    map2: Button,
    space_del: Button,
    edit_on: Button,
    spc: Button,
    line_add: Textbox,
    octaves: [Button; MAX_OCTAVE as usize],
    speaker: Button,
    note_advances: [Button; 4],
    command_advances: [Button; 2],
    unhooked: Vec<(Button, Rect)>,
}

impl Edit {
    pub fn new() -> Self {
        let label = |words: Words| Button::new(placed(fonts::LABEL, words));
        let narrow = |words: Words| Button::new(placed(fonts::NARROW_LABEL, words));
        let small_dot = |words: Words| Button::new(placed(fonts::LABEL_SMALL_DOT, words));
        let unhooked = |button: Button, rect: Rect| (button.enabled(false), rect);
        return Self {
            tracks_on: std::array::from_fn(track_digit),
            all_on: Button::new(placed_at(fonts::TALL, 2, &[(3, "SET")])),
            all_off: Button::new(placed_at(fonts::TALL, 2, &[(3, "CLR")])),
            track_on_off: narrow(&[(4, "TRK"), (29, "ON/OFF")]),
            select_instrument: narrow(&[
                (4, "S"),
                (11, "E"),
                (18, "L"),
                (25, "."),
                (30, "IN"),
                (41, "S"),
                (47, "T"),
                (54, "R"),
            ]),
            map2: narrow(&[(5, "MAP"), (27, "2")]),
            space_del: Button::new(placed_at(
                fonts::NARROW_LABEL,
                2,
                &[(11, "SPACE"), (45, "="), (51, "DEL")],
            )),
            edit_on: Button::new(placed_at(fonts::BIG, 2, &[(8, "EDIT ON")])),
            spc: Button::new(placed(fonts::BIG, &[(8, "SPC")])),
            line_add: widgets::textbox()
                .font(fonts::INTERFACE)
                .padding(0)
                .max_length(LINE_ADD_DIGITS)
                .on_update(|state, typed| {
                    if let Ok(add) = typed.parse::<u8>() {
                        state.line_add = add.clamp(1, MAX_LINE_ADD);
                    }
                }),
            octaves: OCTAVE_LABELS.map(|digit| Button::new(placed(fonts::BIG, digit))),
            speaker: Button::new(picture(&SPEAKER_LABEL)),
            note_advances: std::array::from_fn(|index| {
                Button::new(picture(&NOTE_ADVANCES[index].2))
            }),
            command_advances: std::array::from_fn(|index| {
                Button::new(picture(&COMMAND_ADVANCES[index].2))
            }),
            unhooked: vec![
                unhooked(Button::new(stacked_text(&KEY_CLR_LINES)), KEY_CLR),
                unhooked(
                    Button::new(placed_at(fonts::INTERFACE, 2, &[(4, "0")])).sunken(),
                    KEY_NUMBER,
                ),
                unhooked(
                    Button::new("--- 00000").font(fonts::INTERFACE),
                    KEY_DEFINITION,
                ),
                unhooked(Button::new(picture(&PREVIOUS_KEY_ARROW)), PREVIOUS_KEY),
                unhooked(Button::new(picture(&NEXT_KEY_ARROW)), NEXT_KEY),
                unhooked(small_dot(LEFT_ALT_LETTERS), LEFT_ALT),
                unhooked(small_dot(RIGHT_ALT_LETTERS), RIGHT_ALT),
                unhooked(label(&[(8, "NOTE")]), PICK_NOTE),
                unhooked(label(&[(9, "RANGE")]), PICK_RANGE),
                unhooked(label(&[(8, "BUF")]), PICK_BUFFER),
            ],
        };
    }

    fn update_tracks(&mut self, state: &mut AppState) {
        for (track, button) in self.tracks_on.iter_mut().enumerate() {
            if button.clicked().is_some() {
                state.toggle_track_mute(track);
            }
            button.set_toggled(!state.is_track_muted(track));
        }
        if self.all_on.clicked().is_some() {
            state.set_all_tracks_muted(false);
        }
        if self.all_off.clicked().is_some() {
            state.set_all_tracks_muted(true);
        }
    }

    fn update_keypad(&mut self, state: &mut AppState) {
        let mut maps = ToggleGroup::new([
            (&mut self.track_on_off, KeypadMap::TrackOnOff),
            (&mut self.select_instrument, KeypadMap::SelectInstrument),
        ]);
        if let Some((picked, _)) = maps.take_click() {
            state.keypad_map = picked;
        }
        maps.sync(Some(state.keypad_map));
        if self.map2.clicked().is_some() {
            state.keypad_map2 = !state.keypad_map2;
        }
        self.map2.set_toggled(state.keypad_map2);
    }

    fn update_switches(&mut self, state: &mut AppState) {
        let switches = [
            (&mut self.space_del, &mut state.space_deletes),
            (&mut self.edit_on, &mut state.cursor.edit_mode),
            (&mut self.spc, &mut state.spacing),
            (&mut self.speaker, &mut state.play_on_move),
        ];
        for (button, on) in switches {
            if button.clicked().is_some() {
                *on = !*on;
            }
            button.set_toggled(*on);
        }
        if !self.line_add.is_focused() {
            self.line_add.set_text(state.line_add.to_string());
        }
    }

    fn update_octaves(&mut self, state: &mut AppState) {
        let octaves = self.octaves.iter_mut().zip(1..=MAX_OCTAVE);
        let mut octaves = ToggleGroup::new(octaves);
        if let Some((picked, _)) = octaves.take_click() {
            state.set_octave(picked);
        }
        octaves.sync(Some(state.current_octave));
    }

    fn update_advance(&mut self, state: &mut AppState) {
        let notes = self.note_advances.iter_mut().zip(&NOTE_ADVANCES);
        let mut notes = ToggleGroup::new(notes.map(|(button, (advance, _, _))| (button, *advance)));
        if let Some((picked, _)) = notes.take_click() {
            state.note_advance = picked;
        }
        notes.sync(Some(state.note_advance));
        // unlike the note ones, a command advance can be switched off again
        let commands = self.command_advances.iter_mut().zip(&COMMAND_ADVANCES);
        let mut commands =
            ToggleGroup::new(commands.map(|(button, (advance, _, _))| (button, *advance)));
        if let Some((picked, _)) = commands.take_click() {
            let off = state.command_advance == Some(picked);
            state.command_advance = if off { None } else { Some(picked) };
        }
        commands.sync(state.command_advance);
    }

    fn buttons(&mut self) -> impl Iterator<Item = (&mut Button, Rect)> {
        let tracks = self
            .tracks_on
            .iter_mut()
            .enumerate()
            .map(|(track, button)| (button, range::Range::select_rect(track)));
        let octaves = self
            .octaves
            .iter_mut()
            .enumerate()
            .map(|(index, button)| (button, octave_rect(index)));
        let notes = self
            .note_advances
            .iter_mut()
            .zip(&NOTE_ADVANCES)
            .map(|(button, (_, rect, _))| (button, *rect));
        let commands = self
            .command_advances
            .iter_mut()
            .zip(&COMMAND_ADVANCES)
            .map(|(button, (_, rect, _))| (button, *rect));
        let unhooked = self
            .unhooked
            .iter_mut()
            .map(|(button, rect)| (button, *rect));
        return tracks
            .chain(octaves)
            .chain(notes)
            .chain(commands)
            .chain(unhooked)
            .chain([
                (&mut self.all_on, range::SET),
                (&mut self.all_off, range::CLR),
                (&mut self.track_on_off, TRACK_ON_OFF),
                (&mut self.select_instrument, SELECT_INSTRUMENT),
                (&mut self.map2, MAP2),
                (&mut self.space_del, SPACE_DEL),
                (&mut self.edit_on, EDIT_ON),
                (&mut self.spc, SPC),
                (&mut self.speaker, SPEAKER),
            ]);
    }
}

impl Default for Edit {
    fn default() -> Self {
        Self::new()
    }
}

impl Gadget<AppState, Event> for Edit {
    fn update(&mut self, state: &mut AppState) {
        self.update_tracks(state);
        self.update_keypad(state);
        self.update_switches(state);
        self.update_octaves(state);
        self.update_advance(state);
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        let typed = self.line_add.handle_event(state, event);
        return typed
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
        let [x, y] = EQUALS;
        painter.text_in_font(
            area.x + x,
            area.y + y,
            "=",
            Pen::BACKGROUND,
            fonts::INTERFACE,
        );
        painter.fill(EDIT_ON_RING.within(area), Pen::BACKGROUND);
        for (button, rect) in self.buttons() {
            button.render(state, rect.within(area), painter);
        }
        self.line_add.render(state, LINE_ADD.within(area), painter);
    }
}

pub fn track_digit(track: usize) -> Button {
    return Button::new(format!("{track:X}")).font(fonts::TRACKS);
}

fn octave_rect(index: usize) -> Rect {
    let x = OCTAVE_FIRST.x + index as i32 * OCTAVE_FIRST.w;
    return Rect::new(x, OCTAVE_FIRST.y, OCTAVE_FIRST.w, OCTAVE_FIRST.h);
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::{event::MouseButton, rect::Point, target::Framebuffer};

    use super::*;

    const AREA: Rect = Rect::new(0, 0, 448, 45);

    fn click(panel: &mut Edit, state: &mut AppState, rect: Rect) {
        let mut screen = Framebuffer::new(AREA.w, AREA.h);
        panel.render(state, AREA, &mut Painter::new(&mut screen, &state.palette));
        let pos = Point::new(rect.x + 3, rect.y + 3);
        let button = MouseButton::Left;
        panel.handle_event(state, &Event::MouseDown { pos, button });
        panel.handle_event(state, &Event::MouseUp { pos, button });
        panel.update(state);
    }

    fn setup() -> (Edit, AppState) {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut panel = Edit::new();
        panel.update(&mut state);
        return (panel, state);
    }

    #[test]
    fn track_buttons_mute_and_unmute() {
        let (mut panel, mut state) = setup();
        assert!(panel.tracks_on[3].is_toggled());
        click(&mut panel, &mut state, range::Range::select_rect(3));
        assert!(state.is_track_muted(3) && !panel.tracks_on[3].is_toggled());
        click(&mut panel, &mut state, range::CLR);
        assert!((0..SELECTABLE_TRACKS).all(|track| state.is_track_muted(track)));
        click(&mut panel, &mut state, range::SET);
        assert!((0..SELECTABLE_TRACKS).all(|track| !state.is_track_muted(track)));
    }

    #[test]
    fn keypad_map_buttons() {
        let (mut panel, mut state) = setup();
        assert!(panel.track_on_off.is_toggled());
        click(&mut panel, &mut state, SELECT_INSTRUMENT);
        click(&mut panel, &mut state, MAP2);
        assert_eq!(state.keypad_map, KeypadMap::SelectInstrument);
        assert!(state.keypad_map2 && panel.map2.is_toggled());
        assert!(!panel.track_on_off.is_toggled());
    }

    #[test]
    fn octave_buttons_pick_the_octave() {
        let (mut panel, mut state) = setup();
        click(&mut panel, &mut state, octave_rect(4));
        assert_eq!(state.current_octave, 5);
        assert!(panel.octaves[4].is_toggled() && !panel.octaves[0].is_toggled());
    }

    #[test]
    fn advance_buttons() {
        let (mut panel, mut state) = setup();
        assert!(panel.note_advances[1].is_toggled());
        click(&mut panel, &mut state, NOTE_ADVANCES[2].1);
        assert_eq!(state.note_advance, NoteAdvance::TrackRight);
        click(&mut panel, &mut state, COMMAND_ADVANCES[1].1);
        assert_eq!(state.command_advance, Some(CommandAdvance::Left));
        click(&mut panel, &mut state, COMMAND_ADVANCES[1].1);
        assert_eq!(state.command_advance, None);
        click(&mut panel, &mut state, SPEAKER);
        assert!(state.play_on_move);
    }

    #[test]
    fn switches_and_the_line_add() {
        let (mut panel, mut state) = setup();
        for rect in [EDIT_ON, SPC, SPACE_DEL] {
            click(&mut panel, &mut state, rect);
        }
        assert!(state.cursor.edit_mode && state.spacing && state.space_deletes);
        assert!(panel.edit_on.is_toggled());
        let end_of_line_add = Rect::new(LINE_ADD.right() - 6, LINE_ADD.y, 4, 4);
        click(&mut panel, &mut state, end_of_line_add);
        let event = Event::Key {
            key: ggez::input::keyboard::KeyCode::Back,
            mods: ggez::input::keyboard::KeyMods::NONE,
            repeat: false,
        };
        panel.handle_event(&mut state, &event);
        panel.handle_event(&mut state, &Event::text('5'));
        assert_eq!(state.line_add, 5);
    }

    #[test]
    fn the_programmable_keys_are_ghosted() {
        let (panel, _) = setup();
        assert!(
            panel
                .unhooked
                .iter()
                .all(|(button, _)| !button.is_enabled())
        );
    }
}
