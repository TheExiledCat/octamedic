use std::array;

use ggez::input::keyboard::KeyMods;
use octamedic_core::data::{
    pattern::PatternId,
    tempo::{MAX_BPM, MAX_LINES_PER_BEAT, MAX_TICKS_PER_LINE, OctamedicTempo},
};
use rustuition::{
    gadget::Gadget,
    gadgets::{
        button::{ArrowLabel, ContentArea, IncDec, Triangle},
        slider::SliderDirection,
    },
    painter::{
        Align::{self, Right},
        CHAR_HEIGHT, Painter, Pen, text_y,
    },
    rect::Rect,
    shape::Direction::{Down, Up},
    text::{Line, Span},
};

use crate::{
    app::{
        fonts::{self, POSITION_SLASH},
        header::labels::{Words, draw_words, placed},
        state::{AppState, PlayStart},
    },
    framework::{
        event::{Event, MouseButton},
        widgets::{Button, SteppedSlider},
    },
};
const SEQUENCER_JUMP_BUTTONS: usize = 4;
const INC_DEC_BUTTONS: usize = 2;
const EDIT_BUTTONS: usize = 3;
const SEQUENCER_JUMP_SEPARATOR_ROW: usize = 2;
const JUMP_ARROW_WIDTH: i32 = 12;
// pixels are twice as tall as they're wide, so this looks square
const JUMP_ARROW_HEIGHT: i32 = JUMP_ARROW_WIDTH / 2;
const JUMP_BUTTON_WIDTH: i32 = 20;
const INC_DEC_BUTTON_HEIGHT: i32 = 10;
const INC_DEC_LINE_WIDTH: i32 = 10;
const INC_DEC_LINE_HEIGHT: i32 = 5;
const EDIT_BUTTON_HEIGHT: i32 = 11;
const INS_LABEL_OFFSET: i32 = 6;
const OUTER_JUMP_BUTTON_HEIGHT: i32 = 11;
const INNER_JUMP_BUTTON_HEIGHT: i32 = 10;
const JUMP_SEPARATOR_HEIGHT: i32 = 1;
const SEQUENCE_HEIGHT: i32 = 43;
const SEQUENCE_NUMBERS_RECT: Rect = Rect::new(0, 0, 36, SEQUENCE_HEIGHT);
const JUMP_BUTTONS_RECT: Rect = Rect::new(
    SEQUENCE_NUMBERS_RECT.right(),
    0,
    JUMP_BUTTON_WIDTH,
    SEQUENCE_HEIGHT,
);
const BUTTONS_RECT: Rect = Rect::new(JUMP_BUTTONS_RECT.right(), 0, 36, SEQUENCE_HEIGHT);
const SEQUENCE_RECT: Rect = Rect::new(2, 1, BUTTONS_RECT.right(), SEQUENCE_HEIGHT);

const SEQUENCE_COUNT_MAX: usize = 5;
const ACTIVE_ROW: usize = SEQUENCE_COUNT_MAX / 2;
const SEQUENCE_TOP_MARGIN: i32 = 1;

// from the panel's top left
// each slider with its arrows
const TEMPO_SLIDERS: [Rect; 2] = [Rect::new(94, 0, 226, 15), Rect::new(320, 0, 128, 15)];
const STEP_WIDTH: i32 = 18;
const STEP_ARROW: [i32; 2] = [8, 7];
// OctaMED's tempo handles don't scale with the range
const TEMPO_HANDLE_LENGTH: i32 = 6;
// the O sits a pixel closer to the M than the font has it
const CHANNEL_MODE_WORDS: Words = &[(0, "CH.M"), (24, "ODE:")];
const TEMPO_MODE: Rect = Rect::new(347, 15, 36, 11);
const TEMPO: Rect = Rect::new(383, 15, 65, 11);
const VOL_LABEL: [i32; 2] = [95, 18];
const HEX: Rect = Rect::new(122, 15, 30, 11);
const DEC: Rect = Rect::new(152, 15, 30, 11);
const FILTER: Rect = Rect::new(182, 15, 51, 11);
const METERS: Rect = Rect::new(233, 15, 78, 11);
const STS: Rect = Rect::new(311, 15, 30, 11);
const CHANNEL_MODE_LABEL: [i32; 2] = [97, 28];
const HQ: Rect = Rect::new(145, 26, 26, 9);
const CHANNEL_MODES: Rect = Rect::new(96, 35, 75, 9);
const CHANNEL_MODES_COUNT: usize = 5;
const CHANNEL_MODE_LABELS: [&str; CHANNEL_MODES_COUNT] = ["4", "5", "6", "7", "8"];
const STOP: Rect = Rect::new(173, 26, 48, 18);
const PLAY_SONG: Rect = Rect::new(221, 26, 113, 9);
const CONTINUE_SONG: Rect = Rect::new(334, 26, 114, 9);
const PLAY_BLOCK: Rect = Rect::new(221, 35, 95, 9);
const PLAY_ON_NOTE: Rect = Rect::new(316, 35, 18, 9);
const CONTINUE_BLOCK: Rect = Rect::new(334, 35, 114, 9);

