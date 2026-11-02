use ggez::input::keyboard::KeyCode;
use octamedic_util::{base32, primitives::Flip};
use rustuition::{
    gadget::Gadget,
    gadgets::{
        button::{Highlight, Triangle},
        grid::Grid,
    },
    painter::{Align, Painter},
    rect::Rect,
    shape::Direction,
};

use crate::{
    app::{
        fonts::{self, POSITION_SLASH},
        state::{AppState, MAX_OCTAVE},
    },
    framework::{
        event::{Event, MouseButton},
        keymap::{Action, Key, KeyActions, Keymap},
        widgets::Button,
    },
};

// left to right; they leave the row's last pixel empty
const WIDTHS: [i32; 12] = [84, 68, 22, 14, 22, 35, 56, 20, 237, 13, 13, 49];

// indexes into `WIDTHS`
const SONG_POSITION: usize = 0;
const BLOCK: usize = 1;
const OCTAVES: usize = 2;
const EDIT: usize = 3;
const SP: usize = 4;
const CHRD: usize = 5;
const MUTES: usize = 6;
const INSTRUMENT: usize = 7;
const INSTRUMENT_NAME: usize = 8;
const PREVIOUS_SONG: usize = 9;
const NEXT_SONG: usize = 10;
const SONG: usize = 11;

// one mutebox per track, sitting a line higher than the rest of the row
const MUTE_COLUMNS: usize = 8;
const MUTE_ROWS: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatusBarAction {
    NextInstrument,
    PrevInstrument,
}

impl Action for StatusBarAction {}

pub struct StatusBar {
    keys: Keymap<StatusBarAction>,
    boxes: Vec<Button>,
    mutes: Grid<Button>,
}

impl StatusBar {
    pub fn new() -> Self {
        let labels = ["", "", "", "E", "SP", "CHRD", "", "", "", "", "", ""];
        // labels start at the left, as in OctaMED
        let mut boxes: Vec<Button> = labels
            .into_iter()
            .map(|label| Button::new(label).align(Align::Left).font(fonts::INTERFACE))
            .collect();

        boxes[SONG_POSITION].set_on_click(|state, click| {
            let song_length = state.song().sequence().len();
            let advance = click.button == MouseButton::Left;
            let position = state.active_sequence_index;
            let delta = if advance { 1i8 } else { -1i8 };
            let reset_out_of_bounds = |before: usize, after: usize| {
                if after >= song_length { before } else { after }
            };
            // the position's last two digits step it by 10 and by 1
            state.set_sequence(match click.char_index.unwrap_or(0) {
                2 => reset_out_of_bounds(
                    position,
                    position.saturating_add_signed(delta as isize * 10),
                ),
                3 => reset_out_of_bounds(position, position.saturating_add_signed(delta as isize)),
                _ => position,
            });
        });
        boxes[BLOCK].set_on_click(|state, click| {
            const DIGIT_STEPS: [usize; 3] = [100, 10, 1];
            let Some(step) = click.char_index.and_then(|index| DIGIT_STEPS.get(index)) else {
                return;
            };
            let block = state.active_pattern.0 as usize;
            let last = state.song().patterns().len().saturating_sub(1);
            let target = if click.button == MouseButton::Left {
                block.checked_add(*step)
            } else {
                block.checked_sub(*step)
            };
            if let Some(target) = target.filter(|&target| target <= last) {
                state.select_block(target);
            }
        });

        boxes[OCTAVES].set_on_click(|state, click| {
            let octave = state.current_octave;
            let next = match click.button {
                MouseButton::Right if octave <= 1 => MAX_OCTAVE,
                MouseButton::Right => octave - 1,
                _ if octave >= MAX_OCTAVE => 1,
                _ => octave + 1,
            };
            state.set_octave(next);
        });
        boxes[PREVIOUS_SONG] = Button::new(Triangle::arrow(Direction::Left)).enabled(false);
        boxes[NEXT_SONG] = Button::new(Triangle::arrow(Direction::Right)).enabled(false);
        boxes[SONG] = Button::new("").align(Align::Right).font(fonts::INTERFACE);
        boxes[EDIT] = flag("E", 3).on_click(|state, _| state.cursor.edit_mode.flip());
        boxes[SP] = flag("SP", 2).enabled(false);
        boxes[CHRD] = flag("CHRD", 2).enabled(false);
        boxes[INSTRUMENT] = Button::new("")
            .align(Align::Left)
            .font(fonts::INTERFACE)
            .on_click(|state, click| {
                let advance = click.button == MouseButton::Left;
                let delta = if advance { 1i8 } else { -1i8 };
                match click.char_index {
                    Some(0) => {
                        state.set_instrument(
                            state
                                .active_instrument
                                .0
                                .saturating_add_signed(base32::BASE as i8 * delta),
                        );
                    }
                    Some(1) => {
                        if advance {
                            state.next_instrument();
                        } else {
                            state.prev_instrument();
                        }
                    }
                    _ => (),
                }
            });

        let mute_boxes = (0..MUTE_COLUMNS * MUTE_ROWS).map(mute_box).collect();
        return Self {
            keys: Keymap::new("status")
                .bind(
                    Key::shift(KeyCode::Right),
                    StatusBarAction::NextInstrument,
                    "Next instrument",
                )
                .bind(
                    Key::shift(KeyCode::Left),
                    StatusBarAction::PrevInstrument,
                    "Prev instrument",
                ),
            boxes,
            mutes: Grid::new(MUTE_COLUMNS, MUTE_ROWS, mute_boxes),
        };
    }
}