pub struct Play {
    sequencer: Sequencer,
    playback: Playback,
}
impl Play {
    pub fn new() -> Self {
        return Self {
            sequencer: Sequencer::new(),
            playback: Playback::new(),
        };
    }
}
impl Gadget<AppState, Event> for Play {
    fn update(&mut self, state: &mut AppState) {
        self.sequencer.update(state);
        self.playback.update(state);
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self.sequencer.handle_event(state, event)
            || self.playback.handle_event(state, event);
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        self.sequencer
            .render(state, SEQUENCE_RECT.within(area), painter);
        self.playback.render(state, area, painter);
    }
}

struct Sequencer {
    jump_buttons: [Button; SEQUENCER_JUMP_BUTTONS],
    inc_dec_buttons: [Button; INC_DEC_BUTTONS],
    edit_buttons: [Button; EDIT_BUTTONS],
}
impl Sequencer {
    fn new() -> Self {
        let mut jump_buttons = array::from_fn(|_| Button::new(""));
        jump_buttons[0] =
            Button::new(Triangle::new(Up, JUMP_ARROW_WIDTH, JUMP_ARROW_HEIGHT).with_bar())
                .on_click(|state, click| {
                    let MouseButton::Left = click.button else {
                        return;
                    };
                    state.set_sequence(0);
                });
        jump_buttons[1] = Button::new(Triangle::new(Up, JUMP_ARROW_WIDTH, JUMP_ARROW_HEIGHT))
            .on_click(|state, click| {
                let MouseButton::Left = click.button else {
                    return;
                };
                state.prev_sequence();
            });
        jump_buttons[2] = Button::new(Triangle::new(Down, JUMP_ARROW_WIDTH, JUMP_ARROW_HEIGHT))
            .on_click(|state, click| {
                let MouseButton::Left = click.button else {
                    return;
                };
                state.next_sequence();
            });

        jump_buttons[3] =
            Button::new(Triangle::new(Down, JUMP_ARROW_WIDTH, JUMP_ARROW_HEIGHT).with_bar())
                .on_click(|state, click| {
                    let MouseButton::Left = click.button else {
                        return;
                    };
                    state.set_sequence(usize::MAX);
                });

        let mut inc_dec_buttons = array::from_fn(|_| Button::new(""));
        let [inc, dec] = &mut inc_dec_buttons;
        inc.set_content(IncDec::new(
            true,
            INC_DEC_LINE_WIDTH,
            INC_DEC_LINE_HEIGHT,
            1,
        ));
        dec.set_content(IncDec::new(
            false,
            INC_DEC_LINE_WIDTH,
            INC_DEC_LINE_HEIGHT,
            1,
        ));
        inc.set_on_click(|state, click| {
            let MouseButton::Left = click.button else {
                return;
            };
            state.increment_sequence_pattern();
        });
        dec.set_on_click(|state, click| {
            let MouseButton::Left = click.button else {
                return;
            };
            state.decrement_sequence_pattern();
        });

        let mut edit_buttons = array::from_fn(|_| Button::new("").font(fonts::LABEL));
        let [ins, del, ins_current] = &mut edit_buttons;
        // OctaMED has this label a pixel right of centre
        *ins = Button::new("INS")
            .font(fonts::LABEL)
            .align(Align::Left)
            .padding_xy(INS_LABEL_OFFSET, 1);
        del.set_text("DEL");
        ins_current.set_content(ArrowLabel::new("INS"));
        ins.set_on_click(|state, click| {
            let MouseButton::Left = click.button else {
                return;
            };
            state.insert_sequence(PatternId::ZERO);
        });
        del.set_on_click(|state, click| {
            let MouseButton::Left = click.button else {
                return;
            };
            state.delete_sequence(state.active_sequence_index);
        });
        ins_current.set_on_click(|state, click| {
            let MouseButton::Left = click.button else {
                return;
            };
            state.insert_sequence(state.active_pattern);
        });

        return Self {
            jump_buttons,
            inc_dec_buttons,
            edit_buttons,
        };
    }
}
impl Gadget<AppState, Event> for Sequencer {
    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self
            .jump_buttons
            .iter_mut()
            .chain(&mut self.inc_dec_buttons)
            .chain(&mut self.edit_buttons)
            .any(|button| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        let list_rect = SEQUENCE_NUMBERS_RECT.within(area);
        let sequence = state.song().sequence();
        let active = state.active_sequence_index;
        let previous = active.min(ACTIVE_ROW);
        let next = sequence
            .len()
            .saturating_sub(active + 1)
            .min(SEQUENCE_COUNT_MAX - 1 - ACTIVE_ROW);
        let inside = painter.frame(list_rect, 1);
        let (_, list) = inside.split_top(SEQUENCE_TOP_MARGIN);
        let rows = list.rows(&[CHAR_HEIGHT; SEQUENCE_COUNT_MAX]);
        painter.fill(rows[ACTIVE_ROW], Pen::SHADOW);
        // a loaded song can have an empty sequence
        let shown = sequence
            .get(active - previous..=active + next)
            .unwrap_or_default();
        for (row, entry) in rows[ACTIVE_ROW - previous..].iter().zip(shown) {
            painter.line_in(
                *row,
                &Line::from(format!("{:02}", entry.0).as_str()),
                Pen::SHINE,
                Right,
            );
        }

        let rows = JUMP_BUTTONS_RECT.within(area).rows(&[
            OUTER_JUMP_BUTTON_HEIGHT,
            INNER_JUMP_BUTTON_HEIGHT,
            JUMP_SEPARATOR_HEIGHT,
            INNER_JUMP_BUTTON_HEIGHT,
            OUTER_JUMP_BUTTON_HEIGHT,
        ]);
        painter.fill(rows[SEQUENCER_JUMP_SEPARATOR_ROW], Pen::BACKGROUND);
        for (mut i, button) in self.jump_buttons.iter_mut().enumerate() {
            if i >= SEQUENCER_JUMP_SEPARATOR_ROW {
                i += 1;
            }
            button.render(state, rows[i], painter);
        }

        let rows = BUTTONS_RECT.within(area).rows(&[
            INC_DEC_BUTTON_HEIGHT,
            EDIT_BUTTON_HEIGHT,
            EDIT_BUTTON_HEIGHT,
            EDIT_BUTTON_HEIGHT,
        ]);
        let inc_dec = rows[0].split_columns(INC_DEC_BUTTONS as i32);
        for (button, column) in self.inc_dec_buttons.iter_mut().zip(inc_dec) {
            button.render(state, column, painter);
        }
        for (button, row) in self.edit_buttons.iter_mut().zip(&rows[1..]) {
            button.render(state, *row, painter);
        }
    }
}