impl Gadget<AppState, Event> for StatusBar {
    fn update(&mut self, state: &mut AppState) {
        let song = state.song();
        let instrument = state.active_instrument;
        let name = song
            .instrument(instrument)
            .map_or(String::new(), |instrument| instrument.name.clone());
        let texts = [
            (
                SONG_POSITION,
                format!(
                    "{:04}{POSITION_SLASH}{:04}",
                    state.active_sequence_index + 1,
                    song.sequence().len()
                ),
            ),
            (
                BLOCK,
                format!(
                    "{:03}{POSITION_SLASH}{:03}",
                    state.active_pattern.0,
                    song.patterns().len().saturating_sub(1)
                ),
            ),
            (OCTAVES, octaves_text(state.current_octave)),
            (INSTRUMENT, base32::format(instrument.0 as u32, 2, '0')),
            (INSTRUMENT_NAME, name),
            (
                SONG,
                format!(
                    "{:02}/{:02}",
                    state.active_song.0 + 1,
                    state.project().song_count()
                ),
            ),
        ];
        for (index, text) in texts {
            self.boxes[index].set_text(text);
        }

        self.boxes[EDIT].set_toggled(state.cursor.edit_mode);
        for (track, button) in self.mutes.cells_mut().iter_mut().enumerate() {
            button.set_toggled(!state.is_track_muted(track));
        }
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.dispatch(state, event)
            || self.mutes.handle_event(state, event)
            || self
                .boxes
                .iter_mut()
                .any(|button| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        for (index, (button, rect)) in self.boxes.iter_mut().zip(area.columns(&WIDTHS)).enumerate()
        {
            if index == MUTES {
                let mutes = Rect::new(rect.x, rect.y - 1, rect.w, rect.h + 1);
                self.mutes.render(state, mutes, painter);
            } else {
                button.render(state, rect, painter);
            }
        }
    }
}

// small capitals, set further in than the numbers
fn flag(label: &'static str, padding: i32) -> Button {
    return Button::new(label)
        .align(Align::Left)
        .font(fonts::SMALL)
        .padding_xy(padding, 1);
}

fn mute_box(track: usize) -> Button {
    return Button::new("")
        .highlight(Highlight::Recess)
        .on_click(move |state, click| {
            if click.button == MouseButton::Left {
                state.toggle_track_mute(track);
            }
        });
}

// the bottom row's octave, then the top row's, in hex like OctaMED, so the last is 9A
fn octaves_text(octave: u8) -> String {
    return format!("{octave:X}{:X}", octave + 1);
}

impl KeyActions for StatusBar {
    type Action = StatusBarAction;