impl PlayStart {
    const ALL: [Self; 4] = [
        Self::Song,
        Self::ContinueSong,
        Self::Block,
        Self::ContinueBlock,
    ];

    fn label(self) -> &'static str {
        return match self {
            Self::Song => "PLAY SONG",
            Self::ContinueSong => "CONT. SONG",
            Self::Block => "PLAY BLOCK",
            Self::ContinueBlock => "CONT. BLOCK",
        };
    }

    fn rect(self) -> Rect {
        return match self {
            Self::Song => PLAY_SONG,
            Self::ContinueSong => CONTINUE_SONG,
            Self::Block => PLAY_BLOCK,
            Self::ContinueBlock => CONTINUE_BLOCK,
        };
    }

    // OctaMED doesn't centre these labels
    fn label_offset(self) -> i32 {
        return match self {
            Self::Song => 20,
            Self::ContinueSong | Self::ContinueBlock => 18,
            Self::Block => 14,
        };
    }
}

// the secondary tempo is lines per beat in BPM mode, else ticks per line
fn tempo_values(tempo: &OctamedicTempo) -> [u16; 2] {
    let secondary = if tempo.is_bpm_mode {
        tempo.lines_per_beat
    } else {
        tempo.ticks_per_line
    };
    return [tempo.primary_tempo, secondary as u16];
}