    fn keymap(&self) -> &Keymap<StatusBarAction> {
        return &self.keys;
    }

    fn perform(&mut self, action: StatusBarAction, state: &mut AppState) {
        match action {
            StatusBarAction::NextInstrument => state.next_instrument(),
            StatusBarAction::PrevInstrument => state.prev_instrument(),
        };
    }

    fn describe(action: StatusBarAction) -> &'static str {
        return match action {
            StatusBarAction::NextInstrument => "Next instrument",
            StatusBarAction::PrevInstrument => "Previous instrument",
        };
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{data::project::OctamedicProject, engine::handle::EngineHandle};
    use rustuition::target::Framebuffer;

    use super::*;

    #[test]
    fn muteboxes_toggle_tracks() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut status = StatusBar::new();
        let area = Rect::new(0, 0, 56, 12);
        let mut screen = Framebuffer::new(56, 12);
        status.update(&mut state);
        status
            .mutes
            .render(&state, area, &mut Painter::new(&mut screen, &state.palette));
        let cell = status.mutes.cell_area(area, 9);
        let press = Event::MouseDown {
            pos: [cell.x + 1, cell.y + 1],
            button: MouseButton::Left,
        };
        assert!(status.mutes.cells()[9].is_toggled());
        status.mutes.handle_event(&mut state, &press);
        status.update(&mut state);
        let muted: Vec<usize> = (0..16)
            .filter(|&track| state.is_track_muted(track))
            .collect();
        assert_eq!(muted, [9]);
        assert!(!status.mutes.cells()[9].is_toggled());
    }

    #[test]
    fn octaves_box_shows_and_loops() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut status = StatusBar::new();
        let area = Rect::new(0, 0, 640, 12);
        let mut screen = Framebuffer::new(640, 12);
        status.update(&mut state);
        status.render(&state, area, &mut Painter::new(&mut screen, &state.palette));
        let octaves = status.boxes[OCTAVES].area();
        let at = [octaves.x + 3, octaves.y + 3];
        let press = |button| Event::MouseDown { pos: at, button };
        let release = |button| Event::MouseUp { pos: at, button };
        let mut shown = vec![octaves_text(state.current_octave)];
        for _ in 0..MAX_OCTAVE {
            status.handle_event(&mut state, &press(MouseButton::Left));
            status.handle_event(&mut state, &release(MouseButton::Left));
            shown.push(octaves_text(state.current_octave));
        }
        assert_eq!(
            shown,
            ["12", "23", "34", "45", "56", "67", "78", "89", "9A", "12"]
        );
        status.handle_event(&mut state, &press(MouseButton::Right));
        assert_eq!(state.current_octave, MAX_OCTAVE);
    }

    #[test]
    fn shift_arrows_change_instrument() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut status = StatusBar::new();
        let shift = |key| Event::Key {
            key,
            mods: ggez::input::keyboard::KeyMods::SHIFT,
            repeat: false,
        };
        assert!(status.handle_event(&mut state, &shift(KeyCode::Right)));
        assert!(status.handle_event(&mut state, &shift(KeyCode::Right)));
        assert_eq!(state.active_instrument.0, 3);
        status.handle_event(&mut state, &shift(KeyCode::Left));
        assert_eq!(state.active_instrument.0, 2);
        let plain = Event::Key {
            key: KeyCode::Left,
            mods: ggez::input::keyboard::KeyMods::NONE,
            repeat: false,
        };
        assert!(!status.handle_event(&mut state, &plain));
    }
}