fn tempo_maxima(tempo: &OctamedicTempo) -> [u16; 2] {
    let secondary = if tempo.is_bpm_mode {
        MAX_LINES_PER_BEAT
    } else {
        MAX_TICKS_PER_LINE
    };
    return [MAX_BPM as u16, secondary as u16];
}

fn set_tempo_value(state: &mut AppState, index: usize, value: u16) {
    let mut tempo = state.song().tempo();
    let value = value.clamp(1, tempo_maxima(&tempo)[index]);
    match index {
        0 => tempo.primary_tempo = value,
        _ if tempo.is_bpm_mode => tempo.lines_per_beat = value as u8,
        _ => tempo.ticks_per_line = value as u8,
    }
    state.set_tempo(tempo);
}

// OctaMED doesn't centre this, and spaces the slash wider than the status row does
const TEMPO_INDENT: i32 = 3;
const TEMPO_SLASH_GAPS: [i32; 2] = [1, 2];

// the two tempo numbers with the narrow slash between them
fn tempo_readout([primary, secondary]: [u16; 2]) -> impl Fn(ContentArea, Pen, &mut Painter) {
    let parts = [
        format!("{primary:03}"),
        POSITION_SLASH.to_string(),
        format!("{secondary:02}"),
    ];
    return move |area, pen, painter| {
        let gaps = [0, TEMPO_SLASH_GAPS[0], TEMPO_SLASH_GAPS[1]];
        let mut x = area.inside.x + TEMPO_INDENT;
        let y = text_y(area.padded, fonts::INTERFACE.height());
        for (part, gap) in parts.iter().zip(gaps) {
            x += gap;
            painter.text_in_font(x, y, part, pen, fonts::INTERFACE);
            x += fonts::INTERFACE.width(part);
        }
    };
}

fn left_clicked(button: &mut Button) -> bool {
    return button
        .clicked()
        .is_some_and(|click| click.button == MouseButton::Left);
}

fn label_button(label: &'static str) -> Button {
    return Button::new(label).font(fonts::LABEL);
}

struct Playback {
    // the tempo, then the lines per beat or ticks per line
    sliders: [SteppedSlider; 2],
    tempo_mode: Button,
    tempo: Button,
    hex: Button,
    dec: Button,
    filter: Button,
    meters: [Button; 3],
    sts: Button,
    hq: Button,
    channel_modes: [Button; CHANNEL_MODES_COUNT],
    stop: Button,
    starts: [Button; 4],
    play_on_note: Button,
}

impl Playback {
    fn new() -> Self {
        let mut channel_modes: [Button; CHANNEL_MODES_COUNT] =
            array::from_fn(|mode| label_button(CHANNEL_MODE_LABELS[mode]));
        channel_modes[0].set_toggled(true);
        for mode in &mut channel_modes {
            mode.set_enabled(false);
        }
        return Self {
            sliders: [MAX_BPM as u16, MAX_LINES_PER_BEAT as u16].map(|max| {
                SteppedSlider::new(SliderDirection::Horizontal, 1, max, STEP_ARROW, STEP_WIDTH)
                    .fixed_handle(TEMPO_HANDLE_LENGTH)
            }),
            tempo_mode: Button::new("BPM"),
            tempo: Button::new(tempo_readout([1; 2])),
            hex: label_button("HEX"),
            dec: label_button("DEC"),
            filter: label_button("FILTER"),
            meters: ["E1", "E2", "E3"].map(label_button),
            sts: Button::new(placed(fonts::LABEL, &[(5, "S"), (12, "TS")])).enabled(false),
            hq: label_button("HQ").enabled(false),
            channel_modes,
            stop: label_button("STOP").bold(),
            starts: PlayStart::ALL.map(|start| {
                label_button(start.label())
                    .align(Align::Left)
                    .padding_xy(start.label_offset(), 1)
            }),
            play_on_note: label_button("D"),
        };
    }

    fn buttons(&mut self) -> impl Iterator<Item = &mut Button> {
        return [
            &mut self.tempo_mode,
            &mut self.hex,
            &mut self.dec,
            &mut self.filter,
            &mut self.sts,
            &mut self.hq,
            &mut self.stop,
            &mut self.play_on_note,
        ]
        .into_iter()
        .chain(&mut self.meters)
        .chain(&mut self.channel_modes)
        .chain(&mut self.starts);
    }

    fn update_tempo(&mut self, state: &mut AppState) {
        let tempo = state.song().tempo();
        let maxima = tempo_maxima(&tempo);
        for (index, slider) in self.sliders.iter_mut().enumerate() {
            slider.set_range(1, maxima[index]);
            if let Some(value) = slider.update(state, tempo_values(&tempo)[index]) {
                set_tempo_value(state, index, value);
            }
        }
        if left_clicked(&mut self.tempo_mode) {
            let mut tempo = state.song().tempo();
            tempo.is_bpm_mode = !tempo.is_bpm_mode;
            state.set_tempo(tempo);
        }
        let tempo = state.song().tempo();
        let values = tempo_values(&tempo);
        self.tempo_mode
            .set_text(if tempo.is_bpm_mode { "BPM" } else { "SPD" });
        self.tempo.set_content(tempo_readout(values));
    }

    fn update_meters(&mut self, state: &mut AppState) {
        let meters = &mut state.meters;
        let switches = [&mut meters.bars, &mut meters.vu, &mut meters.scopes];
        for (button, on) in self.meters.iter_mut().zip(switches) {
            if left_clicked(button) {
                *on = !*on;
            }
            button.set_toggled(*on);
        }
    }

    fn update_volume_base(&mut self, state: &mut AppState) {
        // Some(true) for HEX, Some(false) for DEC
        let clicked_hex = if left_clicked(&mut self.hex) {
            Some(true)
        } else if left_clicked(&mut self.dec) {
            Some(false)
        } else {
            None
        };
        // with Shift, the song's volume commands are rewritten to match
        match clicked_hex {
            Some(hex) if state.mouse_mods.contains(KeyMods::SHIFT) => {
                state.convert_volumes_hex(hex)
            }
            Some(hex) => state.set_volumes_hex(hex),
            None => {}
        }
        let hex = state.song().flags.volumes_hex;
        self.hex.set_toggled(hex);
        self.dec.set_toggled(!hex);
    }

    fn update_filter(&mut self, state: &mut AppState) {
        if left_clicked(&mut self.filter) {
            state.set_filter(!state.song().flags.filter_on);
        }
        self.filter.set_toggled(state.song().flags.filter_on);
    }

    fn update_playing(&mut self, state: &mut AppState) {
        if left_clicked(&mut self.stop) {
            state.stop();
        }
        for (start, button) in PlayStart::ALL.into_iter().zip(&mut self.starts) {
            if left_clicked(button) {
                state.start(start);
            }
        }
        if left_clicked(&mut self.play_on_note) {
            state.play_on_note = !state.play_on_note;
        }
        self.play_on_note.set_toggled(state.play_on_note);
        let playing = state.is_playing();
        for (start, button) in PlayStart::ALL.into_iter().zip(&mut self.starts) {
            button.set_toggled(playing && state.started() == Some(start));
        }
    }
}

impl Gadget<AppState, Event> for Playback {
    fn update(&mut self, state: &mut AppState) {
        self.update_tempo(state);
        self.update_volume_base(state);
        self.update_filter(state);
        self.update_meters(state);
        self.update_playing(state);
    }

    fn handle_event(&mut self, state: &mut AppState, event: &Event) -> bool {
        return self
            .sliders
            .iter_mut()
            .any(|slider| slider.handle_event(state, event))
            || self
                .buttons()
                .any(|button| button.handle_event(state, event));
    }

    fn render(&mut self, state: &AppState, area: Rect, painter: &mut Painter) {
        for (slider, rect) in self.sliders.iter_mut().zip(TEMPO_SLIDERS) {
            slider.render(state, rect.within(area), painter);
        }
        self.tempo_mode
            .render(state, TEMPO_MODE.within(area), painter);
        self.tempo.render(state, TEMPO.within(area), painter);

        let [x, y] = VOL_LABEL;
        let vol = Line::from(Span::new("VOL:").font(fonts::LABEL));
        painter.line(area.x + x, area.y + y, &vol, Pen::BACKGROUND);
        self.hex.render(state, HEX.within(area), painter);
        self.dec.render(state, DEC.within(area), painter);
        self.filter.render(state, FILTER.within(area), painter);
        let meters = METERS.within(area).split_columns(self.meters.len() as i32);
        for (button, rect) in self.meters.iter_mut().zip(meters) {
            button.render(state, rect, painter);
        }
        self.sts.render(state, STS.within(area), painter);

        let [x, y] = CHANNEL_MODE_LABEL;
        draw_words(
            painter,
            fonts::SMALL,
            [area.x + x, area.y + y],
            CHANNEL_MODE_WORDS,
            Pen::BACKGROUND,
        );
        self.hq.render(state, HQ.within(area), painter);
        let modes = CHANNEL_MODES
            .within(area)
            .split_columns(CHANNEL_MODES_COUNT as i32);
        for (button, rect) in self.channel_modes.iter_mut().zip(modes) {
            button.render(state, rect, painter);
        }

        self.stop.render(state, STOP.within(area), painter);
        for (start, button) in PlayStart::ALL.into_iter().zip(&mut self.starts) {
            button.render(state, start.rect().within(area), painter);
        }
        self.play_on_note
            .render(state, PLAY_ON_NOTE.within(area), painter);
    }
}

#[cfg(test)]
mod tests {
    use octamedic_core::{
        data::{
            command::{CommandId, SET_VOLUME},
            project::OctamedicProject,
        },
        engine::handle::EngineHandle,
    };
    use rustuition::{rect::Point, target::Framebuffer};

    use super::*;

    #[test]
    fn hex_dec_follow_song() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut playback = Playback::new();
        let mut screen = Framebuffer::new(460, 50);
        playback.update(&mut state);
        playback.render(
            &state,
            screen.bounds(),
            &mut Painter::new(&mut screen, &state.palette),
        );
        assert!(playback.dec.is_toggled() && !playback.hex.is_toggled());
        let press = Event::MouseDown {
            pos: Point::new(HEX.x + 2, HEX.y + 2),
            button: MouseButton::Left,
        };
        playback.handle_event(&mut state, &press);
        playback.update(&mut state);
        assert!(state.song().flags.volumes_hex);
        assert!(!state.is_saved());
        assert!(playback.hex.is_toggled() && !playback.dec.is_toggled());
    }

    #[test]
    fn shift_click_converts_set_volumes() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut playback = Playback::new();
        let cell = &mut state.active_pattern_mut().unwrap().lines[0].tracks[0];
        cell.command_id = CommandId(SET_VOLUME);
        cell.command_value = 0x40;
        let set_volume = |state: &AppState| state.active_pattern().unwrap().lines[0].tracks[0];
        let click = |playback: &mut Playback, state: &mut AppState, area: Rect, shift: bool| {
            // the buttons find out where they are when they're drawn
            let mut screen = Framebuffer::new(460, 50);
            let bounds = screen.bounds();
            playback.render(
                state,
                bounds,
                &mut Painter::new(&mut screen, &state.palette),
            );
            state.mouse_mods = if shift { KeyMods::SHIFT } else { KeyMods::NONE };
            let pos = Point::new(area.x + 2, area.y + 2);
            let button = MouseButton::Left;
            playback.handle_event(state, &Event::MouseDown { pos, button });
            playback.handle_event(state, &Event::MouseUp { pos, button });
            playback.update(state);
        };
        // a plain click leaves it, so 40 now plays as hex 40
        click(&mut playback, &mut state, HEX, false);
        assert!(state.song().flags.volumes_hex);
        assert_eq!(set_volume(&state).command_value, 0x40);
        click(&mut playback, &mut state, DEC, false);
        // decimal 40 is hex 28
        click(&mut playback, &mut state, HEX, true);
        assert!(state.song().flags.volumes_hex);
        assert_eq!(set_volume(&state).command_value, 0x28);
        click(&mut playback, &mut state, DEC, true);
        assert_eq!(set_volume(&state).command_value, 0x40);
    }

    #[test]
    fn filter_follows_song() {
        let mut state = AppState::new(OctamedicProject::new(), EngineHandle::disconnected());
        let mut playback = Playback::new();
        let mut screen = Framebuffer::new(460, 50);
        playback.update(&mut state);
        playback.render(
            &state,
            screen.bounds(),
            &mut Painter::new(&mut screen, &state.palette),
        );
        assert!(!playback.filter.is_toggled());
        let press = Event::MouseDown {
            pos: Point::new(FILTER.x + 2, FILTER.y + 2),
            button: MouseButton::Left,
        };
        playback.handle_event(&mut state, &press);
        playback.update(&mut state);
        assert!(state.song().flags.filter_on);
        assert!(!state.is_saved());
        assert!(playback.filter.is_toggled());
        playback.handle_event(&mut state, &press);
        playback.update(&mut state);
        assert!(!state.song().flags.filter_on);
        assert!(!playback.filter.is_toggled());
    }
}
